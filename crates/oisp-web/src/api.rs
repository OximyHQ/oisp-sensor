//! REST API handlers

use crate::AppState;
use axum::{extract::State, Json};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Serialize)]
pub struct EventsResponse {
    pub events: Vec<serde_json::Value>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct TracesResponse {
    pub traces: Vec<TraceInfo>,
    pub active: usize,
    pub completed: usize,
}

#[derive(Serialize)]
pub struct TraceInfo {
    pub trace_id: String,
    pub process_name: Option<String>,
    pub started_at: String,
    pub duration_ms: i64,
    pub total_tokens: u64,
    pub llm_calls: u32,
    pub tool_calls: u32,
    pub is_complete: bool,
}

#[derive(Serialize)]
pub struct InventoryResponse {
    pub providers: Vec<ProviderInfo>,
    pub apps: Vec<AppInfo>,
}

#[derive(Serialize)]
pub struct ProviderInfo {
    pub name: String,
    pub request_count: u64,
    pub models: Vec<String>,
}

#[derive(Serialize)]
pub struct AppInfo {
    pub name: String,
    pub exe: String,
    pub request_count: u64,
    pub providers: Vec<String>,
    pub account_type: String,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub total_events: u64,
    pub ai_events: u64,
    pub active_traces: usize,
    pub uptime_seconds: u64,
}

pub async fn get_events(State(state): State<Arc<AppState>>) -> Json<EventsResponse> {
    let events = state.events.read().await;
    let event_values: Vec<serde_json::Value> = events
        .iter()
        .take(100)
        .filter_map(|e| serde_json::to_value(e.as_ref()).ok())
        .collect();

    Json(EventsResponse {
        total: events.len(),
        events: event_values,
    })
}

pub async fn get_traces(State(state): State<Arc<AppState>>) -> Json<TracesResponse> {
    let builder = state.trace_builder.read().await;
    let active = builder.active_traces();
    let completed = builder.completed_traces();

    let traces: Vec<TraceInfo> = active
        .values()
        .chain(completed.iter())
        .map(|t| TraceInfo {
            trace_id: t.trace_id.clone(),
            process_name: t.process_name.clone(),
            started_at: t.started_at.to_rfc3339(),
            duration_ms: t.duration().num_milliseconds(),
            total_tokens: t.total_tokens,
            llm_calls: t.llm_call_count,
            tool_calls: t.tool_call_count,
            is_complete: t.is_complete,
        })
        .collect();

    Json(TracesResponse {
        active: active.len(),
        completed: completed.len(),
        traces,
    })
}

pub async fn get_inventory(State(state): State<Arc<AppState>>) -> Json<InventoryResponse> {
    // Build inventory from events
    let events = state.events.read().await;

    let mut providers: HashMap<String, ProviderInfo> = HashMap::new();
    let mut apps: HashMap<String, AppInfo> = HashMap::new();

    for event in events.iter() {
        if let oisp_core::events::OispEvent::AiRequest(e) = event.as_ref() {
            if let Some(provider) = &e.data.provider {
                let entry =
                    providers
                        .entry(provider.name.clone())
                        .or_insert_with(|| ProviderInfo {
                            name: provider.name.clone(),
                            request_count: 0,
                            models: Vec::new(),
                        });
                entry.request_count += 1;

                if let Some(model) = &e.data.model {
                    if !entry.models.contains(&model.id) {
                        entry.models.push(model.id.clone());
                    }
                }
            }

            if let Some(proc) = &e.envelope.process {
                let name = proc.name.clone().unwrap_or_else(|| "unknown".to_string());
                let entry = apps.entry(name.clone()).or_insert_with(|| AppInfo {
                    name,
                    exe: proc.exe.clone().unwrap_or_default(),
                    request_count: 0,
                    providers: Vec::new(),
                    account_type: "unknown".to_string(),
                });
                entry.request_count += 1;

                if let Some(provider) = &e.data.provider {
                    if !entry.providers.contains(&provider.name) {
                        entry.providers.push(provider.name.clone());
                    }
                }
            }
        }
    }

    Json(InventoryResponse {
        providers: providers.into_values().collect(),
        apps: apps.into_values().collect(),
    })
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> Json<StatsResponse> {
    let events = state.events.read().await;
    let builder = state.trace_builder.read().await;

    let ai_events = events.iter().filter(|e| e.is_ai_event()).count() as u64;

    Json(StatsResponse {
        total_events: events.len() as u64,
        ai_events,
        active_traces: builder.active_traces().len(),
        uptime_seconds: 0, // TODO: Track uptime
    })
}
