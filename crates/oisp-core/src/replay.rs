//! Event replay from JSONL files
//!
//! This module provides functionality to replay recorded OISP events from JSONL files,
//! enabling development and testing without requiring live capture capabilities.

use crate::events::OispEvent;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// Configuration for event replay
#[derive(Debug, Clone)]
pub struct ReplayConfig {
    /// Path to the JSONL file containing events
    pub input_file: PathBuf,

    /// Speed multiplier for replay timing
    /// - 1.0 = real-time (preserve original timing between events)
    /// - 0.0 = instant (no delays between events)
    /// - 2.0 = 2x speed (half the delay)
    /// - 0.5 = half speed (double the delay)
    pub speed_multiplier: f64,

    /// Whether to loop playback continuously
    pub loop_playback: bool,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            input_file: PathBuf::new(),
            speed_multiplier: 1.0,
            loop_playback: false,
        }
    }
}

/// Event replay engine
///
/// Reads OISP events from a JSONL file and broadcasts them to subscribers.
/// This bypasses the capture/decode pipeline since events are already in OISP format.
pub struct EventReplay {
    config: ReplayConfig,
    running: Arc<AtomicBool>,
}

impl EventReplay {
    /// Create a new event replay instance
    pub fn new(config: ReplayConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if replay is currently running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Stop the replay
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Get a handle to stop the replay from another task
    pub fn stop_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    /// Run the replay, broadcasting events to the provided channel
    ///
    /// This function will:
    /// 1. Read events from the JSONL file
    /// 2. Parse each line as an OispEvent
    /// 3. Calculate timing delays based on event timestamps
    /// 4. Broadcast events with appropriate timing
    ///
    /// Returns the number of events replayed
    pub async fn run(&self, event_tx: broadcast::Sender<Arc<OispEvent>>) -> anyhow::Result<u64> {
        self.running.store(true, Ordering::Relaxed);

        let mut total_events = 0u64;

        loop {
            let events_this_pass = self.replay_file(&event_tx).await?;
            total_events += events_this_pass;

            if !self.config.loop_playback || !self.running.load(Ordering::Relaxed) {
                break;
            }

            info!("Looping replay, restarting from beginning...");
        }

        self.running.store(false, Ordering::Relaxed);
        Ok(total_events)
    }

    /// Replay a single pass through the file
    async fn replay_file(
        &self,
        event_tx: &broadcast::Sender<Arc<OispEvent>>,
    ) -> anyhow::Result<u64> {
        let file = tokio::fs::File::open(&self.config.input_file).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let mut event_count = 0u64;
        let mut last_timestamp: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut line_number = 0u64;

        info!(
            "Starting replay from {:?} (speed: {}x, loop: {})",
            self.config.input_file, self.config.speed_multiplier, self.config.loop_playback
        );

        while let Some(line) = lines.next_line().await? {
            line_number += 1;

            // Check if we should stop
            if !self.running.load(Ordering::Relaxed) {
                info!("Replay stopped at line {}", line_number);
                break;
            }

            // Skip empty lines
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse the event
            let event: OispEvent = match serde_json::from_str(line) {
                Ok(e) => e,
                Err(err) => {
                    warn!("Failed to parse event at line {}: {}", line_number, err);
                    debug!("Line content: {}", line);
                    continue;
                }
            };

            // Calculate and apply delay based on timestamps
            let current_timestamp = event.envelope().ts;
            if let Some(last_ts) = last_timestamp {
                if self.config.speed_multiplier > 0.0 {
                    let delay = current_timestamp
                        .signed_duration_since(last_ts)
                        .num_milliseconds();

                    if delay > 0 {
                        let adjusted_delay = (delay as f64 / self.config.speed_multiplier) as u64;

                        // Cap delay at 10 seconds to avoid very long waits
                        let capped_delay = adjusted_delay.min(10_000);

                        if capped_delay > 0 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(capped_delay))
                                .await;
                        }
                    }
                }
            }
            last_timestamp = Some(current_timestamp);

            // Broadcast the event
            let event_arc = Arc::new(event);
            match event_tx.send(event_arc.clone()) {
                Ok(receivers) => {
                    debug!(
                        "Replayed event {} ({}) to {} receivers",
                        event_arc.envelope().event_id,
                        event_arc.event_type(),
                        receivers
                    );
                }
                Err(err) => {
                    // No receivers, but that's okay - web server might not be connected yet
                    debug!("No receivers for event: {}", err);
                }
            }

            event_count += 1;
        }

        info!(
            "Replay complete: {} events from {:?}",
            event_count, self.config.input_file
        );

        Ok(event_count)
    }
}

/// Read events from a JSONL file without replaying (for validation/testing)
pub async fn read_events_from_file(path: &PathBuf) -> anyhow::Result<Vec<OispEvent>> {
    let file = tokio::fs::File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut events = Vec::new();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        match serde_json::from_str::<OispEvent>(line) {
            Ok(event) => events.push(event),
            Err(err) => {
                error!("Failed to parse event: {}", err);
            }
        }
    }

    Ok(events)
}

/// Count events in a JSONL file without loading them all into memory
pub async fn count_events_in_file(path: &PathBuf) -> anyhow::Result<u64> {
    let file = tokio::fs::File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut count = 0u64;

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') {
            count += 1;
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_event_json(event_id: &str, ts: &str) -> String {
        format!(
            r#"{{"oisp_version":"0.1","event_id":"{}","event_type":"ai.request","ts":"{}","source":{{"collector":"test"}},"confidence":{{"level":"high","completeness":"full"}},"data":{{"request_id":"req-1","request_type":"completion"}}}}"#,
            event_id, ts
        )
    }

    #[tokio::test]
    async fn test_read_events_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "{}",
            create_test_event_json("evt-1", "2024-01-01T12:00:00Z")
        )
        .unwrap();
        writeln!(
            file,
            "{}",
            create_test_event_json("evt-2", "2024-01-01T12:00:01Z")
        )
        .unwrap();
        writeln!(file, "# comment line").unwrap();
        writeln!(file).unwrap();
        writeln!(
            file,
            "{}",
            create_test_event_json("evt-3", "2024-01-01T12:00:02Z")
        )
        .unwrap();

        let events = read_events_from_file(&file.path().to_path_buf())
            .await
            .unwrap();

        assert_eq!(events.len(), 3);
        assert_eq!(events[0].envelope().event_id, "evt-1");
        assert_eq!(events[1].envelope().event_id, "evt-2");
        assert_eq!(events[2].envelope().event_id, "evt-3");
    }

    #[tokio::test]
    async fn test_count_events_in_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "{}",
            create_test_event_json("evt-1", "2024-01-01T12:00:00Z")
        )
        .unwrap();
        writeln!(file, "# comment").unwrap();
        writeln!(
            file,
            "{}",
            create_test_event_json("evt-2", "2024-01-01T12:00:01Z")
        )
        .unwrap();
        writeln!(file).unwrap();

        let count = count_events_in_file(&file.path().to_path_buf())
            .await
            .unwrap();

        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_event_replay_instant() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "{}",
            create_test_event_json("evt-1", "2024-01-01T12:00:00Z")
        )
        .unwrap();
        writeln!(
            file,
            "{}",
            create_test_event_json("evt-2", "2024-01-01T12:00:10Z")
        )
        .unwrap();

        let config = ReplayConfig {
            input_file: file.path().to_path_buf(),
            speed_multiplier: 0.0, // Instant replay
            loop_playback: false,
        };

        let replay = EventReplay::new(config);
        let (tx, mut rx) = broadcast::channel(100);

        // Run replay in background
        let replay_handle = tokio::spawn(async move { replay.run(tx).await });

        // Collect events
        let mut received = Vec::new();
        while let Ok(event) = rx.recv().await {
            received.push(event);
            if received.len() >= 2 {
                break;
            }
        }

        let count = replay_handle.await.unwrap().unwrap();

        assert_eq!(count, 2);
        assert_eq!(received.len(), 2);
        assert_eq!(received[0].envelope().event_id, "evt-1");
        assert_eq!(received[1].envelope().event_id, "evt-2");
    }
}
