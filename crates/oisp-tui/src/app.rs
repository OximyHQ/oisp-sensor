//! TUI application state

use oisp_core::events::OispEvent;
use oisp_core::trace::{AgentTrace, TraceBuilder};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Current view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Timeline,
    Inventory,
    ProcessTree,
    Traces,
}

/// Provider stats
#[derive(Debug, Clone, Default)]
pub struct ProviderStats {
    pub name: String,
    pub request_count: u64,
    pub models: Vec<String>,
    pub apps: Vec<String>,
}

/// App using AI stats
#[derive(Debug, Clone, Default)]
pub struct AppStats {
    pub name: String,
    pub exe: String,
    pub request_count: u64,
    pub providers: Vec<String>,
    pub account_type: String,
}

/// TUI application state
pub struct App {
    /// Event receiver
    event_rx: broadcast::Receiver<Arc<OispEvent>>,
    
    /// Current view
    pub view: View,
    
    /// Timeline events (most recent first)
    pub timeline: Vec<Arc<OispEvent>>,
    
    /// Maximum events to keep
    max_events: usize,
    
    /// Scroll position
    pub scroll: usize,
    
    /// Provider stats
    pub providers: HashMap<String, ProviderStats>,
    
    /// App stats
    pub apps: HashMap<String, AppStats>,
    
    /// Trace builder
    trace_builder: TraceBuilder,
    
    /// Total events seen
    pub total_events: u64,
    
    /// AI events seen
    pub ai_events: u64,
}

impl App {
    pub fn new(event_rx: broadcast::Receiver<Arc<OispEvent>>) -> Self {
        Self {
            event_rx,
            view: View::Timeline,
            timeline: Vec::new(),
            max_events: 1000,
            scroll: 0,
            providers: HashMap::new(),
            apps: HashMap::new(),
            trace_builder: TraceBuilder::new(),
            total_events: 0,
            ai_events: 0,
        }
    }
    
    pub fn set_view(&mut self, view: View) {
        self.view = view;
        self.scroll = 0;
    }
    
    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }
    
    pub fn scroll_down(&mut self) {
        self.scroll += 1;
    }
    
    pub fn page_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(20);
    }
    
    pub fn page_down(&mut self) {
        self.scroll += 20;
    }
    
    /// Process incoming events
    pub fn process_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            self.total_events += 1;
            
            if event.is_ai_event() {
                self.ai_events += 1;
            }
            
            // Update timeline
            self.timeline.insert(0, event.clone());
            if self.timeline.len() > self.max_events {
                self.timeline.pop();
            }
            
            // Update stats
            self.update_stats(&event);
            
            // Update traces
            self.trace_builder.add_event((*event).clone());
        }
    }
    
    fn update_stats(&mut self, event: &OispEvent) {
        match event {
            OispEvent::AiRequest(e) => {
                // Update provider stats
                if let Some(provider) = &e.data.provider {
                    let stats = self.providers
                        .entry(provider.name.clone())
                        .or_insert_with(|| ProviderStats {
                            name: provider.name.clone(),
                            ..Default::default()
                        });
                    stats.request_count += 1;
                    
                    if let Some(model) = &e.data.model {
                        if !stats.models.contains(&model.id) {
                            stats.models.push(model.id.clone());
                        }
                    }
                }
                
                // Update app stats
                if let Some(proc) = &e.envelope.process {
                    let name = proc.name.clone().unwrap_or_else(|| "unknown".to_string());
                    let stats = self.apps
                        .entry(name.clone())
                        .or_insert_with(|| AppStats {
                            name: name.clone(),
                            exe: proc.exe.clone().unwrap_or_default(),
                            ..Default::default()
                        });
                    stats.request_count += 1;
                    
                    if let Some(provider) = &e.data.provider {
                        if !stats.providers.contains(&provider.name) {
                            stats.providers.push(provider.name.clone());
                        }
                    }
                    
                    if let Some(auth) = &e.data.auth {
                        stats.account_type = match auth.account_type {
                            Some(oisp_core::events::AccountType::Corporate) => "corporate".to_string(),
                            Some(oisp_core::events::AccountType::Personal) => "personal".to_string(),
                            _ => "unknown".to_string(),
                        };
                    }
                }
            }
            _ => {}
        }
    }
    
    /// Get traces
    pub fn traces(&self) -> Vec<&AgentTrace> {
        self.trace_builder.all_traces()
    }
}

