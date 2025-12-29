//! Audit logging for policy decisions
//!
//! Records all policy evaluations and actions for:
//! - Compliance and governance
//! - Security incident investigation
//! - Policy debugging and tuning

use super::actions::PolicyActionType;
use super::PolicyAlert;
use crate::events::OispEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, warn};

/// Audit event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AuditSeverity {
    /// Informational - normal operations
    #[default]
    Info,
    /// Warning - potential issue but allowed
    Warning,
    /// Alert - security-relevant action taken
    Alert,
    /// Critical - blocked or high-risk event
    Critical,
}

impl From<PolicyActionType> for AuditSeverity {
    fn from(action: PolicyActionType) -> Self {
        match action {
            PolicyActionType::Allow => AuditSeverity::Info,
            PolicyActionType::Log => AuditSeverity::Info,
            PolicyActionType::Redact => AuditSeverity::Warning,
            PolicyActionType::Alert => AuditSeverity::Alert,
            PolicyActionType::Block => AuditSeverity::Critical,
        }
    }
}

/// Audit event - records a policy decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique audit event ID
    pub audit_id: String,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Severity level
    pub severity: AuditSeverity,

    /// The event ID that was evaluated
    pub event_id: String,

    /// Event type
    pub event_type: String,

    /// Policy that matched (if any)
    pub policy_id: Option<String>,

    /// Policy name (if matched)
    pub policy_name: Option<String>,

    /// Action taken
    pub action: PolicyActionType,

    /// Reason for the action
    pub reason: Option<String>,

    /// Whether event was modified (e.g., redacted)
    pub modified: bool,

    /// App info (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,

    /// Process info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_name: Option<String>,

    /// Process ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,

    /// Additional context
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub context: HashMap<String, serde_json::Value>,
}

impl AuditEvent {
    /// Create a new audit event from a policy evaluation
    pub fn from_policy_result(
        event: &OispEvent,
        policy_id: Option<&str>,
        policy_name: Option<&str>,
        action: PolicyActionType,
        reason: Option<&str>,
        modified: bool,
    ) -> Self {
        let envelope = event.envelope();

        Self {
            audit_id: ulid::Ulid::new().to_string(),
            timestamp: Utc::now(),
            severity: AuditSeverity::from(action),
            event_id: envelope.event_id.clone(),
            event_type: event.event_type().to_string(),
            policy_id: policy_id.map(String::from),
            policy_name: policy_name.map(String::from),
            action,
            reason: reason.map(String::from),
            modified,
            app_id: envelope.app.as_ref().and_then(|a| a.app_id.clone()),
            process_name: envelope.process.as_ref().and_then(|p| p.name.clone()),
            pid: envelope.process.as_ref().map(|p| p.pid),
            context: HashMap::new(),
        }
    }

    /// Add context to the audit event
    pub fn with_context(mut self, key: &str, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.context.insert(key.to_string(), v);
        }
        self
    }

    /// Create an audit event for an alert
    pub fn from_alert(alert: &PolicyAlert, event: &OispEvent) -> Self {
        let envelope = event.envelope();

        Self {
            audit_id: ulid::Ulid::new().to_string(),
            timestamp: alert.timestamp,
            severity: match alert.severity {
                super::AlertSeverity::Info => AuditSeverity::Info,
                super::AlertSeverity::Warning => AuditSeverity::Alert,
                super::AlertSeverity::Critical => AuditSeverity::Critical,
            },
            event_id: alert.event_id.clone(),
            event_type: event.event_type().to_string(),
            policy_id: Some(alert.policy_id.clone()),
            policy_name: None,
            action: PolicyActionType::Alert,
            reason: Some(alert.message.clone()),
            modified: false,
            app_id: envelope.app.as_ref().and_then(|a| a.app_id.clone()),
            process_name: envelope.process.as_ref().and_then(|p| p.name.clone()),
            pid: envelope.process.as_ref().map(|p| p.pid),
            context: alert.context.clone(),
        }
    }
}

/// Audit logger configuration
#[derive(Debug, Clone)]
pub struct AuditLoggerConfig {
    /// Output file path (None = stdout)
    pub file_path: Option<PathBuf>,
    /// Buffer size before flush
    pub buffer_size: usize,
    /// Flush interval (milliseconds)
    pub flush_interval_ms: u64,
    /// Include full event in audit log
    pub include_full_event: bool,
    /// Minimum severity to log
    pub min_severity: AuditSeverity,
}

impl Default for AuditLoggerConfig {
    fn default() -> Self {
        Self {
            file_path: None,
            buffer_size: 100,
            flush_interval_ms: 1000,
            include_full_event: false,
            min_severity: AuditSeverity::Info,
        }
    }
}

/// Audit logger - writes audit events to file or stdout
pub struct AuditLogger {
    config: AuditLoggerConfig,
    buffer: Arc<RwLock<Vec<AuditEvent>>>,
    sender: mpsc::Sender<AuditEvent>,
    enabled: Arc<RwLock<bool>>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(config: AuditLoggerConfig) -> Self {
        let (tx, rx) = mpsc::channel::<AuditEvent>(1000);
        let buffer = Arc::new(RwLock::new(Vec::new()));
        let enabled = Arc::new(RwLock::new(true));

        let logger = Self {
            config: config.clone(),
            buffer: buffer.clone(),
            sender: tx,
            enabled: enabled.clone(),
        };

        // Start background writer
        let buffer_clone = buffer.clone();
        let enabled_clone = enabled.clone();
        tokio::spawn(Self::writer_task(rx, config, buffer_clone, enabled_clone));

        logger
    }

    /// Background writer task
    async fn writer_task(
        mut rx: mpsc::Receiver<AuditEvent>,
        config: AuditLoggerConfig,
        buffer: Arc<RwLock<Vec<AuditEvent>>>,
        enabled: Arc<RwLock<bool>>,
    ) {
        let flush_interval = tokio::time::Duration::from_millis(config.flush_interval_ms);
        let mut flush_timer = tokio::time::interval(flush_interval);

        loop {
            tokio::select! {
                Some(event) = rx.recv() => {
                    // Check if enabled
                    if !*enabled.read().await {
                        continue;
                    }

                    // Check severity threshold
                    if !Self::meets_severity_threshold(&event, &config) {
                        continue;
                    }

                    // Add to buffer
                    let mut buf = buffer.write().await;
                    buf.push(event);

                    // Flush if buffer is full
                    if buf.len() >= config.buffer_size {
                        let events: Vec<AuditEvent> = buf.drain(..).collect();
                        drop(buf);
                        Self::flush_events(&events, &config).await;
                    }
                }
                _ = flush_timer.tick() => {
                    // Periodic flush
                    let mut buf = buffer.write().await;
                    if !buf.is_empty() {
                        let events: Vec<AuditEvent> = buf.drain(..).collect();
                        drop(buf);
                        Self::flush_events(&events, &config).await;
                    }
                }
            }
        }
    }

    /// Check if event meets severity threshold
    fn meets_severity_threshold(event: &AuditEvent, config: &AuditLoggerConfig) -> bool {
        let event_level = match event.severity {
            AuditSeverity::Info => 0,
            AuditSeverity::Warning => 1,
            AuditSeverity::Alert => 2,
            AuditSeverity::Critical => 3,
        };

        let threshold = match config.min_severity {
            AuditSeverity::Info => 0,
            AuditSeverity::Warning => 1,
            AuditSeverity::Alert => 2,
            AuditSeverity::Critical => 3,
        };

        event_level >= threshold
    }

    /// Flush events to output
    async fn flush_events(events: &[AuditEvent], config: &AuditLoggerConfig) {
        if events.is_empty() {
            return;
        }

        // Format as JSONL
        let mut output = String::new();
        for event in events {
            if let Ok(json) = serde_json::to_string(event) {
                output.push_str(&json);
                output.push('\n');
            }
        }

        if let Some(ref path) = config.file_path {
            // Write to file
            match OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .await
            {
                Ok(mut file) => {
                    if let Err(e) = file.write_all(output.as_bytes()).await {
                        error!(error = %e, "Failed to write audit log");
                    }
                }
                Err(e) => {
                    error!(path = %path.display(), error = %e, "Failed to open audit log file");
                }
            }
        } else {
            // Write to stdout
            print!("{}", output);
        }

        debug!(count = events.len(), "Flushed audit events");
    }

    /// Log an audit event
    pub async fn log(&self, event: AuditEvent) {
        if let Err(e) = self.sender.send(event).await {
            warn!(error = %e, "Failed to send audit event");
        }
    }

    /// Log a policy evaluation result
    pub async fn log_evaluation(
        &self,
        event: &OispEvent,
        policy_id: Option<&str>,
        policy_name: Option<&str>,
        action: PolicyActionType,
        reason: Option<&str>,
        modified: bool,
    ) {
        let audit_event =
            AuditEvent::from_policy_result(event, policy_id, policy_name, action, reason, modified);
        self.log(audit_event).await;
    }

    /// Log an alert
    pub async fn log_alert(&self, alert: &PolicyAlert, event: &OispEvent) {
        let audit_event = AuditEvent::from_alert(alert, event);
        self.log(audit_event).await;
    }

    /// Enable/disable logging
    pub async fn set_enabled(&self, enabled: bool) {
        *self.enabled.write().await = enabled;
    }

    /// Flush any pending events
    pub async fn flush(&self) {
        let mut buf = self.buffer.write().await;
        if !buf.is_empty() {
            let events: Vec<AuditEvent> = buf.drain(..).collect();
            drop(buf);
            Self::flush_events(&events, &self.config).await;
        }
    }

    /// Get pending event count
    pub async fn pending_count(&self) -> usize {
        self.buffer.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{AiRequestData, AiRequestEvent, EventEnvelope};
    use tempfile::tempdir;

    fn create_test_event() -> OispEvent {
        OispEvent::AiRequest(AiRequestEvent {
            envelope: EventEnvelope::new("ai.request"),
            data: AiRequestData {
                request_id: "req-123".to_string(),
                provider: None,
                model: None,
                auth: None,
                request_type: None,
                streaming: None,
                messages: vec![],
                messages_count: None,
                has_system_prompt: None,
                system_prompt_hash: None,
                tools: vec![],
                tools_count: None,
                tool_choice: None,
                parameters: None,
                has_rag_context: None,
                has_images: None,
                image_count: None,
                estimated_tokens: None,
                conversation: None,
                agent: None,
            },
        })
    }

    #[tokio::test]
    async fn test_audit_event_creation() {
        let event = create_test_event();

        let audit = AuditEvent::from_policy_result(
            &event,
            Some("test-policy"),
            Some("Test Policy"),
            PolicyActionType::Allow,
            None,
            false,
        );

        assert_eq!(audit.policy_id, Some("test-policy".to_string()));
        assert_eq!(audit.action, PolicyActionType::Allow);
        assert_eq!(audit.severity, AuditSeverity::Info);
    }

    #[tokio::test]
    async fn test_audit_severity_from_action() {
        assert_eq!(
            AuditSeverity::from(PolicyActionType::Allow),
            AuditSeverity::Info
        );
        assert_eq!(
            AuditSeverity::from(PolicyActionType::Block),
            AuditSeverity::Critical
        );
        assert_eq!(
            AuditSeverity::from(PolicyActionType::Redact),
            AuditSeverity::Warning
        );
        assert_eq!(
            AuditSeverity::from(PolicyActionType::Alert),
            AuditSeverity::Alert
        );
    }

    #[tokio::test]
    async fn test_audit_logger_writes_to_file() {
        let dir = tempdir().unwrap();
        let audit_file = dir.path().join("audit.jsonl");

        let config = AuditLoggerConfig {
            file_path: Some(audit_file.clone()),
            buffer_size: 1, // Flush immediately
            flush_interval_ms: 100,
            ..Default::default()
        };

        let logger = AuditLogger::new(config);
        let event = create_test_event();

        logger
            .log_evaluation(
                &event,
                Some("test-policy"),
                Some("Test"),
                PolicyActionType::Allow,
                None,
                false,
            )
            .await;

        // Wait for flush
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        logger.flush().await;

        // Verify file was written
        let content = std::fs::read_to_string(&audit_file).unwrap_or_default();
        assert!(content.contains("test-policy"));
    }
}
