//! Policy action definitions and execution
//!
//! Supports:
//! - Allow: Pass the event through
//! - Block: Drop the event, optionally log reason
//! - Redact: Apply redaction patterns to specified fields
//! - Alert: Generate an alert (webhook, file, etc.)
//! - Log: Log the event with additional context

use super::{AlertSeverity, PolicyAlert};
use crate::events::OispEvent;
use crate::redaction::{redact, RedactionConfig, RedactionMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Policy action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyAction {
    /// Allow the event through unchanged
    Allow,

    /// Block/drop the event
    Block {
        /// Reason for blocking
        #[serde(default)]
        reason: Option<String>,
    },

    /// Redact sensitive data from specified fields
    Redact {
        /// Fields to redact (supports wildcards like "data.messages.*.content")
        fields: Vec<String>,
        /// Built-in patterns to apply (email, ssn, credit_card, api_key, etc.)
        #[serde(default)]
        patterns: Vec<String>,
        /// Custom regex patterns
        #[serde(default)]
        custom_patterns: Vec<String>,
        /// Replacement string (default: "[REDACTED]")
        #[serde(default)]
        replacement: Option<String>,
    },

    /// Generate an alert
    Alert {
        /// Alert severity
        #[serde(default)]
        severity: AlertSeverity,
        /// Alert message
        message: String,
        /// Webhook URL (optional, uses global config if not set)
        #[serde(default)]
        webhook_url: Option<String>,
        /// Include full event in alert
        #[serde(default = "default_true")]
        include_event: bool,
    },

    /// Log the event (but allow it through)
    Log {
        /// Log level
        #[serde(default)]
        level: LogLevel,
        /// Log message
        message: String,
        /// Additional fields to include
        #[serde(default)]
        include_fields: Vec<String>,
    },
}

fn default_true() -> bool {
    true
}

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

/// Action type enumeration (for result reporting)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyActionType {
    Allow,
    Block,
    Redact,
    Alert,
    Log,
}

impl From<&PolicyAction> for PolicyActionType {
    fn from(action: &PolicyAction) -> Self {
        match action {
            PolicyAction::Allow => PolicyActionType::Allow,
            PolicyAction::Block { .. } => PolicyActionType::Block,
            PolicyAction::Redact { .. } => PolicyActionType::Redact,
            PolicyAction::Alert { .. } => PolicyActionType::Alert,
            PolicyAction::Log { .. } => PolicyActionType::Log,
        }
    }
}

/// Result of executing an action
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// The action type that was executed
    pub action_type: PolicyActionType,
    /// Whether the event should be passed through
    pub pass_through: bool,
    /// Whether the event was modified
    pub modified: bool,
    /// The (possibly modified) event
    pub event: Option<OispEvent>,
    /// Alerts generated
    pub alerts: Vec<PolicyAlert>,
    /// Reason (for blocks, etc.)
    pub reason: Option<String>,
}

/// Action executor - executes policy actions on events
pub struct ActionExecutor {
    /// Global webhook URL for alerts
    webhook_url: Option<String>,
    /// HTTP client for webhooks
    http_client: reqwest::Client,
    /// Alert buffer for batching
    alert_buffer: Arc<RwLock<Vec<PolicyAlert>>>,
}

impl ActionExecutor {
    /// Create a new action executor
    pub fn new(webhook_url: Option<String>) -> Self {
        Self {
            webhook_url,
            http_client: reqwest::Client::new(),
            alert_buffer: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Execute an action on an event
    pub async fn execute(
        &self,
        action: &PolicyAction,
        event: OispEvent,
        policy_id: &str,
    ) -> ActionResult {
        match action {
            PolicyAction::Allow => ActionResult {
                action_type: PolicyActionType::Allow,
                pass_through: true,
                modified: false,
                event: Some(event),
                alerts: vec![],
                reason: None,
            },

            PolicyAction::Block { reason } => {
                info!(
                    policy_id = policy_id,
                    event_id = event.envelope().event_id,
                    reason = reason.as_deref().unwrap_or("no reason"),
                    "Blocking event"
                );
                ActionResult {
                    action_type: PolicyActionType::Block,
                    pass_through: false,
                    modified: false,
                    event: None,
                    alerts: vec![],
                    reason: reason.clone(),
                }
            }

            PolicyAction::Redact {
                fields,
                patterns,
                custom_patterns,
                replacement,
            } => {
                let (modified_event, was_modified) = self.execute_redaction(
                    event,
                    fields,
                    patterns,
                    custom_patterns,
                    replacement.as_deref().unwrap_or("[REDACTED]"),
                );
                ActionResult {
                    action_type: PolicyActionType::Redact,
                    pass_through: true,
                    modified: was_modified,
                    event: Some(modified_event),
                    alerts: vec![],
                    reason: None,
                }
            }

            PolicyAction::Alert {
                severity,
                message,
                webhook_url,
                include_event,
            } => {
                let alert = self
                    .create_alert(policy_id, *severity, message, &event, *include_event)
                    .await;

                // Send webhook if configured
                let url = webhook_url.as_ref().or(self.webhook_url.as_ref());
                if let Some(url) = url {
                    self.send_webhook(url, &alert).await;
                }

                ActionResult {
                    action_type: PolicyActionType::Alert,
                    pass_through: true,
                    modified: false,
                    event: Some(event),
                    alerts: vec![alert],
                    reason: None,
                }
            }

            PolicyAction::Log {
                level,
                message,
                include_fields,
            } => {
                self.execute_log(*level, message, &event, include_fields);
                ActionResult {
                    action_type: PolicyActionType::Log,
                    pass_through: true,
                    modified: false,
                    event: Some(event),
                    alerts: vec![],
                    reason: None,
                }
            }
        }
    }

    /// Execute redaction on an event
    fn execute_redaction(
        &self,
        event: OispEvent,
        fields: &[String],
        patterns: &[String],
        custom_patterns: &[String],
        replacement: &str,
    ) -> (OispEvent, bool) {
        // Convert event to JSON for field manipulation
        let mut json = match serde_json::to_value(&event) {
            Ok(v) => v,
            Err(_) => return (event, false),
        };

        let mut modified = false;

        // Build redaction config from patterns
        let mut config = RedactionConfig {
            mode: RedactionMode::Safe,
            redact_api_keys: patterns.iter().any(|p| p == "api_key" || p == "api_keys"),
            redact_emails: patterns.iter().any(|p| p == "email" || p == "emails"),
            redact_credit_cards: patterns.iter().any(|p| p == "credit_card" || p == "cc"),
            redact_ssn: patterns.iter().any(|p| p == "ssn"),
            redact_phone_numbers: patterns.iter().any(|p| p == "phone"),
            custom_patterns: custom_patterns.to_vec(),
        };

        // If "all" pattern is specified, enable all built-in patterns
        if patterns.iter().any(|p| p == "all") {
            config.redact_api_keys = true;
            config.redact_emails = true;
            config.redact_credit_cards = true;
            config.redact_ssn = true;
            config.redact_phone_numbers = true;
        }

        // Apply redaction to each specified field
        for field_pattern in fields {
            if let Some(field_modified) =
                self.redact_field(&mut json, field_pattern, &config, replacement)
            {
                modified = modified || field_modified;
            }
        }

        // Convert back to event
        if modified {
            match serde_json::from_value(json) {
                Ok(new_event) => (new_event, true),
                Err(e) => {
                    warn!("Failed to deserialize redacted event: {}", e);
                    (event, false)
                }
            }
        } else {
            (event, false)
        }
    }

    /// Redact a field by pattern (supports wildcards)
    fn redact_field(
        &self,
        json: &mut serde_json::Value,
        field_pattern: &str,
        config: &RedactionConfig,
        replacement: &str,
    ) -> Option<bool> {
        let segments: Vec<&str> = field_pattern.split('.').collect();
        self.redact_field_recursive(json, &segments, config, replacement)
    }

    /// Recursively navigate and redact fields
    fn redact_field_recursive(
        &self,
        value: &mut serde_json::Value,
        segments: &[&str],
        config: &RedactionConfig,
        _replacement: &str,
    ) -> Option<bool> {
        if segments.is_empty() {
            // Reached the target field - redact it
            if let Some(s) = value.as_str() {
                let result = redact(s, config);
                if !result.findings.is_empty() {
                    // Replace with custom replacement or use redacted content
                    *value =
                        serde_json::Value::String(if result.findings.iter().any(|f| f.count > 0) {
                            result.content
                        } else {
                            s.to_string()
                        });
                    return Some(true);
                }
            }
            return Some(false);
        }

        let segment = segments[0];
        let remaining = &segments[1..];

        match value {
            serde_json::Value::Object(map) => {
                if segment == "*" {
                    // Wildcard - apply to all keys
                    let mut any_modified = false;
                    for (_, v) in map.iter_mut() {
                        if let Some(m) =
                            self.redact_field_recursive(v, remaining, config, _replacement)
                        {
                            any_modified = any_modified || m;
                        }
                    }
                    Some(any_modified)
                } else if let Some(v) = map.get_mut(segment) {
                    self.redact_field_recursive(v, remaining, config, _replacement)
                } else {
                    None
                }
            }
            serde_json::Value::Array(arr) => {
                if segment == "*" {
                    // Wildcard - apply to all elements
                    let mut any_modified = false;
                    for item in arr.iter_mut() {
                        if let Some(m) =
                            self.redact_field_recursive(item, remaining, config, _replacement)
                        {
                            any_modified = any_modified || m;
                        }
                    }
                    Some(any_modified)
                } else if let Ok(idx) = segment.parse::<usize>() {
                    if let Some(v) = arr.get_mut(idx) {
                        self.redact_field_recursive(v, remaining, config, _replacement)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Create an alert
    async fn create_alert(
        &self,
        policy_id: &str,
        severity: AlertSeverity,
        message: &str,
        event: &OispEvent,
        include_event: bool,
    ) -> PolicyAlert {
        let mut context = HashMap::new();

        if include_event {
            if let Ok(event_json) = serde_json::to_value(event) {
                context.insert("event".to_string(), event_json);
            }
        }

        // Add some basic event info
        context.insert(
            "event_type".to_string(),
            serde_json::Value::String(event.event_type().to_string()),
        );

        PolicyAlert {
            id: ulid::Ulid::new().to_string(),
            policy_id: policy_id.to_string(),
            severity,
            message: message.to_string(),
            event_id: event.envelope().event_id.clone(),
            timestamp: chrono::Utc::now(),
            context,
        }
    }

    /// Send an alert to a webhook
    async fn send_webhook(&self, url: &str, alert: &PolicyAlert) {
        debug!(url = url, alert_id = alert.id, "Sending webhook alert");

        match self.http_client.post(url).json(alert).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    debug!(url = url, "Webhook alert sent successfully");
                } else {
                    warn!(
                        url = url,
                        status = %response.status(),
                        "Webhook alert failed"
                    );
                }
            }
            Err(e) => {
                warn!(url = url, error = %e, "Failed to send webhook alert");
            }
        }
    }

    /// Execute a log action
    fn execute_log(
        &self,
        level: LogLevel,
        message: &str,
        event: &OispEvent,
        include_fields: &[String],
    ) {
        let event_id = &event.envelope().event_id;
        let event_type = event.event_type();

        // Extract specified fields
        let mut fields_str = String::new();
        if !include_fields.is_empty() {
            if let Ok(json) = serde_json::to_value(event) {
                for field in include_fields {
                    if let Some(value) = extract_json_field(&json, field) {
                        if !fields_str.is_empty() {
                            fields_str.push_str(", ");
                        }
                        fields_str.push_str(&format!("{}={}", field, value));
                    }
                }
            }
        }

        match level {
            LogLevel::Debug => {
                debug!(
                    event_id = event_id,
                    event_type = event_type,
                    fields = fields_str,
                    "[POLICY LOG] {}",
                    message
                );
            }
            LogLevel::Info => {
                info!(
                    event_id = event_id,
                    event_type = event_type,
                    fields = fields_str,
                    "[POLICY LOG] {}",
                    message
                );
            }
            LogLevel::Warn => {
                warn!(
                    event_id = event_id,
                    event_type = event_type,
                    fields = fields_str,
                    "[POLICY LOG] {}",
                    message
                );
            }
            LogLevel::Error => {
                tracing::error!(
                    event_id = event_id,
                    event_type = event_type,
                    fields = fields_str,
                    "[POLICY LOG] {}",
                    message
                );
            }
        }
    }

    /// Get pending alerts
    pub async fn pending_alerts(&self) -> Vec<PolicyAlert> {
        self.alert_buffer.read().await.clone()
    }

    /// Clear pending alerts
    pub async fn clear_alerts(&self) {
        self.alert_buffer.write().await.clear();
    }
}

/// Extract a field from JSON using dot notation
fn extract_json_field(json: &serde_json::Value, path: &str) -> Option<String> {
    let mut current = json;
    for segment in path.split('.') {
        match current {
            serde_json::Value::Object(map) => {
                current = map.get(segment)?;
            }
            serde_json::Value::Array(arr) => {
                if let Ok(idx) = segment.parse::<usize>() {
                    current = arr.get(idx)?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(match current {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => current.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{AiRequestData, AiRequestEvent, EventEnvelope};

    fn create_test_event() -> OispEvent {
        let envelope = EventEnvelope::new("ai.request");

        OispEvent::AiRequest(AiRequestEvent {
            envelope,
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
    async fn test_allow_action() {
        let executor = ActionExecutor::new(None);
        let event = create_test_event();
        let event_id = event.envelope().event_id.clone();

        let result = executor
            .execute(&PolicyAction::Allow, event, "test-policy")
            .await;

        assert!(result.pass_through);
        assert!(!result.modified);
        assert_eq!(result.event.unwrap().envelope().event_id, event_id);
    }

    #[tokio::test]
    async fn test_block_action() {
        let executor = ActionExecutor::new(None);
        let event = create_test_event();

        let result = executor
            .execute(
                &PolicyAction::Block {
                    reason: Some("Test block".to_string()),
                },
                event,
                "test-policy",
            )
            .await;

        assert!(!result.pass_through);
        assert!(result.event.is_none());
        assert_eq!(result.reason, Some("Test block".to_string()));
    }

    #[tokio::test]
    async fn test_alert_action() {
        let executor = ActionExecutor::new(None);
        let event = create_test_event();

        let result = executor
            .execute(
                &PolicyAction::Alert {
                    severity: AlertSeverity::Warning,
                    message: "Test alert".to_string(),
                    webhook_url: None,
                    include_event: true,
                },
                event,
                "test-policy",
            )
            .await;

        assert!(result.pass_through);
        assert_eq!(result.alerts.len(), 1);
        assert_eq!(result.alerts[0].message, "Test alert");
        assert_eq!(result.alerts[0].policy_id, "test-policy");
    }
}
