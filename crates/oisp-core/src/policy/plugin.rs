//! Policy plugin - integrates the policy engine with the event pipeline
//!
//! Implements the ActionPlugin trait to process events through policies.

use super::actions::PolicyActionType;
use super::audit::{AuditLogger, AuditLoggerConfig};
use super::evaluator::PolicyEvaluator;
use super::manager::{PolicyManager, PolicyManagerConfig, PolicyManagerError};
use super::parser::Policy;
use super::PolicyConfig;
use crate::events::OispEvent;
use crate::plugins::{
    ActionPlugin, EventAction, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
};
use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use tracing::{debug, info, trace};

/// Policy plugin - evaluates events against policies and executes actions
pub struct PolicyPlugin {
    /// Policy manager (handles loading and hot-reload)
    manager: Option<PolicyManager>,
    /// Audit logger
    audit_logger: Option<Arc<AuditLogger>>,
    /// Configuration
    config: PolicyConfig,
    /// Whether the plugin is initialized
    initialized: bool,
}

impl PolicyPlugin {
    /// Create a new policy plugin with default configuration
    pub fn new() -> Self {
        Self {
            manager: None,
            audit_logger: None,
            config: PolicyConfig::default(),
            initialized: false,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: PolicyConfig) -> Self {
        Self {
            manager: None,
            audit_logger: None,
            config,
            initialized: false,
        }
    }

    /// Create and initialize the plugin
    pub async fn create_initialized(config: PolicyConfig) -> Result<Self, PolicyManagerError> {
        let mut plugin = Self::with_config(config);
        plugin.initialize().await?;
        Ok(plugin)
    }

    /// Initialize the plugin (load policies, start hot-reload)
    pub async fn initialize(&mut self) -> Result<(), PolicyManagerError> {
        if self.initialized {
            return Ok(());
        }

        // Create policy manager
        let manager_config = PolicyManagerConfig {
            policy_file: self.config.policy_file.clone(),
            hot_reload: self.config.hot_reload,
            default_action: self.config.default_action,
            webhook_url: self.config.alert_webhook_url.clone(),
            ..Default::default()
        };

        self.manager = Some(PolicyManager::new(manager_config).await?);

        // Create audit logger if enabled
        if self.config.audit_enabled {
            let audit_config = AuditLoggerConfig {
                file_path: self.config.audit_file.clone(),
                ..Default::default()
            };
            self.audit_logger = Some(Arc::new(AuditLogger::new(audit_config)));
        }

        self.initialized = true;

        let policy_count = if let Some(ref m) = self.manager {
            m.policy_count().await
        } else {
            0
        };

        info!(
            policy_file = %self.config.policy_file.display(),
            policy_count = policy_count,
            hot_reload = self.config.hot_reload,
            audit_enabled = self.config.audit_enabled,
            "Policy plugin initialized"
        );

        Ok(())
    }

    /// Get the policy evaluator
    pub fn evaluator(&self) -> Option<Arc<PolicyEvaluator>> {
        self.manager.as_ref().map(|m| m.evaluator())
    }

    /// Get the audit logger
    pub fn audit_logger(&self) -> Option<Arc<AuditLogger>> {
        self.audit_logger.clone()
    }

    /// Get current policy count
    pub async fn policy_count(&self) -> usize {
        if let Some(ref manager) = self.manager {
            manager.policy_count().await
        } else {
            0
        }
    }

    /// Get all policies
    pub async fn policies(&self) -> Vec<Policy> {
        if let Some(ref manager) = self.manager {
            manager.policies().await
        } else {
            vec![]
        }
    }

    /// Force reload policies
    pub async fn reload_policies(&self) -> Result<(), PolicyManagerError> {
        if let Some(ref manager) = self.manager {
            manager.reload().await
        } else {
            Err(PolicyManagerError::LoadError(
                "Plugin not initialized".into(),
            ))
        }
    }
}

impl Default for PolicyPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginInfo for PolicyPlugin {
    fn name(&self) -> &str {
        "policy"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Policy engine for blocking, redacting, and alerting on AI events"
    }
}

impl Plugin for PolicyPlugin {
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()> {
        // Extract configuration from plugin config
        if let Some(policy_file) = config.get::<String>("policy_file") {
            self.config.policy_file = policy_file.into();
        }
        if let Some(hot_reload) = config.get::<bool>("hot_reload") {
            self.config.hot_reload = hot_reload;
        }
        if let Some(audit_enabled) = config.get::<bool>("audit_enabled") {
            self.config.audit_enabled = audit_enabled;
        }
        if let Some(audit_file) = config.get::<String>("audit_file") {
            self.config.audit_file = Some(audit_file.into());
        }
        if let Some(webhook_url) = config.get::<String>("alert_webhook_url") {
            self.config.alert_webhook_url = Some(webhook_url);
        }

        // Note: actual initialization happens asynchronously via initialize()
        // This is called synchronously from the pipeline setup

        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        // Cleanup would happen here
        self.initialized = false;
        info!("Policy plugin shutdown");
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
impl ActionPlugin for PolicyPlugin {
    async fn process(&self, event: OispEvent) -> PluginResult<(OispEvent, EventAction)> {
        // If not initialized, pass through
        let manager = match &self.manager {
            Some(m) => m,
            None => {
                debug!("Policy plugin not initialized, passing through");
                return Ok((event, EventAction::Pass));
            }
        };

        let evaluator = manager.evaluator();
        let event_type = event.event_type();
        let event_id = event.envelope().event_id.clone();

        trace!(
            event_id = event_id,
            event_type = event_type,
            "Evaluating event against policies"
        );

        // Evaluate and execute
        let result = evaluator.evaluate_and_execute(event.clone()).await;

        // Audit log the decision
        if let Some(ref logger) = self.audit_logger {
            // Get policy name if we have a matched policy
            let policy_name = if let Some(ref policy_id) = result.matched_policy {
                evaluator.get_policy(policy_id).await.map(|p| p.name)
            } else {
                None
            };

            logger
                .log_evaluation(
                    &event,
                    result.matched_policy.as_deref(),
                    policy_name.as_deref(),
                    result.action,
                    result.reason.as_deref(),
                    result.modified,
                )
                .await;

            // Also log any alerts
            for alert in &result.alerts {
                logger.log_alert(alert, &event).await;
            }
        }

        // Determine pipeline action based on policy result
        match result.action {
            PolicyActionType::Block => {
                debug!(
                    event_id = event_id,
                    policy_id = result.matched_policy,
                    "Event blocked by policy"
                );
                Ok((event, EventAction::Drop))
            }
            PolicyActionType::Allow | PolicyActionType::Log | PolicyActionType::Alert => {
                trace!(
                    event_id = event_id,
                    action = ?result.action,
                    "Event allowed by policy"
                );
                Ok((event, EventAction::Pass))
            }
            PolicyActionType::Redact => {
                // For redact, we need to get the modified event from the action executor
                // The evaluate_and_execute returns the original event, but the redacted version
                // is applied internally. We need to re-evaluate to get the modified event.
                debug!(
                    event_id = event_id,
                    policy_id = result.matched_policy,
                    "Event modified by policy (redaction)"
                );

                // Re-find the policy and execute to get the modified event
                if let Some(policy_id) = &result.matched_policy {
                    if let Some(policy) = evaluator.get_policy(policy_id).await {
                        let action_result = evaluator
                            .action_executor()
                            .execute(&policy.action, event.clone(), policy_id)
                            .await;

                        if let Some(modified_event) = action_result.event {
                            return Ok((modified_event, EventAction::Modified));
                        }
                    }
                }

                Ok((event, EventAction::Pass))
            }
        }
    }

    fn applies_to(&self, _event: &OispEvent) -> bool {
        // Policy plugin applies to all events (policies themselves filter by type)
        // But we can optimize by checking if we have any policies at all
        self.initialized
    }
}

/// Create a policy plugin from sensor configuration
pub async fn create_policy_plugin(
    config: PolicyConfig,
) -> Result<Box<dyn ActionPlugin>, PluginError> {
    let plugin = PolicyPlugin::create_initialized(config)
        .await
        .map_err(|e| PluginError::InitializationFailed(e.to_string()))?;

    Ok(Box::new(plugin))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{AiRequestData, AiRequestEvent, AppInfo, AppTier, EventEnvelope};
    use tempfile::tempdir;

    fn create_test_event(app_tier: AppTier) -> OispEvent {
        let mut envelope = EventEnvelope::new("ai.request");
        envelope.app = Some(AppInfo {
            app_id: Some("test-app".to_string()),
            name: None,
            vendor: None,
            version: None,
            bundle_id: None,
            category: None,
            tier: app_tier,
            is_ai_app: None,
            is_ai_host: None,
        });

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
    async fn test_policy_plugin_uninitialized_passes() {
        let plugin = PolicyPlugin::new();

        let event = create_test_event(AppTier::Unknown);
        let (_, action) = plugin.process(event).await.unwrap();

        assert!(matches!(action, EventAction::Pass));
    }

    #[tokio::test]
    async fn test_policy_plugin_initialized() {
        let dir = tempdir().unwrap();
        let policy_file = dir.path().join("policies.yaml");

        // Create a policy that blocks unknown apps
        let yaml = r#"
version: "1"
policies:
  - id: block-unknown
    name: Block Unknown
    conditions:
      field: app.tier
      op: equals
      value: unknown
    action:
      type: block
      reason: "Unknown app"
"#;
        std::fs::write(&policy_file, yaml).unwrap();

        let config = PolicyConfig {
            policy_file: policy_file.clone(),
            hot_reload: false,
            audit_enabled: false,
            ..Default::default()
        };

        let plugin = PolicyPlugin::create_initialized(config).await.unwrap();

        // Unknown app should be blocked
        let event = create_test_event(AppTier::Unknown);
        let (_, action) = plugin.process(event).await.unwrap();
        assert!(matches!(action, EventAction::Drop));

        // Identified app should pass
        let event = create_test_event(AppTier::Identified);
        let (_, action) = plugin.process(event).await.unwrap();
        assert!(matches!(action, EventAction::Pass));
    }

    #[tokio::test]
    async fn test_policy_plugin_with_audit() {
        let dir = tempdir().unwrap();
        let policy_file = dir.path().join("policies.yaml");
        let audit_file = dir.path().join("audit.jsonl");

        let yaml = r#"
version: "1"
policies:
  - id: allow-all
    name: Allow All
    conditions:
      field: event_type
      op: exists
    action:
      type: allow
"#;
        std::fs::write(&policy_file, yaml).unwrap();

        let config = PolicyConfig {
            policy_file,
            hot_reload: false,
            audit_enabled: true,
            audit_file: Some(audit_file.clone()),
            ..Default::default()
        };

        let plugin = PolicyPlugin::create_initialized(config).await.unwrap();

        // Process an event
        let event = create_test_event(AppTier::Identified);
        let _ = plugin.process(event).await.unwrap();

        // Flush audit logger
        if let Some(logger) = plugin.audit_logger() {
            logger.flush().await;
        }

        // Give it time to write
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Verify audit file exists and has content
        let content = std::fs::read_to_string(&audit_file).unwrap_or_default();
        assert!(content.contains("allow-all") || content.contains("allow"));
    }
}
