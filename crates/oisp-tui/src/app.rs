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
    pub app_id: Option<String>,
    pub name: String,
    pub vendor: Option<String>,
    pub exe: String,
    pub tier: String,
    pub request_count: u64,
    pub providers: Vec<String>,
    pub account_type: String,
}

/// Web app stats (browser-originated requests)
#[derive(Debug, Clone, Default)]
pub struct WebAppStats {
    pub web_app_id: String,
    pub name: String,
    pub web_app_type: String, // "direct" or "embedded"
    pub request_count: u64,
    pub providers: Vec<String>,
}

/// Process info for tree view
#[derive(Debug, Clone, Default)]
pub struct ProcessNode {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub name: String,
    pub exe: Option<String>,
    pub event_count: u64,
    pub ai_event_count: u64,
    pub children: Vec<u32>,
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

    /// Web app stats (browser-originated requests)
    pub web_apps: HashMap<String, WebAppStats>,

    /// Process tree by PID
    pub processes: HashMap<u32, ProcessNode>,

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
            web_apps: HashMap::new(),
            processes: HashMap::new(),
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

            let is_ai = event.is_ai_event();
            if is_ai {
                self.ai_events += 1;
            }

            // Update timeline
            self.timeline.insert(0, event.clone());
            if self.timeline.len() > self.max_events {
                self.timeline.pop();
            }

            // Update stats
            self.update_stats(&event);

            // Update process tree
            self.update_process_tree(&event, is_ai);

            // Update traces
            self.trace_builder.add_event((*event).clone());
        }
    }

    /// Update process tree from event
    fn update_process_tree(&mut self, event: &OispEvent, is_ai: bool) {
        let envelope = event.envelope();
        if let Some(proc) = &envelope.process {
            let pid = proc.pid;
            let ppid = proc.ppid;

            // Get or create process node
            let node = self.processes.entry(pid).or_insert_with(|| ProcessNode {
                pid,
                ppid,
                name: proc.name.clone().unwrap_or_else(|| "unknown".to_string()),
                exe: proc.exe.clone(),
                event_count: 0,
                ai_event_count: 0,
                children: Vec::new(),
            });

            node.event_count += 1;
            if is_ai {
                node.ai_event_count += 1;
            }

            // Update parent's children list
            if let Some(parent_pid) = ppid {
                if let Some(parent) = self.processes.get_mut(&parent_pid) {
                    if !parent.children.contains(&pid) {
                        parent.children.push(pid);
                    }
                }
            }
        }
    }

    /// Get root processes (no parent or parent not tracked)
    pub fn root_processes(&self) -> Vec<&ProcessNode> {
        self.processes
            .values()
            .filter(|p| {
                p.ppid
                    .map(|ppid| !self.processes.contains_key(&ppid))
                    .unwrap_or(true)
            })
            .collect()
    }

    /// Get children of a process
    pub fn get_children(&self, pid: u32) -> Vec<&ProcessNode> {
        self.processes
            .values()
            .filter(|p| p.ppid == Some(pid))
            .collect()
    }

    fn update_stats(&mut self, event: &OispEvent) {
        if let OispEvent::AiRequest(e) = event {
            // Update provider stats
            if let Some(provider) = &e.data.provider {
                let stats = self
                    .providers
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

            // Update app stats - prefer AppInfo from envelope, fall back to process name
            let (app_key, app_id, app_name, vendor, tier, exe) =
                if let Some(app_info) = &e.envelope.app {
                    let key = app_info
                        .app_id
                        .clone()
                        .or_else(|| app_info.name.clone())
                        .unwrap_or_else(|| {
                            e.envelope
                                .process
                                .as_ref()
                                .and_then(|p| p.name.clone())
                                .unwrap_or_else(|| "unknown".to_string())
                        });
                    let tier_str = match app_info.tier {
                        oisp_core::events::AppTier::Profiled => "profiled",
                        oisp_core::events::AppTier::Identified => "identified",
                        oisp_core::events::AppTier::Unknown => "unknown",
                    };
                    (
                        key,
                        app_info.app_id.clone(),
                        app_info.name.clone().unwrap_or_else(|| {
                            e.envelope
                                .process
                                .as_ref()
                                .and_then(|p| p.name.clone())
                                .unwrap_or_else(|| "unknown".to_string())
                        }),
                        app_info.vendor.clone(),
                        tier_str.to_string(),
                        e.envelope
                            .process
                            .as_ref()
                            .and_then(|p| p.exe.clone())
                            .unwrap_or_default(),
                    )
                } else if let Some(proc) = &e.envelope.process {
                    let name = proc.name.clone().unwrap_or_else(|| "unknown".to_string());
                    (
                        name.clone(),
                        None,
                        name,
                        None,
                        "unknown".to_string(),
                        proc.exe.clone().unwrap_or_default(),
                    )
                } else {
                    (
                        "unknown".to_string(),
                        None,
                        "unknown".to_string(),
                        None,
                        "unknown".to_string(),
                        String::new(),
                    )
                };

            let stats = self
                .apps
                .entry(app_key.clone())
                .or_insert_with(|| AppStats {
                    app_id,
                    name: app_name,
                    vendor,
                    exe,
                    tier,
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

            // Update web app stats if web context is present
            if let Some(web_ctx) = &e.envelope.web_context {
                if let Some(web_app_id) = &web_ctx.web_app_id {
                    let web_stats =
                        self.web_apps
                            .entry(web_app_id.clone())
                            .or_insert_with(|| WebAppStats {
                                web_app_id: web_app_id.clone(),
                                name: web_ctx
                                    .web_app_name
                                    .clone()
                                    .unwrap_or_else(|| web_app_id.clone()),
                                web_app_type: web_ctx
                                    .web_app_type
                                    .map(|t| match t {
                                        oisp_core::events::WebAppType::Direct => "direct",
                                        oisp_core::events::WebAppType::Embedded => "embedded",
                                    })
                                    .unwrap_or("unknown")
                                    .to_string(),
                                ..Default::default()
                            });
                    web_stats.request_count += 1;

                    if let Some(provider) = &e.data.provider {
                        if !web_stats.providers.contains(&provider.name) {
                            web_stats.providers.push(provider.name.clone());
                        }
                    }
                }
            }
        }
    }

    /// Get traces
    pub fn traces(&self) -> Vec<&AgentTrace> {
        self.trace_builder.all_traces()
    }
}
