//! Policy synchronization with Oximy Cloud
//!
//! Fetches and applies policies from the cloud to the local sensor.

use crate::client::CloudClient;
use crate::error::{OximyError, OximyResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Policy document from cloud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDocument {
    /// Policy version identifier
    pub version: String,

    /// When the policy was last updated
    pub updated_at: DateTime<Utc>,

    /// The actual policies
    pub policies: Vec<CloudPolicy>,

    /// Default action when no policy matches
    pub default_action: Option<String>,
}

/// Individual policy from cloud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudPolicy {
    /// Policy ID
    pub id: String,

    /// Policy name
    pub name: String,

    /// Policy description
    pub description: Option<String>,

    /// Whether policy is enabled
    pub enabled: bool,

    /// Policy priority (higher = evaluated first)
    pub priority: i32,

    /// Conditions for matching
    pub conditions: Vec<PolicyCondition>,

    /// Actions to take when matched
    pub actions: Vec<PolicyAction>,
}

/// Policy condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCondition {
    /// Field to match (e.g., "provider", "model", "actor.app.name")
    pub field: String,

    /// Operator (eq, ne, contains, matches, in, not_in)
    pub operator: String,

    /// Value to compare
    pub value: serde_json::Value,
}

/// Policy action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyAction {
    /// Action type (allow, block, redact, alert)
    #[serde(rename = "type")]
    pub action_type: String,

    /// Action parameters
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Policy synchronization service
pub struct PolicySync {
    client: Arc<CloudClient>,
    current_policy: Arc<RwLock<Option<PolicyDocument>>>,
    last_sync: Arc<RwLock<Option<DateTime<Utc>>>>,
    sync_interval: Duration,
}

impl PolicySync {
    /// Create new policy sync service
    pub fn new(client: Arc<CloudClient>) -> Self {
        Self {
            client,
            current_policy: Arc::new(RwLock::new(None)),
            last_sync: Arc::new(RwLock::new(None)),
            sync_interval: Duration::from_secs(300), // 5 minutes default
        }
    }

    /// Create with custom sync interval
    pub fn with_interval(client: Arc<CloudClient>, interval: Duration) -> Self {
        Self {
            client,
            current_policy: Arc::new(RwLock::new(None)),
            last_sync: Arc::new(RwLock::new(None)),
            sync_interval: interval,
        }
    }

    /// Fetch policies from cloud
    pub async fn fetch_policies(&self) -> OximyResult<PolicyDocument> {
        let (device_id, token) = self.client.ensure_authenticated().await?;

        debug!("Fetching policies from cloud");

        let url = format!(
            "{}/v1/devices/{}/policies",
            self.client.config().api_endpoint,
            device_id
        );

        let response = reqwest::Client::new()
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(OximyError::server(status, message));
        }

        let policy_doc: PolicyDocument = response.json().await?;

        info!(
            "Fetched policy version {} with {} policies",
            policy_doc.version,
            policy_doc.policies.len()
        );

        Ok(policy_doc)
    }

    /// Sync policies - fetch and update local copy
    pub async fn sync(&self) -> OximyResult<bool> {
        let policy_doc = self.fetch_policies().await?;

        let updated = {
            let current = self.current_policy.read().await;
            current
                .as_ref()
                .map(|p| p.version != policy_doc.version)
                .unwrap_or(true)
        };

        if updated {
            let version = policy_doc.version.clone();
            let count = policy_doc.policies.len();

            {
                let mut current = self.current_policy.write().await;
                *current = Some(policy_doc);
            }

            {
                let mut last = self.last_sync.write().await;
                *last = Some(Utc::now());
            }

            info!("Policy updated to version {} ({} policies)", version, count);
        } else {
            debug!("Policy unchanged");

            let mut last = self.last_sync.write().await;
            *last = Some(Utc::now());
        }

        Ok(updated)
    }

    /// Get current policy version
    pub async fn current_version(&self) -> Option<String> {
        let policy = self.current_policy.read().await;
        policy.as_ref().map(|p| p.version.clone())
    }

    /// Get current policies
    pub async fn current_policies(&self) -> Option<PolicyDocument> {
        let policy = self.current_policy.read().await;
        policy.clone()
    }

    /// Check if sync is needed based on interval
    pub async fn needs_sync(&self) -> bool {
        let last = self.last_sync.read().await;
        match *last {
            Some(t) => {
                let elapsed = Utc::now().signed_duration_since(t);
                elapsed.num_seconds() as u64 >= self.sync_interval.as_secs()
            }
            None => true,
        }
    }

    /// Start background sync task
    pub fn start_background_sync(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("Starting policy sync background task");

            loop {
                // Wait for sync interval
                tokio::time::sleep(self.sync_interval).await;

                // Check if we have credentials
                if !self.client.has_valid_credentials().await {
                    debug!("Skipping policy sync - not enrolled");
                    continue;
                }

                // Sync policies
                match self.sync().await {
                    Ok(updated) => {
                        if updated {
                            info!("Policy sync completed - policies updated");
                        } else {
                            debug!("Policy sync completed - no changes");
                        }
                    }
                    Err(e) => {
                        warn!("Policy sync failed: {}", e);
                    }
                }
            }
        })
    }

    /// Convert cloud policies to local policy format
    ///
    /// This converts the cloud policy format to the format expected by
    /// the oisp-core PolicyEngine.
    pub fn to_local_policies(&self, doc: &PolicyDocument) -> Vec<LocalPolicy> {
        doc.policies
            .iter()
            .filter(|p| p.enabled)
            .map(|p| LocalPolicy {
                name: p.name.clone(),
                description: p.description.clone(),
                priority: p.priority,
                conditions: p.conditions.clone(),
                actions: p.actions.clone(),
            })
            .collect()
    }
}

/// Local policy format (simplified for oisp-core integration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPolicy {
    pub name: String,
    pub description: Option<String>,
    pub priority: i32,
    pub conditions: Vec<PolicyCondition>,
    pub actions: Vec<PolicyAction>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_document_deserialize() {
        let json = r#"{
            "version": "pol_v1",
            "updated_at": "2024-01-01T00:00:00Z",
            "policies": [
                {
                    "id": "pol_123",
                    "name": "Block sensitive models",
                    "description": "Block GPT-4 for non-prod",
                    "enabled": true,
                    "priority": 100,
                    "conditions": [
                        {"field": "model", "operator": "eq", "value": "gpt-4"}
                    ],
                    "actions": [
                        {"type": "block", "params": {"reason": "Not allowed"}}
                    ]
                }
            ],
            "default_action": "allow"
        }"#;

        let doc: PolicyDocument = serde_json::from_str(json).unwrap();
        assert_eq!(doc.version, "pol_v1");
        assert_eq!(doc.policies.len(), 1);
        assert_eq!(doc.policies[0].name, "Block sensitive models");
        assert!(doc.policies[0].enabled);
    }

    #[test]
    fn test_policy_condition() {
        let json = r#"{"field": "provider", "operator": "in", "value": ["openai", "anthropic"]}"#;
        let cond: PolicyCondition = serde_json::from_str(json).unwrap();

        assert_eq!(cond.field, "provider");
        assert_eq!(cond.operator, "in");
    }
}
