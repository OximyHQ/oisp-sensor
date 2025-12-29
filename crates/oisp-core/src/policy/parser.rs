//! YAML policy parser
//!
//! Parses policy files in the OISP policy DSL format.

use super::actions::PolicyAction;
use super::condition::Condition;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Policy parsing errors
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invalid policy: {0}")]
    InvalidPolicy(String),
}

/// A complete policy file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyFile {
    /// File version (for future compatibility)
    #[serde(default = "default_version")]
    pub version: String,

    /// List of policies
    pub policies: Vec<Policy>,

    /// Global settings (optional)
    #[serde(default)]
    pub settings: PolicyFileSettings,
}

fn default_version() -> String {
    "1".to_string()
}

/// Global settings for the policy file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyFileSettings {
    /// Enable debug logging for policy evaluation
    #[serde(default)]
    pub debug: bool,

    /// Default action when no policy matches
    #[serde(default)]
    pub default_action: Option<String>,
}

/// A single policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique policy ID
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description of what this policy does
    #[serde(default)]
    pub description: Option<String>,

    /// Whether the policy is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Priority (higher = evaluated first)
    #[serde(default)]
    pub priority: i32,

    /// Event types this policy applies to (empty = all)
    #[serde(default)]
    pub event_types: Vec<String>,

    /// Conditions for the policy to match
    pub conditions: Condition,

    /// Action to take when policy matches
    pub action: PolicyAction,

    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,

    /// Metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

fn default_enabled() -> bool {
    true
}

impl Policy {
    /// Create a new policy
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        conditions: Condition,
        action: PolicyAction,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            enabled: true,
            priority: 0,
            event_types: Vec::new(),
            conditions,
            action,
            tags: Vec::new(),
            metadata: Default::default(),
        }
    }

    /// Builder: set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Builder: set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Builder: limit to specific event types
    pub fn for_event_types(mut self, types: Vec<String>) -> Self {
        self.event_types = types;
        self
    }

    /// Builder: add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Check if this policy applies to a given event type
    pub fn applies_to_event_type(&self, event_type: &str) -> bool {
        self.event_types.is_empty() || self.event_types.iter().any(|t| t == event_type)
    }

    /// Validate the policy
    pub fn validate(&self) -> Result<(), ParseError> {
        if self.id.is_empty() {
            return Err(ParseError::Validation("Policy ID cannot be empty".into()));
        }
        if self.name.is_empty() {
            return Err(ParseError::Validation("Policy name cannot be empty".into()));
        }
        Ok(())
    }
}

/// Parse policies from a YAML string
pub fn parse_policies(yaml: &str) -> Result<PolicyFile, ParseError> {
    let file: PolicyFile = serde_yaml::from_str(yaml)?;

    // Validate each policy
    for policy in &file.policies {
        policy.validate()?;
    }

    Ok(file)
}

/// Parse policies from a file
pub fn parse_policies_file(path: &Path) -> Result<PolicyFile, ParseError> {
    let content = std::fs::read_to_string(path)?;
    parse_policies(&content)
}

/// Create a sample/example policy file
pub fn example_policy_file() -> PolicyFile {
    PolicyFile {
        version: "1".to_string(),
        policies: vec![
            Policy::new(
                "block-unknown-apps",
                "Block requests from unknown applications",
                Condition::all(vec![
                    Condition::equals("event_type", "ai.request"),
                    Condition::equals("app.tier", "unknown"),
                ]),
                PolicyAction::Block {
                    reason: Some("Blocked: unknown application attempting AI access".to_string()),
                },
            )
            .with_description("Prevents AI requests from unrecognized applications")
            .with_priority(100)
            .with_tags(vec!["security".to_string(), "access-control".to_string()]),
            Policy::new(
                "alert-high-token-usage",
                "Alert on high token usage",
                Condition::Simple {
                    field: "data.usage.total_tokens".into(),
                    op: super::condition::ConditionOp::Gt,
                    value: Some(serde_json::json!(10000)),
                    ignore_case: false,
                },
                PolicyAction::Alert {
                    severity: super::AlertSeverity::Warning,
                    message: "High token usage detected".to_string(),
                    webhook_url: None,
                    include_event: true,
                },
            )
            .with_description("Sends alert when token usage exceeds threshold")
            .with_priority(50)
            .for_event_types(vec!["ai.response".to_string()]),
            Policy::new(
                "redact-pii-in-prompts",
                "Redact PII from AI prompts",
                Condition::equals("event_type", "ai.request"),
                PolicyAction::Redact {
                    fields: vec!["data.messages.*.content".to_string()],
                    patterns: vec![
                        "email".to_string(),
                        "ssn".to_string(),
                        "credit_card".to_string(),
                    ],
                    custom_patterns: vec![],
                    replacement: Some("[REDACTED]".to_string()),
                },
            )
            .with_description("Automatically redact sensitive data from prompts")
            .with_priority(200),
            Policy::new(
                "allow-approved-apps",
                "Allow requests from approved applications",
                Condition::is_in(
                    "app.app_id",
                    vec!["cursor".into(), "github-copilot".into(), "vscode".into()],
                ),
                PolicyAction::Allow,
            )
            .with_description("Explicitly allow known development tools")
            .with_priority(90),
        ],
        settings: PolicyFileSettings::default(),
    }
}

/// Generate example YAML for documentation
pub fn example_yaml() -> String {
    let example = example_policy_file();
    serde_yaml::to_string(&example).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_policy() {
        let yaml = r#"
version: "1"
policies:
  - id: test-policy
    name: Test Policy
    enabled: true
    priority: 100
    conditions:
      field: event_type
      op: equals
      value: ai.request
    action:
      type: allow
"#;

        let file = parse_policies(yaml).unwrap();
        assert_eq!(file.policies.len(), 1);
        assert_eq!(file.policies[0].id, "test-policy");
        assert_eq!(file.policies[0].priority, 100);
    }

    #[test]
    fn test_parse_complex_conditions() {
        let yaml = r#"
version: "1"
policies:
  - id: complex-policy
    name: Complex Policy
    conditions:
      all:
        - field: event_type
          op: equals
          value: ai.request
        - any:
            - field: app.tier
              op: equals
              value: unknown
            - field: data.provider.name
              op: not_in
              value: ["openai", "anthropic"]
    action:
      type: block
      reason: "Blocked by complex policy"
"#;

        let file = parse_policies(yaml).unwrap();
        assert_eq!(file.policies.len(), 1);

        // Verify the condition structure
        match &file.policies[0].conditions {
            Condition::All { all } => {
                assert_eq!(all.len(), 2);
            }
            _ => panic!("Expected All condition"),
        }
    }

    #[test]
    fn test_parse_redact_action() {
        let yaml = r#"
version: "1"
policies:
  - id: redact-policy
    name: Redact PII
    conditions:
      field: event_type
      op: equals
      value: ai.request
    action:
      type: redact
      fields:
        - data.messages.*.content
      patterns:
        - email
        - ssn
      replacement: "[REDACTED]"
"#;

        let file = parse_policies(yaml).unwrap();
        match &file.policies[0].action {
            PolicyAction::Redact {
                fields, patterns, ..
            } => {
                assert_eq!(fields.len(), 1);
                assert_eq!(patterns.len(), 2);
            }
            _ => panic!("Expected Redact action"),
        }
    }

    #[test]
    fn test_example_yaml_parses() {
        let yaml = example_yaml();
        let file = parse_policies(&yaml).unwrap();
        assert!(!file.policies.is_empty());
    }

    #[test]
    fn test_validation_empty_id() {
        let policy = Policy {
            id: String::new(),
            name: "Test".to_string(),
            description: None,
            enabled: true,
            priority: 0,
            event_types: vec![],
            conditions: Condition::equals("event_type", "ai.request"),
            action: PolicyAction::Allow,
            tags: vec![],
            metadata: Default::default(),
        };

        assert!(policy.validate().is_err());
    }
}
