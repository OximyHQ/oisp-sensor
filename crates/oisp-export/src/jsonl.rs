//! JSONL file exporter

use async_trait::async_trait;
use oisp_core::events::OispEvent;
use oisp_core::plugins::{
    ExportPlugin, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
};
use std::any::Any;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::info;

/// JSONL exporter configuration
#[derive(Debug, Clone)]
pub struct JsonlExporterConfig {
    /// Output file path
    pub path: PathBuf,

    /// Whether to append to existing file
    pub append: bool,

    /// Pretty print JSON (not recommended for large files)
    pub pretty: bool,

    /// Flush after each write
    pub flush_each: bool,
}

impl Default for JsonlExporterConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("/tmp/oisp-events.jsonl"),
            append: true,
            pretty: false,
            flush_each: true,
        }
    }
}

/// JSONL file exporter
pub struct JsonlExporter {
    config: JsonlExporterConfig,
    writer: Option<Mutex<BufWriter<File>>>,
    events_written: std::sync::atomic::AtomicU64,
}

impl JsonlExporter {
    pub fn new(config: JsonlExporterConfig) -> Self {
        Self {
            config,
            writer: None,
            events_written: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn ensure_writer(&mut self) -> PluginResult<()> {
        if self.writer.is_none() {
            let file = if self.config.append {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.config.path)?
            } else {
                File::create(&self.config.path)?
            };

            self.writer = Some(Mutex::new(BufWriter::new(file)));
            info!("JSONL exporter writing to: {:?}", self.config.path);
        }
        Ok(())
    }
}

impl PluginInfo for JsonlExporter {
    fn name(&self) -> &str {
        "jsonl-exporter"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Exports events to JSONL files"
    }
}

impl Plugin for JsonlExporter {
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()> {
        if let Some(path) = config.get::<String>("path") {
            self.config.path = PathBuf::from(path);
        }
        if let Some(append) = config.get::<bool>("append") {
            self.config.append = append;
        }
        if let Some(pretty) = config.get::<bool>("pretty") {
            self.config.pretty = pretty;
        }

        self.ensure_writer()?;
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        if let Some(writer) = &self.writer {
            if let Ok(mut w) = writer.lock() {
                let _ = w.flush();
            }
        }
        self.writer = None;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl ExportPlugin for JsonlExporter {
    async fn export(&self, event: &OispEvent) -> PluginResult<()> {
        let json = if self.config.pretty {
            serde_json::to_string_pretty(event)?
        } else {
            serde_json::to_string(event)?
        };

        if let Some(writer) = &self.writer {
            let mut w = writer
                .lock()
                .map_err(|e| PluginError::OperationFailed(format!("Lock poisoned: {}", e)))?;

            writeln!(w, "{}", json)?;

            if self.config.flush_each {
                w.flush()?;
            }

            self.events_written
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        Ok(())
    }

    async fn flush(&self) -> PluginResult<()> {
        if let Some(writer) = &self.writer {
            let mut w = writer
                .lock()
                .map_err(|e| PluginError::OperationFailed(format!("Lock poisoned: {}", e)))?;
            w.flush()?;
        }
        Ok(())
    }
}
