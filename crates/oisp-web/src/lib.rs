//! Web UI backend for OISP Sensor

mod api;
mod ws;

use axum::{
    routing::get,
    Router,
    response::Html,
};
use oisp_core::events::OispEvent;
use oisp_core::trace::TraceBuilder;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

/// Web server configuration
#[derive(Debug, Clone)]
pub struct WebConfig {
    pub host: String,
    pub port: u16,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 7777,
        }
    }
}

/// Shared application state
pub struct AppState {
    pub event_rx: broadcast::Sender<Arc<OispEvent>>,
    pub trace_builder: Arc<RwLock<TraceBuilder>>,
    pub events: Arc<RwLock<Vec<Arc<OispEvent>>>>,
}

/// Start the web server
pub async fn start_server(
    config: WebConfig,
    event_tx: broadcast::Sender<Arc<OispEvent>>,
    trace_builder: Arc<RwLock<TraceBuilder>>,
) -> anyhow::Result<()> {
    let state = Arc::new(AppState {
        event_rx: event_tx,
        trace_builder,
        events: Arc::new(RwLock::new(Vec::new())),
    });
    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    let app = Router::new()
        .route("/", get(index))
        .route("/timeline", get(timeline_page))
        .route("/api/events", get(api::get_events))
        .route("/api/traces", get(api::get_traces))
        .route("/api/inventory", get(api::get_inventory))
        .route("/api/stats", get(api::get_stats))
        .route("/ws", get(ws::ws_handler))
        .layer(cors)
        .with_state(state);
    
    let addr = format!("{}:{}", config.host, config.port);
    info!("Web UI available at http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn timeline_page() -> Html<&'static str> {
    Html(include_str!("../static/timeline.html"))
}

