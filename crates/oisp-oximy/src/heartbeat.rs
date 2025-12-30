//! Heartbeat service for periodic status reporting
//!
//! Sends regular heartbeats to Oximy Cloud to report sensor health
//! and receive commands/policy updates.

use crate::client::CloudClient;
use crate::error::OximyResult;
use crate::types::{HeartbeatResponse, SensorStats, SensorStatus, ServerCommand};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Heartbeat service configuration
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// Interval between heartbeats
    pub interval: Duration,

    /// Timeout for heartbeat requests
    pub timeout: Duration,

    /// Max consecutive failures before alerting
    pub max_failures: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            max_failures: 5,
        }
    }
}

/// Callback for handling server commands
pub type CommandHandler = Arc<dyn Fn(ServerCommand) + Send + Sync>;

/// Heartbeat service
pub struct HeartbeatService {
    client: Arc<CloudClient>,
    config: HeartbeatConfig,
    stats_provider: Arc<dyn StatsProvider>,
    command_handler: Option<CommandHandler>,

    // State
    last_heartbeat: RwLock<Option<Instant>>,
    last_response: RwLock<Option<HeartbeatResponse>>,
    consecutive_failures: AtomicU64,
    total_sent: AtomicU64,
    total_failed: AtomicU64,
}

/// Trait for providing sensor stats
pub trait StatsProvider: Send + Sync {
    /// Get current sensor stats
    fn get_stats(&self) -> SensorStats;

    /// Get current sensor status
    fn get_status(&self) -> SensorStatus;
}

/// Default stats provider (returns static values)
pub struct DefaultStatsProvider {
    start_time: Instant,
}

impl Default for DefaultStatsProvider {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

impl StatsProvider for DefaultStatsProvider {
    fn get_stats(&self) -> SensorStats {
        SensorStats {
            sensor_version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            events_captured: 0,
            events_exported: 0,
            events_queued: 0,
            policy_version: None,
            memory_mb: 0,
            cpu_percent: 0.0,
        }
    }

    fn get_status(&self) -> SensorStatus {
        SensorStatus::Active
    }
}

impl HeartbeatService {
    /// Create new heartbeat service
    pub fn new(client: Arc<CloudClient>, stats_provider: Arc<dyn StatsProvider>) -> Self {
        Self {
            client,
            config: HeartbeatConfig::default(),
            stats_provider,
            command_handler: None,
            last_heartbeat: RwLock::new(None),
            last_response: RwLock::new(None),
            consecutive_failures: AtomicU64::new(0),
            total_sent: AtomicU64::new(0),
            total_failed: AtomicU64::new(0),
        }
    }

    /// Create with configuration
    pub fn with_config(
        client: Arc<CloudClient>,
        stats_provider: Arc<dyn StatsProvider>,
        config: HeartbeatConfig,
    ) -> Self {
        Self {
            client,
            config,
            stats_provider,
            command_handler: None,
            last_heartbeat: RwLock::new(None),
            last_response: RwLock::new(None),
            consecutive_failures: AtomicU64::new(0),
            total_sent: AtomicU64::new(0),
            total_failed: AtomicU64::new(0),
        }
    }

    /// Set command handler
    pub fn set_command_handler(&mut self, handler: CommandHandler) {
        self.command_handler = Some(handler);
    }

    /// Send a single heartbeat
    pub async fn send_heartbeat(&self) -> OximyResult<HeartbeatResponse> {
        let (device_id, token) = self.client.ensure_authenticated().await?;

        let status = self.stats_provider.get_status();
        let stats = self.stats_provider.get_stats();

        debug!("Sending heartbeat for device {}", device_id);

        let response = self
            .client
            .http()
            .heartbeat(&device_id, &token, status, stats)
            .await?;

        // Update state
        {
            let mut last = self.last_heartbeat.write().await;
            *last = Some(Instant::now());
        }
        {
            let mut last_resp = self.last_response.write().await;
            *last_resp = Some(response.clone());
        }

        self.consecutive_failures.store(0, Ordering::Relaxed);
        self.total_sent.fetch_add(1, Ordering::Relaxed);

        // Handle commands
        if !response.commands.is_empty() {
            self.handle_commands(&response.commands).await;
        }

        // Check for policy updates
        if let Some(ref version) = response.policy_version {
            debug!("Server indicates policy version: {}", version);
        }

        Ok(response)
    }

    /// Handle server commands
    async fn handle_commands(&self, commands: &[ServerCommand]) {
        for cmd in commands {
            info!("Received server command: {:?}", cmd);

            if let Some(ref handler) = self.command_handler {
                handler(cmd.clone());
            } else {
                match cmd {
                    ServerCommand::RotateToken => {
                        warn!("Token rotation requested but no handler configured");
                    }
                    ServerCommand::FetchPolicies => {
                        debug!("Policy fetch requested");
                    }
                    ServerCommand::Restart => {
                        warn!("Restart requested - not implemented");
                    }
                    ServerCommand::Update { version } => {
                        info!("Update to version {} requested", version);
                    }
                }
            }
        }
    }

    /// Start background heartbeat task
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let interval = self.config.interval;
        let max_failures = self.config.max_failures;

        tokio::spawn(async move {
            info!(
                "Starting heartbeat service with {}s interval",
                interval.as_secs()
            );

            loop {
                // Wait for interval
                tokio::time::sleep(interval).await;

                // Check if we have credentials
                if !self.client.has_valid_credentials().await {
                    debug!("Skipping heartbeat - not enrolled");
                    continue;
                }

                // Send heartbeat
                match self.send_heartbeat().await {
                    Ok(response) => {
                        debug!("Heartbeat successful, server time: {}", response.timestamp);
                    }
                    Err(e) => {
                        let failures =
                            self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                        self.total_failed.fetch_add(1, Ordering::Relaxed);

                        if failures >= max_failures as u64 {
                            error!("Heartbeat failed {} consecutive times: {}", failures, e);
                        } else {
                            warn!("Heartbeat failed ({}/{}): {}", failures, max_failures, e);
                        }
                    }
                }
            }
        })
    }

    /// Get heartbeat statistics
    pub fn stats(&self) -> HeartbeatStats {
        HeartbeatStats {
            total_sent: self.total_sent.load(Ordering::Relaxed),
            total_failed: self.total_failed.load(Ordering::Relaxed),
            consecutive_failures: self.consecutive_failures.load(Ordering::Relaxed),
        }
    }

    /// Get time since last successful heartbeat
    pub async fn time_since_last(&self) -> Option<Duration> {
        let last = self.last_heartbeat.read().await;
        last.map(|t| t.elapsed())
    }

    /// Check if heartbeat is overdue
    pub async fn is_overdue(&self) -> bool {
        match self.time_since_last().await {
            Some(elapsed) => elapsed > self.config.interval * 2,
            None => true, // Never sent
        }
    }
}

/// Heartbeat statistics
#[derive(Debug, Clone, Default)]
pub struct HeartbeatStats {
    /// Total heartbeats sent successfully
    pub total_sent: u64,

    /// Total failed heartbeats
    pub total_failed: u64,

    /// Current consecutive failures
    pub consecutive_failures: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_config_default() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.max_failures, 5);
    }

    #[test]
    fn test_default_stats_provider() {
        let provider = DefaultStatsProvider::default();

        let stats = provider.get_stats();
        assert!(!stats.sensor_version.is_empty());
        assert!(stats.uptime_seconds < 1); // Just created

        let status = provider.get_status();
        assert_eq!(status, SensorStatus::Active);
    }

    #[test]
    fn test_heartbeat_stats_default() {
        let stats = HeartbeatStats::default();
        assert_eq!(stats.total_sent, 0);
        assert_eq!(stats.total_failed, 0);
        assert_eq!(stats.consecutive_failures, 0);
    }
}
