//! Error types for oisp-oximy crate

use thiserror::Error;

/// Errors that can occur in the Oximy cloud connector
#[derive(Debug, Error)]
pub enum OximyError {
    /// Network/HTTP error
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// Invalid API key format
    #[error("Invalid API key")]
    InvalidApiKey,

    /// Invalid enrollment token
    #[error("Invalid enrollment token")]
    InvalidEnrollmentToken,

    /// Device not enrolled
    #[error("Device not enrolled")]
    NotEnrolled,

    /// Token has expired
    #[error("Token expired")]
    TokenExpired,

    /// Rate limited by server
    #[error("Rate limited: retry after {0}s")]
    RateLimited(u64),

    /// Server error
    #[error("Server error: {status} - {message}")]
    Server { status: u16, message: String },

    /// Credential storage error
    #[error("Credential storage error: {0}")]
    CredentialStore(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// URL parse error
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// Database error (offline queue)
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Connection closed
    #[error("Connection closed")]
    ConnectionClosed,

    /// Timeout
    #[error("Operation timed out")]
    Timeout,
}

impl OximyError {
    /// Check if this is a network-related error
    pub fn is_network_error(&self) -> bool {
        matches!(
            self,
            OximyError::Network(_) | OximyError::WebSocket(_) | OximyError::ConnectionClosed
        )
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            OximyError::Network(_)
                | OximyError::WebSocket(_)
                | OximyError::RateLimited(_)
                | OximyError::Server { .. }
                | OximyError::ConnectionClosed
                | OximyError::Timeout
        )
    }

    /// Create a server error from status and message
    pub fn server(status: u16, message: impl Into<String>) -> Self {
        OximyError::Server {
            status,
            message: message.into(),
        }
    }
}

/// Result type for Oximy operations
pub type OximyResult<T> = Result<T, OximyError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_network_error() {
        assert!(OximyError::WebSocket("connection reset".to_string()).is_network_error());
        assert!(OximyError::ConnectionClosed.is_network_error());
        assert!(!OximyError::InvalidApiKey.is_network_error());
        assert!(!OximyError::NotEnrolled.is_network_error());
    }

    #[test]
    fn test_is_retryable() {
        assert!(OximyError::RateLimited(30).is_retryable());
        assert!(OximyError::server(500, "Internal error").is_retryable());
        assert!(OximyError::Timeout.is_retryable());
        assert!(!OximyError::InvalidApiKey.is_retryable());
        assert!(!OximyError::Auth("bad token".to_string()).is_retryable());
    }
}
