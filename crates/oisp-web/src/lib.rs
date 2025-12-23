//! Web UI backend for OISP Sensor
//!
//! Serves the React frontend (embedded) and provides REST/WebSocket APIs.

mod api;
pub mod web_event;
mod ws;

pub use web_event::{WebEvent, WebEventType, WebEventsResponse};

use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use oisp_core::events::OispEvent;
use oisp_core::metrics::SharedMetrics;
use oisp_core::trace::TraceBuilder;
use rust_embed::RustEmbed;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, info};

/// Embedded frontend assets (built from frontend/ directory)
#[derive(RustEmbed)]
#[folder = "../../frontend/out"]
#[prefix = ""]
struct FrontendAssets;

/// Web server configuration
#[derive(Debug, Clone)]
pub struct WebConfig {
    pub host: String,
    pub port: u16,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            // Use 0.0.0.0 for Docker compatibility
            host: "0.0.0.0".to_string(),
            port: 7777,
        }
    }
}

/// Maximum events to keep in memory for API access
const MAX_EVENTS: usize = 1000;

/// Shared application state
pub struct AppState {
    pub event_tx: broadcast::Sender<Arc<OispEvent>>,
    pub trace_builder: Arc<RwLock<TraceBuilder>>,
    pub events: Arc<RwLock<Vec<Arc<OispEvent>>>>,
    pub metrics: Option<SharedMetrics>,
}

/// Start the web server
pub async fn start_server(
    config: WebConfig,
    event_tx: broadcast::Sender<Arc<OispEvent>>,
    trace_builder: Arc<RwLock<TraceBuilder>>,
) -> anyhow::Result<()> {
    start_server_with_metrics(config, event_tx, trace_builder, None).await
}

/// Start the web server with optional metrics collector
pub async fn start_server_with_metrics(
    config: WebConfig,
    event_tx: broadcast::Sender<Arc<OispEvent>>,
    trace_builder: Arc<RwLock<TraceBuilder>>,
    metrics: Option<SharedMetrics>,
) -> anyhow::Result<()> {
    let events = Arc::new(RwLock::new(Vec::new()));

    // Spawn a background task to collect events
    let events_clone = events.clone();
    let mut event_rx = event_tx.subscribe();
    tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let mut events = events_clone.write().await;
                    events.insert(0, event);
                    // Keep only MAX_EVENTS
                    if events.len() > MAX_EVENTS {
                        events.pop();
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    debug!("Event collector lagged by {} events", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    debug!("Event broadcast channel closed");
                    break;
                }
            }
        }
    });

    let state = Arc::new(AppState {
        event_tx,
        trace_builder,
        events,
        metrics,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // API routes
        .route("/api/events", get(api::get_events))
        .route("/api/web-events", get(api::get_web_events))
        .route("/api/traces", get(api::get_traces))
        .route("/api/inventory", get(api::get_inventory))
        .route("/api/stats", get(api::get_stats))
        .route("/api/metrics", get(api::get_metrics))
        .route("/api/metrics/processes", get(api::get_process_metrics))
        .route("/metrics", get(api::get_metrics_prometheus))
        .route("/api/health", get(health_check))
        .route("/ws", get(ws::ws_handler))
        // Static file serving (fallback to legacy pages)
        .route("/legacy", get(legacy_index))
        .route("/legacy/timeline", get(legacy_timeline))
        // Frontend routes - serve React app for all paths
        .fallback(serve_frontend)
        .layer(cors)
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    info!("Web UI available at http://{}", addr);
    info!("  - React frontend at /");
    info!("  - Legacy UI at /legacy");
    info!("  - API at /api/*");
    info!("  - WebSocket at /ws");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Serve embedded frontend files
async fn serve_frontend(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try exact path first
    if let Some(content) = FrontendAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(content.data.into_owned()))
            .unwrap();
    }

    // Try path with index.html for directories
    let index_path = if path.is_empty() {
        "index.html".to_string()
    } else {
        format!("{}/index.html", path)
    };

    if let Some(content) = FrontendAssets::get(&index_path) {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(content.data.into_owned()))
            .unwrap();
    }

    // For SPA routing: serve root index.html for any unmatched route
    if let Some(content) = FrontendAssets::get("index.html") {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(content.data.into_owned()))
            .unwrap();
    }

    // Fallback to 404
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/html")
        .body(Body::from(
            "<html><body><h1>404 Not Found</h1></body></html>",
        ))
        .unwrap()
}

/// Legacy index page - redirects to React frontend
async fn legacy_index() -> impl IntoResponse {
    axum::response::Redirect::permanent("/")
}

/// Legacy timeline page - redirects to React frontend
async fn legacy_timeline() -> impl IntoResponse {
    axum::response::Redirect::permanent("/")
}

/// Health check endpoint for Docker/Kubernetes probes
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "oisp-sensor",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
