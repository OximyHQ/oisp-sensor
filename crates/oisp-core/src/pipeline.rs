//! Event pipeline - orchestrates the flow from capture to export

use crate::events::{OispEvent, EventEnvelope};
use crate::plugins::{
    ActionPlugin, CapturePlugin, DecodePlugin, EnrichPlugin, EventAction, ExportPlugin,
    PluginError, PluginResult, RawCaptureEvent,
};
use crate::trace::TraceBuilder;
use std::cmp::Reverse;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Channel buffer size for raw events
    pub raw_buffer_size: usize,

    /// Channel buffer size for processed events
    pub event_buffer_size: usize,

    /// Enable trace building
    pub build_traces: bool,

    /// Maximum events to buffer before dropping
    pub max_buffer: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            raw_buffer_size: 10000,
            event_buffer_size: 5000,
            build_traces: true,
            max_buffer: 100000,
        }
    }
}

/// The main event pipeline
pub struct Pipeline {
    config: PipelineConfig,

    /// Capture plugins
    capture_plugins: Vec<Arc<RwLock<Box<dyn CapturePlugin>>>>,

    /// Decode plugins (sorted by priority)
    decode_plugins: Vec<Arc<Box<dyn DecodePlugin>>>,

    /// Enrich plugins
    enrich_plugins: Vec<Arc<Box<dyn EnrichPlugin>>>,

    /// Action plugins
    action_plugins: Vec<Arc<Box<dyn ActionPlugin>>>,

    /// Export plugins
    export_plugins: Vec<Arc<Box<dyn ExportPlugin>>>,

    /// Trace builder
    trace_builder: Option<Arc<RwLock<TraceBuilder>>>,

    /// Broadcast channel for events (for UI, etc.)
    event_broadcast: broadcast::Sender<Arc<OispEvent>>,

    /// Running state
    running: Arc<RwLock<bool>>,

    /// Shutdown signal
    shutdown_tx: Option<broadcast::Sender<()>>,
}

impl Pipeline {
    /// Create a new pipeline with configuration
    pub fn new(config: PipelineConfig) -> Self {
        let (event_broadcast, _) = broadcast::channel(config.event_buffer_size);

        Self {
            config,
            capture_plugins: Vec::new(),
            decode_plugins: Vec::new(),
            enrich_plugins: Vec::new(),
            action_plugins: Vec::new(),
            export_plugins: Vec::new(),
            trace_builder: None,
            event_broadcast,
            running: Arc::new(RwLock::new(false)),
            shutdown_tx: None,
        }
    }

    /// Add a capture plugin
    pub fn add_capture(&mut self, plugin: Box<dyn CapturePlugin>) {
        self.capture_plugins.push(Arc::new(RwLock::new(plugin)));
    }

    /// Add a decode plugin
    pub fn add_decode(&mut self, plugin: Box<dyn DecodePlugin>) {
        self.decode_plugins.push(Arc::new(plugin));
        // Sort by priority (higher first)
        self.decode_plugins.sort_by_key(|p| Reverse(p.priority()));
    }

    /// Add an enrich plugin
    pub fn add_enrich(&mut self, plugin: Box<dyn EnrichPlugin>) {
        self.enrich_plugins.push(Arc::new(plugin));
    }

    /// Add an action plugin
    pub fn add_action(&mut self, plugin: Box<dyn ActionPlugin>) {
        self.action_plugins.push(Arc::new(plugin));
    }

    /// Add an export plugin
    pub fn add_export(&mut self, plugin: Box<dyn ExportPlugin>) {
        self.export_plugins.push(Arc::new(plugin));
    }

    /// Enable trace building
    pub fn enable_traces(&mut self) {
        self.trace_builder = Some(Arc::new(RwLock::new(TraceBuilder::new())));
    }

    /// Subscribe to event broadcast
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<OispEvent>> {
        self.event_broadcast.subscribe()
    }

    /// Get the event broadcast sender (for sharing with web server, etc.)
    pub fn event_sender(&self) -> broadcast::Sender<Arc<OispEvent>> {
        self.event_broadcast.clone()
    }

    /// Get the trace builder (if enabled)
    pub fn trace_builder(&self) -> Option<Arc<RwLock<TraceBuilder>>> {
        self.trace_builder.clone()
    }

    /// Start the pipeline
    pub async fn start(&mut self) -> PluginResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(PluginError::OperationFailed(
                "Pipeline already running".into(),
            ));
        }
        *running = true;
        drop(running);

        let (shutdown_tx, _) = broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx.clone());

        // Channel for raw events from capture plugins
        let (raw_tx, mut raw_rx) = mpsc::channel::<RawCaptureEvent>(self.config.raw_buffer_size);

        // Start all capture plugins
        for capture in &self.capture_plugins {
            let tx = raw_tx.clone();
            let mut capture = capture.write().await;
            if let Err(e) = capture.start(tx).await {
                error!("Failed to start capture plugin {}: {}", capture.name(), e);
            } else {
                info!("Started capture plugin: {}", capture.name());
            }
        }

        // Drop the original sender so the channel closes when all captures stop
        drop(raw_tx);

        // Clone references for the processing task
        let decode_plugins = self.decode_plugins.clone();
        let enrich_plugins = self.enrich_plugins.clone();
        let action_plugins = self.action_plugins.clone();
        let export_plugins = self.export_plugins.clone();
        let trace_builder = self.trace_builder.clone();
        let event_broadcast = self.event_broadcast.clone();
        let running = self.running.clone();
        let mut shutdown_rx = shutdown_tx.subscribe();

        // Main processing loop
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(raw_event) = raw_rx.recv() => {
                        // Debug log for raw event reception
                        info!("Received raw event: id={}, kind={:?}, size={} bytes", 
                            raw_event.id, raw_event.kind, raw_event.data.len());
                        
                        // Process the raw event through the pipeline
                        if let Err(e) = Self::process_raw_event(
                            raw_event,
                            &decode_plugins,
                            &enrich_plugins,
                            &action_plugins,
                            &export_plugins,
                            trace_builder.as_ref(),
                            &event_broadcast,
                        ).await {
                            debug!("Error processing event: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Pipeline shutdown signal received");
                        break;
                    }
                    else => {
                        // All senders dropped, channel closed
                        break;
                    }
                }
            }

            // Flush all export plugins
            for export in &export_plugins {
                if let Err(e) = export.flush().await {
                    warn!("Error flushing export plugin {}: {}", export.name(), e);
                }
            }

            *running.write().await = false;
            info!("Pipeline stopped");
        });

        Ok(())
    }

    /// Stop the pipeline
    pub async fn stop(&mut self) -> PluginResult<()> {
        // Send shutdown signal
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
        }

        // Stop all capture plugins
        for capture in &self.capture_plugins {
            let mut capture = capture.write().await;
            if let Err(e) = capture.stop().await {
                warn!("Error stopping capture plugin {}: {}", capture.name(), e);
            }
        }

        // Wait for running to become false
        loop {
            if !*self.running.read().await {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// Process a single raw event through the pipeline
    async fn process_raw_event(
        raw: RawCaptureEvent,
        decode_plugins: &[Arc<Box<dyn DecodePlugin>>],
        enrich_plugins: &[Arc<Box<dyn EnrichPlugin>>],
        action_plugins: &[Arc<Box<dyn ActionPlugin>>],
        export_plugins: &[Arc<Box<dyn ExportPlugin>>],
        trace_builder: Option<&Arc<RwLock<TraceBuilder>>>,
        event_broadcast: &broadcast::Sender<Arc<OispEvent>>,
    ) -> PluginResult<()> {
        // 0. CREATE RAW CAPTURE EVENT (for debugging/visibility)
        let mut raw_envelope = EventEnvelope::new("capture.raw");
        raw_envelope.ts = chrono::Utc::now();
        raw_envelope.ts_mono = Some(raw.timestamp_ns);
        raw_envelope.process = Some(crate::events::ProcessInfo {
            pid: raw.pid,
            ppid: raw.metadata.ppid,
            exe: raw.metadata.exe.clone(),
            name: raw.metadata.comm.clone(),
            tid: raw.tid,
            ..Default::default()
        });
        
        let raw_oisp_event = OispEvent::CaptureRaw(crate::events::CaptureRawEvent {
            envelope: raw_envelope,
            data: crate::events::CaptureRawData {
                kind: format!("{:?}", raw.kind),
                data: String::from_utf8_lossy(&raw.data).to_string(),
                len: raw.data.len(),
                pid: raw.pid,
                tid: raw.tid,
                comm: raw.metadata.comm.clone(),
            },
        });
        
        let raw_arc = Arc::new(raw_oisp_event);
        
        // Broadcast and export raw event
        let _ = event_broadcast.send(raw_arc.clone());
        for exporter in export_plugins {
            if let Err(e) = exporter.export(&raw_arc).await {
                debug!("Exporter {} failed for raw event: {}", exporter.name(), e);
            }
        }

        // 1. DECODE: Find a decoder and decode the raw event
        let mut events = Vec::new();
        for decoder in decode_plugins {
            if decoder.can_decode(&raw) {
                match decoder.decode(raw.clone()).await {
                    Ok(decoded) => {
                        events = decoded;
                        break;
                    }
                    Err(e) => {
                        debug!("Decoder {} failed: {}", decoder.name(), e);
                    }
                }
            }
        }

        if events.is_empty() {
            return Ok(()); // No decoder handled this event
        }

        // Process each decoded event
        for mut event in events {
            // 2. ENRICH: Add context to the event
            for enricher in enrich_plugins {
                if enricher.applies_to(&event) {
                    if let Err(e) = enricher.enrich(&mut event).await {
                        debug!("Enricher {} failed: {}", enricher.name(), e);
                    }
                }
            }

            // 3. ACTION: Filter/transform/redact
            let mut current_events = vec![event];
            for action in action_plugins {
                let mut next_events = Vec::new();
                for evt in current_events {
                    if action.applies_to(&evt) {
                        match action.process(evt).await {
                            Ok((processed, action_result)) => match action_result {
                                EventAction::Pass => next_events.push(processed),
                                EventAction::Modified => next_events.push(processed),
                                EventAction::Drop => {} // Don't add to next
                                EventAction::Replace(replacements) => {
                                    next_events.extend(replacements);
                                }
                            },
                            Err(e) => {
                                debug!("Action {} failed: {}", action.name(), e);
                            }
                        }
                    } else {
                        next_events.push(evt);
                    }
                }
                current_events = next_events;
            }

            // 4. Process final events
            for final_event in current_events {
                let event_arc = Arc::new(final_event);

                // Add to trace builder if enabled
                if let Some(tb) = trace_builder {
                    let mut builder = tb.write().await;
                    builder.add_event((*event_arc).clone());
                }

                // Broadcast to subscribers
                let _ = event_broadcast.send(event_arc.clone());

                // 5. EXPORT: Send to all exporters
                for exporter in export_plugins {
                    if let Err(e) = exporter.export(&event_arc).await {
                        debug!("Exporter {} failed: {}", exporter.name(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if pipeline is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}
