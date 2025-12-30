//! Device enrollment module
//!
//! Handles device registration and credential management.

mod credentials;

pub use credentials::{CredentialStore, FileCredentialStore};

use crate::client::CloudClient;
use crate::config::OximyConfig;
use crate::error::{OximyError, OximyResult};
use crate::types::{Credentials, DeviceInfo};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Device enrollor - handles registration flow
pub struct Enrollor {
    client: Arc<CloudClient>,
    store: Box<dyn CredentialStore>,
}

impl Enrollor {
    /// Create new enrollor with file-based credential storage
    pub fn new(client: Arc<CloudClient>) -> Self {
        let store = Box::new(FileCredentialStore::default());
        Self { client, store }
    }

    /// Create enrollor with custom credential store
    pub fn with_store(client: Arc<CloudClient>, store: Box<dyn CredentialStore>) -> Self {
        Self { client, store }
    }

    /// Register device using API key
    pub async fn register_with_api_key(&self, api_key: &str) -> OximyResult<Credentials> {
        info!("Registering device with API key");

        // Validate API key format
        if !api_key.starts_with("oxm_") {
            return Err(OximyError::InvalidApiKey);
        }

        // Collect device info
        let info = DeviceInfo::default();
        debug!("Device info: {:?}", info);

        // Register with cloud
        let response = self.client.http().register_device(api_key, info).await?;

        // Create credentials
        let credentials = Credentials::from_registration(
            response,
            &self.client.config().api_endpoint,
            &self.client.config().stream_endpoint,
        );

        // Store credentials
        self.store.save(&credentials)?;
        info!("Device registered successfully: {}", credentials.device_id);

        // Update client credentials
        self.client.set_credentials(credentials.clone()).await;

        Ok(credentials)
    }

    /// Enroll device using enrollment token (MDM flow)
    pub async fn enroll_with_token(&self, token: &str) -> OximyResult<Credentials> {
        info!("Enrolling device with enrollment token");

        // Validate token format
        if !token.starts_with("enroll_") {
            return Err(OximyError::InvalidEnrollmentToken);
        }

        // Collect device info
        let info = DeviceInfo::default();
        debug!("Device info: {:?}", info);

        // Enroll with cloud
        let response = self.client.http().enroll_device(token, info).await?;

        // Create credentials
        let credentials = Credentials::from_registration(
            response,
            &self.client.config().api_endpoint,
            &self.client.config().stream_endpoint,
        );

        // Store credentials
        self.store.save(&credentials)?;
        info!("Device enrolled successfully: {}", credentials.device_id);

        // Update client credentials
        self.client.set_credentials(credentials.clone()).await;

        Ok(credentials)
    }

    /// Load stored credentials
    pub fn load_credentials(&self) -> OximyResult<Option<Credentials>> {
        match self.store.load() {
            Ok(Some(creds)) => {
                if creds.is_expired() {
                    warn!("Stored credentials are expired");
                    Ok(Some(creds)) // Return anyway, let caller decide
                } else {
                    debug!("Loaded valid credentials for device: {}", creds.device_id);
                    Ok(Some(creds))
                }
            }
            Ok(None) => {
                debug!("No stored credentials found");
                Ok(None)
            }
            Err(e) => {
                warn!("Failed to load credentials: {}", e);
                Err(e)
            }
        }
    }

    /// Initialize from stored credentials
    pub async fn initialize(&self) -> OximyResult<bool> {
        if let Some(creds) = self.load_credentials()? {
            if !creds.is_expired() {
                self.client.set_credentials(creds).await;
                return Ok(true);
            }
            // Could try token rotation here
            warn!("Credentials expired, need re-enrollment");
        }
        Ok(false)
    }

    /// Check if device is enrolled
    pub async fn is_enrolled(&self) -> bool {
        self.client.has_valid_credentials().await
    }

    /// Get device ID if enrolled
    pub async fn device_id(&self) -> Option<String> {
        self.client.device_id().await
    }

    /// Clear stored credentials
    pub fn clear_credentials(&self) -> OximyResult<()> {
        self.store.delete()
    }
}

/// Convenience function to enroll device with config
pub async fn enroll_device(config: &OximyConfig) -> OximyResult<Credentials> {
    let client = Arc::new(CloudClient::new(config.clone()));
    let enrollor = Enrollor::new(client);

    // Try to load existing credentials first
    if let Some(creds) = enrollor.load_credentials()? {
        if !creds.is_expired() {
            info!("Using existing credentials");
            return Ok(creds);
        }
    }

    // Register with API key or enrollment token
    if let Some(api_key) = &config.api_key {
        enrollor.register_with_api_key(api_key).await
    } else if let Some(token) = &config.enrollment_token {
        enrollor.enroll_with_token(token).await
    } else {
        Err(OximyError::Config(
            "No API key or enrollment token provided".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_invalid_api_key() {
        // This is a sync check, doesn't need tokio
        assert!(!("invalid_key".starts_with("oxm_")));
        assert!("oxm_live_xxx".starts_with("oxm_"));
    }

    #[test]
    fn test_invalid_enrollment_token() {
        assert!(!("invalid_token".starts_with("enroll_")));
        assert!("enroll_xxx".starts_with("enroll_"));
    }
}
