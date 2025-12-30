//! Credential storage implementations
//!
//! Provides secure storage for device credentials.

use crate::error::{OximyError, OximyResult};
use crate::types::Credentials;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, warn};

/// Trait for credential storage backends
pub trait CredentialStore: Send + Sync {
    /// Save credentials
    fn save(&self, credentials: &Credentials) -> OximyResult<()>;

    /// Load credentials
    fn load(&self) -> OximyResult<Option<Credentials>>;

    /// Delete credentials
    fn delete(&self) -> OximyResult<()>;

    /// Check if credentials exist
    fn exists(&self) -> bool {
        self.load().map(|c| c.is_some()).unwrap_or(false)
    }
}

/// File-based credential storage
///
/// Stores credentials as JSON in a file. This is a simple implementation
/// for development/testing. Production should use OS keychain.
pub struct FileCredentialStore {
    path: PathBuf,
}

impl FileCredentialStore {
    /// Create with custom path
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Get default credential path
    fn default_path() -> PathBuf {
        // Try standard locations
        if let Some(config_dir) = dirs::config_dir() {
            return config_dir.join("oisp-sensor").join("credentials.json");
        }

        // Fallback to home directory
        if let Some(home) = dirs::home_dir() {
            return home.join(".oisp-sensor").join("credentials.json");
        }

        // Last resort
        PathBuf::from("/var/lib/oisp-sensor/credentials.json")
    }

    /// Ensure parent directory exists
    fn ensure_dir(&self) -> OximyResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

impl Default for FileCredentialStore {
    fn default() -> Self {
        Self::new(Self::default_path())
    }
}

impl CredentialStore for FileCredentialStore {
    fn save(&self, credentials: &Credentials) -> OximyResult<()> {
        self.ensure_dir()?;

        let json = serde_json::to_string_pretty(credentials)?;

        // Write atomically using temp file
        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, &json)?;
        fs::rename(&temp_path, &self.path)?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&self.path, perms)?;
        }

        debug!("Saved credentials to {:?}", self.path);
        Ok(())
    }

    fn load(&self) -> OximyResult<Option<Credentials>> {
        if !self.path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&self.path)?;
        let credentials: Credentials = serde_json::from_str(&json)?;

        debug!("Loaded credentials from {:?}", self.path);
        Ok(Some(credentials))
    }

    fn delete(&self) -> OximyResult<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
            debug!("Deleted credentials from {:?}", self.path);
        }
        Ok(())
    }
}

/// OS Keychain credential storage (cross-platform)
///
/// Uses the `keyring` crate for platform-specific secure storage:
/// - macOS: Keychain
/// - Linux: Secret Service (libsecret)
/// - Windows: Credential Manager
#[allow(dead_code)]
pub struct KeychainStore {
    service: String,
    user: String,
}

#[allow(dead_code)]
impl KeychainStore {
    /// Create with default service/user
    pub fn new() -> Self {
        Self {
            service: "oisp-sensor".to_string(),
            user: "device-credentials".to_string(),
        }
    }

    /// Create with custom service name
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            user: "device-credentials".to_string(),
        }
    }
}

impl Default for KeychainStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore for KeychainStore {
    fn save(&self, credentials: &Credentials) -> OximyResult<()> {
        let entry = keyring::Entry::new(&self.service, &self.user)
            .map_err(|e| OximyError::CredentialStore(e.to_string()))?;

        let json = serde_json::to_string(credentials)?;

        entry
            .set_password(&json)
            .map_err(|e| OximyError::CredentialStore(e.to_string()))?;

        debug!("Saved credentials to keychain");
        Ok(())
    }

    fn load(&self) -> OximyResult<Option<Credentials>> {
        let entry = keyring::Entry::new(&self.service, &self.user)
            .map_err(|e| OximyError::CredentialStore(e.to_string()))?;

        match entry.get_password() {
            Ok(json) => {
                let credentials: Credentials = serde_json::from_str(&json)?;
                debug!("Loaded credentials from keychain");
                Ok(Some(credentials))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => {
                warn!("Failed to load from keychain: {}", e);
                Err(OximyError::CredentialStore(e.to_string()))
            }
        }
    }

    fn delete(&self) -> OximyResult<()> {
        let entry = keyring::Entry::new(&self.service, &self.user)
            .map_err(|e| OximyError::CredentialStore(e.to_string()))?;

        match entry.delete_credential() {
            Ok(_) => {
                debug!("Deleted credentials from keychain");
                Ok(())
            }
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(OximyError::CredentialStore(e.to_string())),
        }
    }
}

/// In-memory credential store (for testing)
#[cfg(test)]
pub struct MemoryCredentialStore {
    credentials: std::sync::Mutex<Option<Credentials>>,
}

#[cfg(test)]
impl MemoryCredentialStore {
    pub fn new() -> Self {
        Self {
            credentials: std::sync::Mutex::new(None),
        }
    }
}

#[cfg(test)]
impl CredentialStore for MemoryCredentialStore {
    fn save(&self, credentials: &Credentials) -> OximyResult<()> {
        let mut guard = self.credentials.lock().unwrap();
        *guard = Some(credentials.clone());
        Ok(())
    }

    fn load(&self) -> OximyResult<Option<Credentials>> {
        let guard = self.credentials.lock().unwrap();
        Ok(guard.clone())
    }

    fn delete(&self) -> OximyResult<()> {
        let mut guard = self.credentials.lock().unwrap();
        *guard = None;
        Ok(())
    }
}

// Use dirs crate for cross-platform paths
mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            home_dir().map(|h| h.join("Library").join("Application Support"))
        }

        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_CONFIG_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| home_dir().map(|h| h.join(".config")))
        }

        #[cfg(target_os = "windows")]
        {
            std::env::var("APPDATA").ok().map(PathBuf::from)
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()
            .map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn test_credentials() -> Credentials {
        Credentials {
            device_id: "dev_test123".to_string(),
            device_token: "tok_secret".to_string(),
            token_expires_at: Utc::now() + chrono::Duration::hours(24),
            organization_id: "org_test".to_string(),
            workspace_id: Some("ws_test".to_string()),
            api_endpoint: "https://api.oximy.com".to_string(),
            stream_endpoint: "wss://stream.oximy.com".to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_file_store_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("creds.json");
        let store = FileCredentialStore::new(path);

        let creds = test_credentials();

        // Save
        store.save(&creds).unwrap();
        assert!(store.exists());

        // Load
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.device_id, creds.device_id);
        assert_eq!(loaded.device_token, creds.device_token);

        // Delete
        store.delete().unwrap();
        assert!(!store.exists());
    }

    #[test]
    fn test_file_store_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.json");
        let store = FileCredentialStore::new(path);

        let result = store.load().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_memory_store() {
        let store = MemoryCredentialStore::new();
        let creds = test_credentials();

        store.save(&creds).unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.device_id, creds.device_id);

        store.delete().unwrap();
        assert!(store.load().unwrap().is_none());
    }
}
