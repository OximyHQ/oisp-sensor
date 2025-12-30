//! Cloud client module
//!
//! Central client that manages connections to Oximy Cloud.

mod http;

pub use http::HttpClient;

use crate::config::OximyConfig;
use crate::error::{OximyError, OximyResult};
use crate::types::Credentials;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cloud client for Oximy platform
///
/// Manages HTTP and WebSocket connections to the Oximy cloud.
pub struct CloudClient {
    config: OximyConfig,
    http: HttpClient,
    credentials: Arc<RwLock<Option<Credentials>>>,
}

impl CloudClient {
    /// Create a new cloud client
    pub fn new(config: OximyConfig) -> Self {
        let http = HttpClient::new(&config.api_endpoint, config.connect_timeout());

        Self {
            config,
            http,
            credentials: Arc::new(RwLock::new(None)),
        }
    }

    /// Get HTTP client reference
    pub fn http(&self) -> &HttpClient {
        &self.http
    }

    /// Get configuration reference
    pub fn config(&self) -> &OximyConfig {
        &self.config
    }

    /// Set credentials
    pub async fn set_credentials(&self, creds: Credentials) {
        let mut guard = self.credentials.write().await;
        *guard = Some(creds);
    }

    /// Get current credentials
    pub async fn credentials(&self) -> Option<Credentials> {
        let guard = self.credentials.read().await;
        guard.clone()
    }

    /// Check if we have credentials
    pub async fn has_credentials(&self) -> bool {
        let guard = self.credentials.read().await;
        guard.is_some()
    }

    /// Check if credentials are valid (not expired)
    pub async fn has_valid_credentials(&self) -> bool {
        let guard = self.credentials.read().await;
        guard.as_ref().map(|c| !c.is_expired()).unwrap_or(false)
    }

    /// Get device ID if enrolled
    pub async fn device_id(&self) -> Option<String> {
        let guard = self.credentials.read().await;
        guard.as_ref().map(|c| c.device_id.clone())
    }

    /// Get device token if enrolled
    pub async fn device_token(&self) -> Option<String> {
        let guard = self.credentials.read().await;
        guard.as_ref().map(|c| c.device_token.clone())
    }

    /// Clear credentials
    pub async fn clear_credentials(&self) {
        let mut guard = self.credentials.write().await;
        *guard = None;
    }

    /// Authenticated HTTP request helper - ensures we have valid credentials
    pub async fn ensure_authenticated(&self) -> OximyResult<(String, String)> {
        let guard = self.credentials.read().await;
        match guard.as_ref() {
            Some(creds) if !creds.is_expired() => {
                Ok((creds.device_id.clone(), creds.device_token.clone()))
            }
            Some(_) => Err(OximyError::TokenExpired),
            None => Err(OximyError::NotEnrolled),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_cloud_client_new() {
        let config = OximyConfig::default();
        let client = CloudClient::new(config);

        assert!(!client.has_credentials().await);
        assert!(!client.has_valid_credentials().await);
        assert!(client.device_id().await.is_none());
    }

    #[tokio::test]
    async fn test_set_credentials() {
        let config = OximyConfig::default();
        let client = CloudClient::new(config);

        let creds = Credentials {
            device_id: "dev_123".to_string(),
            device_token: "tok_xxx".to_string(),
            token_expires_at: Utc::now() + chrono::Duration::hours(24),
            organization_id: "org_123".to_string(),
            workspace_id: None,
            api_endpoint: "https://api.oximy.com".to_string(),
            stream_endpoint: "wss://stream.oximy.com".to_string(),
            created_at: Utc::now(),
        };

        client.set_credentials(creds).await;

        assert!(client.has_credentials().await);
        assert!(client.has_valid_credentials().await);
        assert_eq!(client.device_id().await, Some("dev_123".to_string()));
    }
}
