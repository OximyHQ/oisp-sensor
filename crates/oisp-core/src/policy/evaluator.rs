//! Policy evaluator - matches events against policies and executes actions
//!
//! The evaluator:
//! 1. Filters policies by event type
//! 2. Sorts by priority (higher first)
//! 3. Evaluates conditions
//! 4. Executes the first matching policy's action

use super::actions::{ActionExecutor, PolicyActionType};
use super::parser::Policy;
use super::{DefaultAction, PolicyResult};
use crate::events::OispEvent;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, trace};

/// Result of policy evaluation
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    /// The policy that matched (if any)
    pub matched_policy: Option<String>,
    /// Whether any policy matched
    pub matched: bool,
    /// Policies that were evaluated
    pub policies_evaluated: usize,
    /// Time taken to evaluate (microseconds)
    pub evaluation_time_us: u64,
}

/// Policy evaluator
pub struct PolicyEvaluator {
    /// Policies sorted by priority
    policies: Arc<RwLock<Vec<Policy>>>,
    /// Action executor
    action_executor: ActionExecutor,
    /// Default action when no policy matches
    default_action: DefaultAction,
    /// Enable debug logging
    debug: bool,
}

impl PolicyEvaluator {
    /// Create a new evaluator with policies
    pub fn new(
        policies: Vec<Policy>,
        default_action: DefaultAction,
        webhook_url: Option<String>,
    ) -> Self {
        let mut sorted = policies;
        // Sort by priority (higher first)
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        Self {
            policies: Arc::new(RwLock::new(sorted)),
            action_executor: ActionExecutor::new(webhook_url),
            default_action,
            debug: false,
        }
    }

    /// Enable debug logging
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Update policies (used for hot-reload)
    pub async fn update_policies(&self, policies: Vec<Policy>) {
        let mut sorted = policies;
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));
        let count = sorted.len();
        *self.policies.write().await = sorted;
        info!(count = count, "Policies updated");
    }

    /// Get current policy count
    pub async fn policy_count(&self) -> usize {
        self.policies.read().await.len()
    }

    /// Evaluate an event against all policies
    pub async fn evaluate(&self, event: &OispEvent) -> EvaluationResult {
        let start = std::time::Instant::now();
        let event_type = event.event_type();
        let policies = self.policies.read().await;

        let mut evaluated = 0;

        for policy in policies.iter() {
            // Skip disabled policies
            if !policy.enabled {
                continue;
            }

            // Check if policy applies to this event type
            if !policy.applies_to_event_type(event_type) {
                continue;
            }

            evaluated += 1;

            // Evaluate conditions
            if self.debug {
                debug!(
                    policy_id = policy.id,
                    event_type = event_type,
                    "Evaluating policy"
                );
            }

            if policy.conditions.evaluate(event) {
                if self.debug {
                    debug!(
                        policy_id = policy.id,
                        event_type = event_type,
                        "Policy matched"
                    );
                }

                return EvaluationResult {
                    matched_policy: Some(policy.id.clone()),
                    matched: true,
                    policies_evaluated: evaluated,
                    evaluation_time_us: start.elapsed().as_micros() as u64,
                };
            }
        }

        EvaluationResult {
            matched_policy: None,
            matched: false,
            policies_evaluated: evaluated,
            evaluation_time_us: start.elapsed().as_micros() as u64,
        }
    }

    /// Evaluate and execute the matching policy's action
    pub async fn evaluate_and_execute(&self, event: OispEvent) -> PolicyResult {
        let _start = std::time::Instant::now();
        let event_type = event.event_type();
        let event_id = event.envelope().event_id.clone();
        let policies = self.policies.read().await;

        for policy in policies.iter() {
            // Skip disabled policies
            if !policy.enabled {
                continue;
            }

            // Check if policy applies to this event type
            if !policy.applies_to_event_type(event_type) {
                continue;
            }

            // Evaluate conditions
            if policy.conditions.evaluate(&event) {
                trace!(
                    policy_id = policy.id,
                    event_id = event_id,
                    event_type = event_type,
                    "Policy matched, executing action"
                );

                // Execute the action
                let result = self
                    .action_executor
                    .execute(&policy.action, event, &policy.id)
                    .await;

                return PolicyResult {
                    matched_policy: Some(policy.id.clone()),
                    action: result.action_type,
                    reason: result.reason,
                    modified: result.modified,
                    alerts: result.alerts,
                };
            }
        }

        // No policy matched - apply default action
        trace!(
            event_id = event_id,
            event_type = event_type,
            default_action = ?self.default_action,
            "No policy matched, applying default action"
        );

        let action = match self.default_action {
            DefaultAction::Allow => PolicyActionType::Allow,
            DefaultAction::Block => PolicyActionType::Block,
            DefaultAction::Log => PolicyActionType::Log,
        };

        PolicyResult {
            matched_policy: None,
            action,
            reason: None,
            modified: false,
            alerts: vec![],
        }
    }

    /// Get matching policy for an event (without executing)
    pub async fn find_matching_policy(&self, event: &OispEvent) -> Option<Policy> {
        let event_type = event.event_type();
        let policies = self.policies.read().await;

        for policy in policies.iter() {
            if !policy.enabled {
                continue;
            }

            if !policy.applies_to_event_type(event_type) {
                continue;
            }

            if policy.conditions.evaluate(event) {
                return Some(policy.clone());
            }
        }

        None
    }

    /// Get all policies
    pub async fn policies(&self) -> Vec<Policy> {
        self.policies.read().await.clone()
    }

    /// Get a policy by ID
    pub async fn get_policy(&self, id: &str) -> Option<Policy> {
        self.policies
            .read()
            .await
            .iter()
            .find(|p| p.id == id)
            .cloned()
    }

    /// Get action executor for direct access
    pub fn action_executor(&self) -> &ActionExecutor {
        &self.action_executor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{AiRequestData, AiRequestEvent, AppInfo, AppTier, EventEnvelope};
    use crate::policy::actions::PolicyAction;
    use crate::policy::condition::Condition;

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
    async fn test_evaluator_matches_policy() {
        let policies = vec![Policy::new(
            "block-unknown",
            "Block Unknown Apps",
            Condition::equals("app.tier", "unknown"),
            PolicyAction::Block {
                reason: Some("Unknown app".to_string()),
            },
        )];

        let evaluator = PolicyEvaluator::new(policies, DefaultAction::Allow, None);

        // Unknown app should match
        let event = create_test_event(AppTier::Unknown);
        let result = evaluator.evaluate(&event).await;
        assert!(result.matched);
        assert_eq!(result.matched_policy, Some("block-unknown".to_string()));

        // Identified app should not match
        let event = create_test_event(AppTier::Identified);
        let result = evaluator.evaluate(&event).await;
        assert!(!result.matched);
    }

    #[tokio::test]
    async fn test_evaluator_priority_order() {
        let policies = vec![
            Policy::new(
                "low-priority",
                "Low Priority",
                Condition::equals("event_type", "ai.request"),
                PolicyAction::Allow,
            )
            .with_priority(10),
            Policy::new(
                "high-priority",
                "High Priority",
                Condition::equals("event_type", "ai.request"),
                PolicyAction::Block {
                    reason: Some("High priority block".to_string()),
                },
            )
            .with_priority(100),
        ];

        let evaluator = PolicyEvaluator::new(policies, DefaultAction::Allow, None);

        let event = create_test_event(AppTier::Unknown);
        let result = evaluator.evaluate(&event).await;

        // High priority should match first
        assert_eq!(result.matched_policy, Some("high-priority".to_string()));
    }

    #[tokio::test]
    async fn test_evaluator_event_type_filter() {
        let policies = vec![Policy::new(
            "response-only",
            "Response Only",
            Condition::equals("app.tier", "unknown"),
            PolicyAction::Alert {
                severity: crate::policy::AlertSeverity::Warning,
                message: "Test".to_string(),
                webhook_url: None,
                include_event: false,
            },
        )
        .for_event_types(vec!["ai.response".to_string()])];

        let evaluator = PolicyEvaluator::new(policies, DefaultAction::Allow, None);

        // ai.request should not match (policy only for ai.response)
        let event = create_test_event(AppTier::Unknown);
        let result = evaluator.evaluate(&event).await;
        assert!(!result.matched);
    }

    #[tokio::test]
    async fn test_evaluator_default_action() {
        let evaluator = PolicyEvaluator::new(vec![], DefaultAction::Block, None);

        let event = create_test_event(AppTier::Unknown);
        let result = evaluator.evaluate_and_execute(event).await;

        assert!(result.matched_policy.is_none());
        assert_eq!(result.action, PolicyActionType::Block);
    }

    #[tokio::test]
    async fn test_evaluator_disabled_policy() {
        let mut policy = Policy::new(
            "disabled",
            "Disabled Policy",
            Condition::equals("event_type", "ai.request"),
            PolicyAction::Block {
                reason: Some("Should not match".to_string()),
            },
        );
        policy.enabled = false;

        let evaluator = PolicyEvaluator::new(vec![policy], DefaultAction::Allow, None);

        let event = create_test_event(AppTier::Unknown);
        let result = evaluator.evaluate(&event).await;
        assert!(!result.matched);
    }

    #[tokio::test]
    async fn test_evaluator_update_policies() {
        let evaluator = PolicyEvaluator::new(vec![], DefaultAction::Allow, None);

        // Initially no policies
        assert_eq!(evaluator.policy_count().await, 0);

        // Update with new policies
        let policies = vec![Policy::new(
            "new-policy",
            "New Policy",
            Condition::equals("event_type", "ai.request"),
            PolicyAction::Allow,
        )];
        evaluator.update_policies(policies).await;

        assert_eq!(evaluator.policy_count().await, 1);
    }
}
