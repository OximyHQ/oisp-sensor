//! HTTP client for Oximy REST API
//!
//! Handles all REST API calls to api.oximy.com

use crate::error::{OximyError, OximyResult};
use crate::types::{
    ApiError, DeviceInfo, HeartbeatRequest, HeartbeatResponse, RegistrationResponse, SensorStats,
    SensorStatus,
};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use tracing::{debug, error, warn};

/// HTTP client for Oximy API
pub struct HttpClient {
    client: Client,
    base_url: String,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new(base_url: &str, timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(format!("oisp-sensor/{}", env!("CARGO_PKG_VERSION")))
            .gzip(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Register device with API key
    pub async fn register_device(
        &self,
        api_key: &str,
        info: DeviceInfo,
    ) -> OximyResult<RegistrationResponse> {
        let url = format!("{}/v1/devices/register", self.base_url);

        debug!("Registering device with API key");

        let response = self
            .client
            .post(&url)
            .header("X-API-Key", api_key)
            .json(&info)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Enroll device with enrollment token (MDM flow)
    pub async fn enroll_device(
        &self,
        token: &str,
        info: DeviceInfo,
    ) -> OximyResult<RegistrationResponse> {
        let url = format!("{}/v1/devices/enroll", self.base_url);

        debug!("Enrolling device with enrollment token");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&info)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Send heartbeat
    pub async fn heartbeat(
        &self,
        device_id: &str,
        token: &str,
        status: SensorStatus,
        stats: SensorStats,
    ) -> OximyResult<HeartbeatResponse> {
        let url = format!("{}/v1/devices/{}/heartbeat", self.base_url, device_id);

        let request = HeartbeatRequest { status, stats };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Rotate device token
    pub async fn rotate_token(
        &self,
        device_id: &str,
        token: &str,
    ) -> OximyResult<RegistrationResponse> {
        let url = format!("{}/v1/devices/{}/rotate-token", self.base_url, device_id);

        debug!("Rotating device token");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Send event batch (fallback when WebSocket unavailable)
    pub async fn send_events(
        &self,
        device_id: &str,
        token: &str,
        events: &[oisp_core::OispEvent],
    ) -> OximyResult<BatchResponse> {
        let url = format!("{}/v1/events/batch", self.base_url);

        let request = BatchRequest {
            device_id: device_id.to_string(),
            events: events.to_vec(),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Generic response handler
    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> OximyResult<T> {
        let status = response.status();

        match status {
            StatusCode::OK | StatusCode::CREATED => {
                let body = response.json::<T>().await?;
                Ok(body)
            }
            StatusCode::UNAUTHORIZED => {
                let error = self.parse_error(response).await;
                error!("Authentication failed: {}", error.message);
                Err(OximyError::Auth(error.message))
            }
            StatusCode::FORBIDDEN => {
                let error = self.parse_error(response).await;
                if error.code == "invalid_api_key" {
                    Err(OximyError::InvalidApiKey)
                } else if error.code == "token_expired" {
                    Err(OximyError::TokenExpired)
                } else {
                    Err(OximyError::Auth(error.message))
                }
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);
                warn!("Rate limited, retry after {}s", retry_after);
                Err(OximyError::RateLimited(retry_after))
            }
            StatusCode::NOT_FOUND => {
                let error = self.parse_error(response).await;
                Err(OximyError::server(status.as_u16(), error.message))
            }
            _ if status.is_server_error() => {
                let error = self.parse_error(response).await;
                error!("Server error {}: {}", status, error.message);
                Err(OximyError::server(status.as_u16(), error.message))
            }
            _ => {
                let error = self.parse_error(response).await;
                Err(OximyError::server(status.as_u16(), error.message))
            }
        }
    }

    async fn parse_error(&self, response: reqwest::Response) -> ApiError {
        response
            .json::<ApiError>()
            .await
            .unwrap_or_else(|_| ApiError {
                code: "unknown".to_string(),
                message: "Unknown error".to_string(),
                details: None,
            })
    }
}

/// Batch request payload
#[derive(Debug, serde::Serialize)]
struct BatchRequest {
    device_id: String,
    events: Vec<oisp_core::OispEvent>,
}

/// Batch response
#[derive(Debug, serde::Deserialize)]
pub struct BatchResponse {
    /// Number of events received
    pub received: usize,

    /// Batch ID for tracking
    pub batch_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_new() {
        let client = HttpClient::new("https://api.oximy.com", Duration::from_secs(10));
        assert_eq!(client.base_url, "https://api.oximy.com");
    }

    #[test]
    fn test_trailing_slash_removed() {
        let client = HttpClient::new("https://api.oximy.com/", Duration::from_secs(10));
        assert_eq!(client.base_url, "https://api.oximy.com");
    }
}
