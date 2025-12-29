//! Policy manager - file loading, hot-reload, and policy lifecycle management
//!
//! Features:
//! - Load policies from YAML files
//! - Hot-reload on file changes (using notify)
//! - Default policy generation
//! - Policy validation

use super::evaluator::PolicyEvaluator;
use super::parser::{example_policy_file, parse_policies, parse_policies_file, Policy};
use super::{DefaultAction, PolicyConfig};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Policy manager errors
#[derive(Error, Debug)]
pub enum PolicyManagerError {
    #[error("Failed to load policy file: {0}")]
    LoadError(String),

    #[error("Policy validation failed: {0}")]
    ValidationError(String),

    #[error("File watch error: {0}")]
    WatchError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Configuration for the policy manager
#[derive(Debug, Clone)]
pub struct PolicyManagerConfig {
    /// Path to the policy file
    pub policy_file: PathBuf,
    /// Enable hot-reload
    pub hot_reload: bool,
    /// Default action when no policy matches
    pub default_action: DefaultAction,
    /// Webhook URL for alerts
    pub webhook_url: Option<String>,
    /// Debounce duration for file changes
    pub debounce_duration: Duration,
    /// Create default policies if file doesn't exist
    pub create_default: bool,
}

impl Default for PolicyManagerConfig {
    fn default() -> Self {
        Self {
            policy_file: super::default_policy_path(),
            hot_reload: true,
            default_action: DefaultAction::Allow,
            webhook_url: None,
            debounce_duration: Duration::from_millis(500),
            create_default: true,
        }
    }
}

impl From<PolicyConfig> for PolicyManagerConfig {
    fn from(config: PolicyConfig) -> Self {
        Self {
            policy_file: config.policy_file,
            hot_reload: config.hot_reload,
            default_action: config.default_action,
            webhook_url: config.alert_webhook_url,
            debounce_duration: Duration::from_millis(500),
            create_default: true,
        }
    }
}

/// Policy manager - manages policy loading and hot-reload
pub struct PolicyManager {
    /// Configuration
    config: PolicyManagerConfig,
    /// Policy evaluator
    evaluator: Arc<PolicyEvaluator>,
    /// File watcher (if hot-reload enabled)
    watcher: Option<RecommendedWatcher>,
    /// Channel for reload signals
    reload_tx: Option<mpsc::Sender<()>>,
    /// Last loaded file content hash (for change detection)
    last_hash: Arc<RwLock<Option<u64>>>,
}

impl PolicyManager {
    /// Create a new policy manager
    pub async fn new(config: PolicyManagerConfig) -> Result<Self, PolicyManagerError> {
        // Load initial policies
        let policies = Self::load_policies_from_file(&config).await?;

        info!(
            path = %config.policy_file.display(),
            count = policies.len(),
            "Loaded policies"
        );

        let evaluator = Arc::new(PolicyEvaluator::new(
            policies,
            config.default_action,
            config.webhook_url.clone(),
        ));

        let mut manager = Self {
            config,
            evaluator,
            watcher: None,
            reload_tx: None,
            last_hash: Arc::new(RwLock::new(None)),
        };

        // Start hot-reload if enabled
        if manager.config.hot_reload {
            manager.start_hot_reload().await?;
        }

        Ok(manager)
    }

    /// Create with custom config
    pub async fn with_config(config: PolicyConfig) -> Result<Self, PolicyManagerError> {
        Self::new(PolicyManagerConfig::from(config)).await
    }

    /// Load policies from file
    async fn load_policies_from_file(
        config: &PolicyManagerConfig,
    ) -> Result<Vec<Policy>, PolicyManagerError> {
        let path = &config.policy_file;

        // Check if file exists
        if !path.exists() {
            if config.create_default {
                info!(
                    path = %path.display(),
                    "Policy file not found, creating default"
                );
                Self::create_default_policy_file(path)?;
            } else {
                warn!(
                    path = %path.display(),
                    "Policy file not found, using empty policy set"
                );
                return Ok(vec![]);
            }
        }

        // Parse the file
        let file = parse_policies_file(path).map_err(|e| {
            PolicyManagerError::LoadError(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        // Filter to enabled policies
        let enabled_policies: Vec<Policy> =
            file.policies.into_iter().filter(|p| p.enabled).collect();

        Ok(enabled_policies)
    }

    /// Create a default policy file
    fn create_default_policy_file(path: &Path) -> Result<(), PolicyManagerError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Generate example policies
        let example = example_policy_file();
        let yaml = serde_yaml::to_string(&example).map_err(|e| {
            PolicyManagerError::LoadError(format!("Failed to serialize default policies: {}", e))
        })?;

        // Write with a header comment
        let content = format!(
            "# OISP Policy Configuration\n\
             # See: https://github.com/oximy/oisp-sensor/blob/main/docs/phases/PHASE_3_POLICY_ENGINE.md\n\
             #\n\
             # This file was auto-generated. Customize it for your needs.\n\
             # Policies are evaluated in priority order (higher first).\n\
             #\n\
             {yaml}"
        );

        std::fs::write(path, content)?;

        info!(path = %path.display(), "Created default policy file");
        Ok(())
    }

    /// Start hot-reload file watching
    async fn start_hot_reload(&mut self) -> Result<(), PolicyManagerError> {
        let path = self.config.policy_file.clone();
        let evaluator = self.evaluator.clone();
        let debounce = self.config.debounce_duration;
        let _default_action = self.config.default_action;
        let _webhook_url = self.config.webhook_url.clone();
        let last_hash = self.last_hash.clone();

        // Compute initial hash
        if let Ok(content) = std::fs::read_to_string(&path) {
            *last_hash.write().await = Some(hash_string(&content));
        }

        // Create channel for reload events
        let (tx, mut rx) = mpsc::channel::<()>(10);
        self.reload_tx = Some(tx.clone());

        // Create file watcher
        let tx_clone = tx.clone();
        let path_clone = path.clone();

        let watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                match result {
                    Ok(event) => {
                        // Check if our file was modified
                        if event.paths.iter().any(|p| p == &path_clone) {
                            debug!(path = %path_clone.display(), "Policy file changed");
                            let _ = tx_clone.blocking_send(());
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "File watcher error");
                    }
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(1)),
        )
        .map_err(|e| PolicyManagerError::WatchError(e.to_string()))?;

        // Watch the parent directory (to catch file creation)
        let watch_path = path.parent().unwrap_or(Path::new("."));

        // Store watcher (we'll add watch after)
        self.watcher = Some(watcher);

        // Add watch
        if let Some(ref mut w) = self.watcher {
            w.watch(watch_path, RecursiveMode::NonRecursive)
                .map_err(|e| PolicyManagerError::WatchError(e.to_string()))?;
        }

        // Spawn reload handler
        let path_for_task = path.clone();
        tokio::spawn(async move {
            let mut last_reload = std::time::Instant::now();

            while let Some(()) = rx.recv().await {
                // Debounce
                let elapsed = last_reload.elapsed();
                if elapsed < debounce {
                    tokio::time::sleep(debounce - elapsed).await;
                }
                last_reload = std::time::Instant::now();

                // Check if file actually changed (hash comparison)
                let content = match std::fs::read_to_string(&path_for_task) {
                    Ok(c) => c,
                    Err(e) => {
                        warn!(error = %e, "Failed to read policy file");
                        continue;
                    }
                };

                let new_hash = hash_string(&content);
                let old_hash = *last_hash.read().await;

                if old_hash == Some(new_hash) {
                    debug!("Policy file unchanged (same hash)");
                    continue;
                }

                // Parse new policies
                match parse_policies(&content) {
                    Ok(file) => {
                        let enabled: Vec<Policy> =
                            file.policies.into_iter().filter(|p| p.enabled).collect();

                        info!(
                            path = %path_for_task.display(),
                            count = enabled.len(),
                            "Reloading policies"
                        );

                        evaluator.update_policies(enabled).await;
                        *last_hash.write().await = Some(new_hash);
                    }
                    Err(e) => {
                        error!(
                            path = %path_for_task.display(),
                            error = %e,
                            "Failed to parse updated policies, keeping old policies"
                        );
                    }
                }
            }
        });

        info!(
            path = %path.display(),
            "Hot-reload enabled for policies"
        );

        Ok(())
    }

    /// Force reload policies from file
    pub async fn reload(&self) -> Result<(), PolicyManagerError> {
        let config = PolicyManagerConfig {
            policy_file: self.config.policy_file.clone(),
            ..self.config.clone()
        };

        let policies = Self::load_policies_from_file(&config).await?;
        self.evaluator.update_policies(policies).await;

        // Update hash
        if let Ok(content) = std::fs::read_to_string(&self.config.policy_file) {
            *self.last_hash.write().await = Some(hash_string(&content));
        }

        Ok(())
    }

    /// Get the policy evaluator
    pub fn evaluator(&self) -> Arc<PolicyEvaluator> {
        self.evaluator.clone()
    }

    /// Get current policy count
    pub async fn policy_count(&self) -> usize {
        self.evaluator.policy_count().await
    }

    /// Get all policies
    pub async fn policies(&self) -> Vec<Policy> {
        self.evaluator.policies().await
    }

    /// Get the policy file path
    pub fn policy_file(&self) -> &Path {
        &self.config.policy_file
    }

    /// Check if hot-reload is enabled
    pub fn hot_reload_enabled(&self) -> bool {
        self.config.hot_reload && self.watcher.is_some()
    }

    /// Stop the policy manager (stops file watching)
    pub async fn stop(&mut self) {
        // Drop watcher
        self.watcher = None;

        // Close reload channel
        self.reload_tx = None;

        info!("Policy manager stopped");
    }
}

/// Simple string hash for change detection
fn hash_string(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_policy_manager_creates_default() {
        let dir = tempdir().unwrap();
        let policy_file = dir.path().join("policies.yaml");

        let config = PolicyManagerConfig {
            policy_file: policy_file.clone(),
            hot_reload: false,
            create_default: true,
            ..Default::default()
        };

        let manager = PolicyManager::new(config).await.unwrap();

        // File should be created
        assert!(policy_file.exists());

        // Should have default policies
        assert!(manager.policy_count().await > 0);
    }

    #[tokio::test]
    async fn test_policy_manager_loads_custom() {
        let dir = tempdir().unwrap();
        let policy_file = dir.path().join("policies.yaml");

        // Create a custom policy file
        let yaml = r#"
version: "1"
policies:
  - id: custom-policy
    name: Custom Policy
    enabled: true
    conditions:
      field: event_type
      op: equals
      value: ai.request
    action:
      type: allow
"#;
        std::fs::write(&policy_file, yaml).unwrap();

        let config = PolicyManagerConfig {
            policy_file: policy_file.clone(),
            hot_reload: false,
            create_default: false,
            ..Default::default()
        };

        let manager = PolicyManager::new(config).await.unwrap();
        assert_eq!(manager.policy_count().await, 1);

        let policies = manager.policies().await;
        assert_eq!(policies[0].id, "custom-policy");
    }

    #[tokio::test]
    async fn test_policy_manager_reload() {
        let dir = tempdir().unwrap();
        let policy_file = dir.path().join("policies.yaml");

        // Create initial policy file
        let yaml1 = r#"
version: "1"
policies:
  - id: policy-1
    name: Policy 1
    conditions:
      field: event_type
      op: equals
      value: ai.request
    action:
      type: allow
"#;
        std::fs::write(&policy_file, yaml1).unwrap();

        let config = PolicyManagerConfig {
            policy_file: policy_file.clone(),
            hot_reload: false,
            ..Default::default()
        };

        let manager = PolicyManager::new(config).await.unwrap();
        assert_eq!(manager.policy_count().await, 1);

        // Update the file
        let yaml2 = r#"
version: "1"
policies:
  - id: policy-1
    name: Policy 1
    conditions:
      field: event_type
      op: equals
      value: ai.request
    action:
      type: allow
  - id: policy-2
    name: Policy 2
    conditions:
      field: event_type
      op: equals
      value: ai.response
    action:
      type: allow
"#;
        std::fs::write(&policy_file, yaml2).unwrap();

        // Manual reload
        manager.reload().await.unwrap();
        assert_eq!(manager.policy_count().await, 2);
    }
}
