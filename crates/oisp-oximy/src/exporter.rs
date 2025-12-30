//! Oximy Cloud Exporter
//!
//! Implements the `ExportPlugin` trait to send events to Oximy Cloud.

use crate::client::CloudClient;
use crate::error::OximyResult;
use crate::offline_queue::OfflineQueue;
use async_trait::async_trait;
use oisp_core::events::OispEvent;
use oisp_core::plugins::{
    ExportPlugin, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
};
use std::any::Any;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};

/// Configuration for the Oximy exporter
#[derive(Debug, Clone)]
pub struct OximyExporterConfig {
    /// Batch size before sending
    pub batch_size: usize,

    /// Flush interval
    pub flush_interval: Duration,

    /// Enable offline queue
    pub offline_queue_enabled: bool,

    /// Offline queue path (SQLite database)
    pub offline_queue_path: Option<String>,

    /// Max events in offline queue
    pub offline_queue_max_events: usize,
}

impl Default for OximyExporterConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            flush_interval: Duration::from_secs(5),
            offline_queue_enabled: true,
            offline_queue_path: None,
            offline_queue_max_events: 100_000,
        }
    }
}

/// Oximy Cloud Exporter
///
/// Exports events to Oximy Cloud via HTTP batch API.
/// Supports offline buffering when disconnected.
pub struct OximyExporter {
    client: Arc<CloudClient>,
    config: OximyExporterConfig,
    buffer: Mutex<Vec<OispEvent>>,
    offline_queue: Option<OfflineQueue>,
    last_flush: Mutex<Instant>,

    // Stats
    events_exported: AtomicU64,
    events_failed: AtomicU64,
    events_queued: AtomicU64,
    batches_sent: AtomicU64,
}

impl OximyExporter {
    /// Create a new Oximy exporter
    pub fn new(client: Arc<CloudClient>, config: OximyExporterConfig) -> OximyResult<Self> {
        let offline_queue = if config.offline_queue_enabled {
            let path = config.offline_queue_path.clone().unwrap_or_else(|| {
                dirs::data_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/var/lib/oisp-sensor"))
                    .join("offline_queue.db")
                    .to_string_lossy()
                    .to_string()
            });

            Some(OfflineQueue::new(&path, config.offline_queue_max_events)?)
        } else {
            None
        };

        Ok(Self {
            client,
            config,
            buffer: Mutex::new(Vec::new()),
            offline_queue,
            last_flush: Mutex::new(Instant::now()),
            events_exported: AtomicU64::new(0),
            events_failed: AtomicU64::new(0),
            events_queued: AtomicU64::new(0),
            batches_sent: AtomicU64::new(0),
        })
    }

    /// Create with default config
    pub fn with_client(client: Arc<CloudClient>) -> OximyResult<Self> {
        Self::new(client, OximyExporterConfig::default())
    }

    /// Get export statistics
    pub fn stats(&self) -> ExporterStats {
        ExporterStats {
            events_exported: self.events_exported.load(Ordering::Relaxed),
            events_failed: self.events_failed.load(Ordering::Relaxed),
            events_queued: self.events_queued.load(Ordering::Relaxed),
            batches_sent: self.batches_sent.load(Ordering::Relaxed),
        }
    }

    /// Check if flush is needed based on time
    async fn should_flush_by_time(&self) -> bool {
        let last = self.last_flush.lock().await;
        last.elapsed() >= self.config.flush_interval
    }

    /// Send batch to cloud
    async fn send_batch(&self, events: Vec<OispEvent>) -> OximyResult<()> {
        if events.is_empty() {
            return Ok(());
        }

        let (device_id, token) = self.client.ensure_authenticated().await?;
        let count = events.len();

        debug!("Sending batch of {} events to cloud", count);

        match self
            .client
            .http()
            .send_events(&device_id, &token, &events)
            .await
        {
            Ok(response) => {
                self.events_exported
                    .fetch_add(count as u64, Ordering::Relaxed);
                self.batches_sent.fetch_add(1, Ordering::Relaxed);
                debug!(
                    "Batch sent successfully: {} events, batch_id={}",
                    response.received, response.batch_id
                );
                Ok(())
            }
            Err(e) if e.is_network_error() => {
                warn!("Network error sending batch, queueing for retry: {}", e);
                self.queue_for_retry(events).await?;
                Err(e)
            }
            Err(e) => {
                error!("Failed to send batch: {}", e);
                self.events_failed
                    .fetch_add(count as u64, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    /// Queue events for retry (offline queue)
    async fn queue_for_retry(&self, events: Vec<OispEvent>) -> OximyResult<()> {
        if let Some(queue) = &self.offline_queue {
            let count = events.len();
            queue.enqueue(&events)?;
            self.events_queued
                .fetch_add(count as u64, Ordering::Relaxed);
            debug!("Queued {} events for retry", count);
        } else {
            // No offline queue, events are lost
            let count = events.len();
            self.events_failed
                .fetch_add(count as u64, Ordering::Relaxed);
            warn!("No offline queue, {} events lost", count);
        }
        Ok(())
    }

    /// Try to drain offline queue
    pub async fn drain_offline_queue(&self) -> OximyResult<usize> {
        let queue = match &self.offline_queue {
            Some(q) => q,
            None => return Ok(0),
        };

        let pending = queue.pending_count()?;
        if pending == 0 {
            return Ok(0);
        }

        info!("Draining offline queue: {} events pending", pending);

        let mut total_sent = 0;
        loop {
            let batch = queue.dequeue(self.config.batch_size)?;
            if batch.is_empty() {
                break;
            }

            match self.send_batch(batch.clone()).await {
                Ok(_) => {
                    total_sent += batch.len();
                    self.events_queued
                        .fetch_sub(batch.len() as u64, Ordering::Relaxed);
                }
                Err(e) if e.is_network_error() => {
                    // Re-queue and stop trying
                    queue.enqueue(&batch)?;
                    warn!("Network error while draining queue, will retry later");
                    break;
                }
                Err(e) => {
                    error!("Error draining queue: {}", e);
                    break;
                }
            }
        }

        if total_sent > 0 {
            info!("Drained {} events from offline queue", total_sent);
        }

        Ok(total_sent)
    }
}

impl PluginInfo for OximyExporter {
    fn name(&self) -> &str {
        "oximy-exporter"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Exports events to Oximy Cloud"
    }
}

impl Plugin for OximyExporter {
    fn init(&mut self, _config: &PluginConfig) -> PluginResult<()> {
        info!("Oximy exporter initialized");
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        // Flush remaining events synchronously is tricky in async context
        // The pipeline should call flush() before shutdown
        info!("Oximy exporter shutting down");
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
impl ExportPlugin for OximyExporter {
    async fn export(&self, event: &OispEvent) -> PluginResult<()> {
        // Check if enrolled
        if !self.client.has_valid_credentials().await {
            return Err(PluginError::OperationFailed(
                "Device not enrolled with Oximy Cloud".to_string(),
            ));
        }

        // Add to buffer
        let should_flush = {
            let mut buffer = self.buffer.lock().await;
            buffer.push(event.clone());
            buffer.len() >= self.config.batch_size
        };

        // Flush if batch is full or time elapsed
        if should_flush || self.should_flush_by_time().await {
            self.flush().await?;
        }

        Ok(())
    }

    async fn export_batch(&self, events: &[OispEvent]) -> PluginResult<()> {
        if !self.client.has_valid_credentials().await {
            return Err(PluginError::OperationFailed(
                "Device not enrolled with Oximy Cloud".to_string(),
            ));
        }

        // Add all to buffer
        {
            let mut buffer = self.buffer.lock().await;
            buffer.extend(events.iter().cloned());
        }

        // Flush if we have enough
        let buffer_len = {
            let buffer = self.buffer.lock().await;
            buffer.len()
        };

        if buffer_len >= self.config.batch_size {
            self.flush().await?;
        }

        Ok(())
    }

    async fn flush(&self) -> PluginResult<()> {
        // Take all events from buffer
        let events = {
            let mut buffer = self.buffer.lock().await;
            std::mem::take(&mut *buffer)
        };

        if events.is_empty() {
            return Ok(());
        }

        // Update last flush time
        {
            let mut last = self.last_flush.lock().await;
            *last = Instant::now();
        }

        // Send in batches
        for chunk in events.chunks(self.config.batch_size) {
            if let Err(e) = self.send_batch(chunk.to_vec()).await {
                if !e.is_network_error() {
                    return Err(PluginError::OperationFailed(e.to_string()));
                }
                // Network errors are handled by queueing
            }
        }

        Ok(())
    }
}

/// Exporter statistics
#[derive(Debug, Clone, Default)]
pub struct ExporterStats {
    /// Total events exported successfully
    pub events_exported: u64,

    /// Events that failed to export
    pub events_failed: u64,

    /// Events currently queued (offline)
    pub events_queued: u64,

    /// Total batches sent
    pub batches_sent: u64,
}

// Helper for data directory
mod dirs {
    use std::path::PathBuf;

    pub fn data_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            std::env::var("HOME").ok().map(|h| {
                PathBuf::from(h)
                    .join("Library")
                    .join("Application Support")
                    .join("oisp-sensor")
            })
        }

        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_DATA_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var("HOME").ok().map(|h| {
                        PathBuf::from(h)
                            .join(".local")
                            .join("share")
                            .join("oisp-sensor")
                    })
                })
        }

        #[cfg(target_os = "windows")]
        {
            std::env::var("LOCALAPPDATA")
                .ok()
                .map(|p| PathBuf::from(p).join("oisp-sensor"))
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exporter_config_default() {
        let config = OximyExporterConfig::default();
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.flush_interval, Duration::from_secs(5));
        assert!(config.offline_queue_enabled);
    }

    #[test]
    fn test_exporter_stats_default() {
        let stats = ExporterStats::default();
        assert_eq!(stats.events_exported, 0);
        assert_eq!(stats.events_failed, 0);
        assert_eq!(stats.events_queued, 0);
        assert_eq!(stats.batches_sent, 0);
    }
}
