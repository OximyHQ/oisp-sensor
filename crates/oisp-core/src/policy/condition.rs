//! Condition types and evaluation for the policy DSL
//!
//! Supports:
//! - Field path navigation (e.g., "app.tier", "data.provider.name")
//! - Comparison operators (equals, not_equals, contains, etc.)
//! - List membership (in, not_in)
//! - Pattern matching (matches regex)
//! - Numeric comparisons (gt, lt, gte, lte)
//! - Boolean logic (all, any, not)

use crate::events::OispEvent;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A field path for accessing nested event data
/// e.g., "app.tier", "data.provider.name", "process.exe"
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FieldPath(pub String);

impl FieldPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// Split the path into segments
    pub fn segments(&self) -> Vec<&str> {
        self.0.split('.').collect()
    }

    /// Get a value from an event using this path
    pub fn extract(&self, event: &OispEvent) -> Option<FieldValue> {
        // Convert event to JSON for flexible field access
        let json = serde_json::to_value(event).ok()?;
        self.extract_from_json(&json)
    }

    /// Extract from a JSON value
    fn extract_from_json(&self, value: &serde_json::Value) -> Option<FieldValue> {
        let mut current = value;

        for segment in self.segments() {
            match current {
                serde_json::Value::Object(map) => {
                    current = map.get(segment)?;
                }
                serde_json::Value::Array(arr) => {
                    // Support array indexing: "messages.0.content"
                    if let Ok(idx) = segment.parse::<usize>() {
                        current = arr.get(idx)?;
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        Some(FieldValue::from_json(current))
    }
}

impl From<&str> for FieldPath {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for FieldPath {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// A value extracted from an event field
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<FieldValue>),
    Null,
}

impl FieldValue {
    pub fn from_json(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::String(s) => FieldValue::String(s.clone()),
            serde_json::Value::Number(n) => FieldValue::Number(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::Bool(b) => FieldValue::Bool(*b),
            serde_json::Value::Array(arr) => {
                FieldValue::Array(arr.iter().map(Self::from_json).collect())
            }
            serde_json::Value::Null => FieldValue::Null,
            serde_json::Value::Object(_) => {
                // Convert object to string representation
                FieldValue::String(value.to_string())
            }
        }
    }

    /// Get as string if possible
    pub fn as_str(&self) -> Option<&str> {
        match self {
            FieldValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as number if possible
    pub fn as_number(&self) -> Option<f64> {
        match self {
            FieldValue::Number(n) => Some(*n),
            FieldValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Get as bool if possible
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            FieldValue::Bool(b) => Some(*b),
            FieldValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "yes" | "1" => Some(true),
                "false" | "no" | "0" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// Check if null
    pub fn is_null(&self) -> bool {
        matches!(self, FieldValue::Null)
    }
}

/// Comparison operators for conditions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOp {
    /// Exact equality
    Equals,
    /// Not equal
    NotEquals,
    /// String contains substring
    Contains,
    /// String does not contain substring
    NotContains,
    /// String starts with prefix
    StartsWith,
    /// String ends with suffix
    EndsWith,
    /// Regex match
    Matches,
    /// Value is in list
    In,
    /// Value is not in list
    NotIn,
    /// Greater than (numeric)
    Gt,
    /// Greater than or equal (numeric)
    Gte,
    /// Less than (numeric)
    Lt,
    /// Less than or equal (numeric)
    Lte,
    /// Field exists (is not null)
    Exists,
    /// Field does not exist (is null)
    NotExists,
}

/// A single condition in a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    /// Simple field comparison
    Simple {
        field: FieldPath,
        op: ConditionOp,
        #[serde(default)]
        value: Option<serde_json::Value>,
        /// Case-insensitive comparison (for string ops)
        #[serde(default)]
        ignore_case: bool,
    },
    /// All conditions must match (AND)
    All { all: Vec<Condition> },
    /// Any condition must match (OR)
    Any { any: Vec<Condition> },
    /// Negate a condition (NOT)
    Not { not: Box<Condition> },
}

impl Condition {
    /// Create a simple equals condition
    pub fn equals(field: impl Into<FieldPath>, value: impl Into<serde_json::Value>) -> Self {
        Condition::Simple {
            field: field.into(),
            op: ConditionOp::Equals,
            value: Some(value.into()),
            ignore_case: false,
        }
    }

    /// Create an "in" condition
    pub fn is_in(field: impl Into<FieldPath>, values: Vec<serde_json::Value>) -> Self {
        Condition::Simple {
            field: field.into(),
            op: ConditionOp::In,
            value: Some(serde_json::Value::Array(values)),
            ignore_case: false,
        }
    }

    /// Create an "all" (AND) condition
    pub fn all(conditions: Vec<Condition>) -> Self {
        Condition::All { all: conditions }
    }

    /// Create an "any" (OR) condition
    pub fn any(conditions: Vec<Condition>) -> Self {
        Condition::Any { any: conditions }
    }

    /// Create a "not" condition
    pub fn negate(condition: Condition) -> Self {
        Condition::Not {
            not: Box::new(condition),
        }
    }

    /// Evaluate this condition against an event
    pub fn evaluate(&self, event: &OispEvent) -> bool {
        match self {
            Condition::Simple {
                field,
                op,
                value,
                ignore_case,
            } => evaluate_simple(field, op, value.as_ref(), *ignore_case, event),
            Condition::All { all } => all.iter().all(|c| c.evaluate(event)),
            Condition::Any { any } => any.iter().any(|c| c.evaluate(event)),
            Condition::Not { not } => !not.evaluate(event),
        }
    }
}

/// Evaluate a simple condition
fn evaluate_simple(
    field: &FieldPath,
    op: &ConditionOp,
    expected: Option<&serde_json::Value>,
    ignore_case: bool,
    event: &OispEvent,
) -> bool {
    let actual = field.extract(event);

    match op {
        ConditionOp::Exists => {
            actual.is_some() && !actual.as_ref().map(|v| v.is_null()).unwrap_or(true)
        }
        ConditionOp::NotExists => {
            actual.is_none() || actual.as_ref().map(|v| v.is_null()).unwrap_or(false)
        }
        _ => {
            let actual = match actual {
                Some(v) => v,
                None => return false,
            };
            let expected = match expected {
                Some(v) => v,
                None => return false,
            };
            evaluate_comparison(&actual, op, expected, ignore_case)
        }
    }
}

/// Evaluate a comparison between actual and expected values
fn evaluate_comparison(
    actual: &FieldValue,
    op: &ConditionOp,
    expected: &serde_json::Value,
    ignore_case: bool,
) -> bool {
    match op {
        ConditionOp::Equals => {
            let expected_fv = FieldValue::from_json(expected);
            if ignore_case {
                match (actual.as_str(), expected_fv.as_str()) {
                    (Some(a), Some(e)) => a.to_lowercase() == e.to_lowercase(),
                    _ => *actual == expected_fv,
                }
            } else {
                *actual == expected_fv
            }
        }
        ConditionOp::NotEquals => {
            let expected_fv = FieldValue::from_json(expected);
            if ignore_case {
                match (actual.as_str(), expected_fv.as_str()) {
                    (Some(a), Some(e)) => a.to_lowercase() != e.to_lowercase(),
                    _ => *actual != expected_fv,
                }
            } else {
                *actual != expected_fv
            }
        }
        ConditionOp::Contains => match (actual.as_str(), expected.as_str()) {
            (Some(a), Some(e)) => {
                if ignore_case {
                    a.to_lowercase().contains(&e.to_lowercase())
                } else {
                    a.contains(e)
                }
            }
            _ => false,
        },
        ConditionOp::NotContains => {
            match (actual.as_str(), expected.as_str()) {
                (Some(a), Some(e)) => {
                    if ignore_case {
                        !a.to_lowercase().contains(&e.to_lowercase())
                    } else {
                        !a.contains(e)
                    }
                }
                _ => true, // If we can't compare, consider it "not contained"
            }
        }
        ConditionOp::StartsWith => match (actual.as_str(), expected.as_str()) {
            (Some(a), Some(e)) => {
                if ignore_case {
                    a.to_lowercase().starts_with(&e.to_lowercase())
                } else {
                    a.starts_with(e)
                }
            }
            _ => false,
        },
        ConditionOp::EndsWith => match (actual.as_str(), expected.as_str()) {
            (Some(a), Some(e)) => {
                if ignore_case {
                    a.to_lowercase().ends_with(&e.to_lowercase())
                } else {
                    a.ends_with(e)
                }
            }
            _ => false,
        },
        ConditionOp::Matches => match (actual.as_str(), expected.as_str()) {
            (Some(a), Some(pattern)) => {
                let regex_result = if ignore_case {
                    Regex::new(&format!("(?i){}", pattern))
                } else {
                    Regex::new(pattern)
                };
                match regex_result {
                    Ok(regex) => regex.is_match(a),
                    Err(_) => false,
                }
            }
            _ => false,
        },
        ConditionOp::In => {
            let list = match expected.as_array() {
                Some(l) => l,
                None => return false,
            };
            let set: HashSet<String> = list
                .iter()
                .filter_map(|v| {
                    v.as_str().map(|s| {
                        if ignore_case {
                            s.to_lowercase()
                        } else {
                            s.to_string()
                        }
                    })
                })
                .collect();

            match actual {
                FieldValue::String(s) => {
                    let s = if ignore_case {
                        s.to_lowercase()
                    } else {
                        s.clone()
                    };
                    set.contains(&s)
                }
                FieldValue::Number(n) => list.iter().any(|v| {
                    v.as_f64()
                        .map(|vn| (vn - n).abs() < f64::EPSILON)
                        .unwrap_or(false)
                }),
                _ => false,
            }
        }
        ConditionOp::NotIn => {
            let list = match expected.as_array() {
                Some(l) => l,
                None => return true, // If not a list, consider it "not in"
            };
            let set: HashSet<String> = list
                .iter()
                .filter_map(|v| {
                    v.as_str().map(|s| {
                        if ignore_case {
                            s.to_lowercase()
                        } else {
                            s.to_string()
                        }
                    })
                })
                .collect();

            match actual {
                FieldValue::String(s) => {
                    let s = if ignore_case {
                        s.to_lowercase()
                    } else {
                        s.clone()
                    };
                    !set.contains(&s)
                }
                FieldValue::Number(n) => !list.iter().any(|v| {
                    v.as_f64()
                        .map(|vn| (vn - n).abs() < f64::EPSILON)
                        .unwrap_or(false)
                }),
                _ => true,
            }
        }
        ConditionOp::Gt => match (actual.as_number(), expected.as_f64()) {
            (Some(a), Some(e)) => a > e,
            _ => false,
        },
        ConditionOp::Gte => match (actual.as_number(), expected.as_f64()) {
            (Some(a), Some(e)) => a >= e,
            _ => false,
        },
        ConditionOp::Lt => match (actual.as_number(), expected.as_f64()) {
            (Some(a), Some(e)) => a < e,
            _ => false,
        },
        ConditionOp::Lte => match (actual.as_number(), expected.as_f64()) {
            (Some(a), Some(e)) => a <= e,
            _ => false,
        },
        ConditionOp::Exists | ConditionOp::NotExists => {
            // Already handled above
            unreachable!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{
        AiRequestData, AiRequestEvent, AppInfo, AppTier, EventEnvelope, ModelInfo, ProcessInfo,
        ProviderInfo,
    };

    fn create_test_event() -> OispEvent {
        let mut envelope = EventEnvelope::new("ai.request");
        envelope.app = Some(AppInfo {
            app_id: Some("cursor".to_string()),
            name: Some("Cursor".to_string()),
            tier: AppTier::Identified,
            vendor: None,
            version: None,
            bundle_id: None,
            category: None,
            is_ai_app: None,
            is_ai_host: None,
        });
        envelope.process = Some(ProcessInfo {
            pid: 1234,
            ppid: None,
            exe: Some("/Applications/Cursor.app/Contents/MacOS/Cursor".to_string()),
            name: Some("cursor".to_string()),
            cmdline: None,
            cwd: None,
            tid: None,
            container_id: None,
            hash: None,
            bundle_id: None,
            code_signature: None,
        });

        OispEvent::AiRequest(AiRequestEvent {
            envelope,
            data: AiRequestData {
                request_id: "req-123".to_string(),
                provider: Some(ProviderInfo {
                    name: "openai".to_string(),
                    endpoint: Some("https://api.openai.com/v1/chat/completions".to_string()),
                    region: None,
                    organization_id: None,
                    project_id: None,
                }),
                model: Some(ModelInfo {
                    id: "gpt-4".to_string(),
                    name: Some("GPT-4".to_string()),
                    family: None,
                    version: None,
                    capabilities: None,
                    context_window: None,
                    max_output_tokens: None,
                }),
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

    #[test]
    fn test_field_extraction() {
        let event = create_test_event();

        let path = FieldPath::new("event_type");
        let value = path.extract(&event);
        assert_eq!(value, Some(FieldValue::String("ai.request".to_string())));

        let path = FieldPath::new("app.tier");
        let value = path.extract(&event);
        assert_eq!(value, Some(FieldValue::String("identified".to_string())));

        let path = FieldPath::new("data.provider.name");
        let value = path.extract(&event);
        assert_eq!(value, Some(FieldValue::String("openai".to_string())));

        let path = FieldPath::new("process.pid");
        let value = path.extract(&event);
        assert_eq!(value, Some(FieldValue::Number(1234.0)));
    }

    #[test]
    fn test_equals_condition() {
        let event = create_test_event();

        let condition = Condition::equals("event_type", "ai.request");
        assert!(condition.evaluate(&event));

        let condition = Condition::equals("event_type", "ai.response");
        assert!(!condition.evaluate(&event));
    }

    #[test]
    fn test_in_condition() {
        let event = create_test_event();

        let condition = Condition::is_in(
            "data.provider.name",
            vec!["openai".into(), "anthropic".into()],
        );
        assert!(condition.evaluate(&event));

        let condition = Condition::is_in(
            "data.provider.name",
            vec!["anthropic".into(), "google".into()],
        );
        assert!(!condition.evaluate(&event));
    }

    #[test]
    fn test_contains_condition() {
        let event = create_test_event();

        let condition = Condition::Simple {
            field: FieldPath::new("process.exe"),
            op: ConditionOp::Contains,
            value: Some("Cursor".into()),
            ignore_case: false,
        };
        assert!(condition.evaluate(&event));
    }

    #[test]
    fn test_all_condition() {
        let event = create_test_event();

        let condition = Condition::all(vec![
            Condition::equals("event_type", "ai.request"),
            Condition::equals("data.provider.name", "openai"),
        ]);
        assert!(condition.evaluate(&event));

        let condition = Condition::all(vec![
            Condition::equals("event_type", "ai.request"),
            Condition::equals("data.provider.name", "anthropic"),
        ]);
        assert!(!condition.evaluate(&event));
    }

    #[test]
    fn test_any_condition() {
        let event = create_test_event();

        let condition = Condition::any(vec![
            Condition::equals("data.provider.name", "anthropic"),
            Condition::equals("data.provider.name", "openai"),
        ]);
        assert!(condition.evaluate(&event));
    }

    #[test]
    fn test_not_condition() {
        let event = create_test_event();

        let condition = Condition::negate(Condition::equals("data.provider.name", "anthropic"));
        assert!(condition.evaluate(&event));
    }

    #[test]
    fn test_exists_condition() {
        let event = create_test_event();

        let condition = Condition::Simple {
            field: FieldPath::new("app.app_id"),
            op: ConditionOp::Exists,
            value: None,
            ignore_case: false,
        };
        assert!(condition.evaluate(&event));

        let condition = Condition::Simple {
            field: FieldPath::new("app.nonexistent"),
            op: ConditionOp::NotExists,
            value: None,
            ignore_case: false,
        };
        assert!(condition.evaluate(&event));
    }
}
