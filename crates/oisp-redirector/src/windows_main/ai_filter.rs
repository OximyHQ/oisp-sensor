//! AI Endpoint Filtering
//!
//! This module loads the OISP spec bundle and filters connections
//! to only intercept traffic to known AI API endpoints.

use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Embedded spec bundle (compiled into binary)
const EMBEDDED_SPEC: &str = include_str!("../../../oisp-core/data/oisp-spec-bundle.json");

/// OISP Spec Bundle structure
#[derive(Debug, Deserialize)]
pub struct SpecBundle {
    /// Direct domain to provider mapping
    #[serde(default)]
    pub domain_index: HashMap<String, String>,

    /// Regex patterns for domain matching
    #[serde(default)]
    pub domain_patterns: Vec<DomainPattern>,
}

/// Domain pattern for regex matching
#[derive(Debug, Deserialize)]
pub struct DomainPattern {
    /// Human-readable pattern (e.g., "*.openai.azure.com")
    pub pattern: String,

    /// Provider name
    pub provider: String,

    /// Regex pattern for matching
    pub regex: String,
}

/// Compiled domain pattern
pub struct CompiledPattern {
    pub provider: String,
    pub regex: Regex,
}

/// AI Endpoint Filter
pub struct AiEndpointFilter {
    /// Direct domain lookup
    domain_index: HashMap<String, String>,

    /// Compiled regex patterns
    patterns: Vec<CompiledPattern>,

    /// Number of domains in index
    domain_count: usize,

    /// Number of patterns
    pattern_count: usize,
}

impl AiEndpointFilter {
    /// Create a new filter from embedded spec
    pub fn new() -> Result<Self> {
        Self::from_json(EMBEDDED_SPEC)
    }

    /// Create a filter from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let spec: SpecBundle =
            serde_json::from_str(json).context("Failed to parse OISP spec bundle")?;

        let domain_count = spec.domain_index.len();

        // Compile regex patterns
        let mut patterns = Vec::new();
        for pattern in spec.domain_patterns {
            match Regex::new(&pattern.regex) {
                Ok(regex) => {
                    patterns.push(CompiledPattern {
                        provider: pattern.provider,
                        regex,
                    });
                }
                Err(e) => {
                    warn!("Failed to compile regex '{}': {}", pattern.regex, e);
                }
            }
        }

        let pattern_count = patterns.len();

        info!(
            "AI filter initialized: {} domains, {} patterns",
            domain_count, pattern_count
        );

        Ok(Self {
            domain_index: spec.domain_index,
            patterns,
            domain_count,
            pattern_count,
        })
    }

    /// Check if a hostname is an AI endpoint
    ///
    /// Returns Some(provider) if the hostname matches an AI endpoint,
    /// None otherwise.
    pub fn is_ai_endpoint(&self, hostname: &str) -> Option<&str> {
        // First, try direct lookup
        if let Some(provider) = self.domain_index.get(hostname) {
            debug!("AI endpoint match (direct): {} -> {}", hostname, provider);
            return Some(provider);
        }

        // Then try regex patterns
        for pattern in &self.patterns {
            if pattern.regex.is_match(hostname) {
                debug!(
                    "AI endpoint match (pattern): {} -> {}",
                    hostname, pattern.provider
                );
                return Some(&pattern.provider);
            }
        }

        None
    }

    /// Check if a socket address is an AI endpoint
    ///
    /// This checks both "host:port" and just "host" formats.
    pub fn is_ai_endpoint_addr(&self, addr: &SocketAddr, hostname: Option<&str>) -> Option<&str> {
        // If we have a hostname, prefer that
        if let Some(host) = hostname {
            if let Some(provider) = self.is_ai_endpoint(host) {
                return Some(provider);
            }
            // Also try with port
            let host_port = format!("{}:{}", host, addr.port());
            if let Some(provider) = self.is_ai_endpoint(&host_port) {
                return Some(provider);
            }
        }

        // Try IP address with port (for localhost endpoints like Ollama)
        let ip_port = format!("{}:{}", addr.ip(), addr.port());
        if let Some(provider) = self.is_ai_endpoint(&ip_port) {
            return Some(provider);
        }

        None
    }

    /// Get statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.domain_count, self.pattern_count)
    }

    /// Get all known providers
    pub fn providers(&self) -> Vec<&str> {
        let mut providers: Vec<&str> = self.domain_index.values().map(|s| s.as_str()).collect();
        for pattern in &self.patterns {
            if !providers.contains(&pattern.provider.as_str()) {
                providers.push(&pattern.provider);
            }
        }
        providers.sort();
        providers.dedup();
        providers
    }
}

impl Default for AiEndpointFilter {
    fn default() -> Self {
        Self::new().expect("Failed to load embedded AI endpoint spec")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_lookup() {
        let filter = AiEndpointFilter::new().unwrap();

        // Known AI endpoints
        assert_eq!(filter.is_ai_endpoint("api.openai.com"), Some("openai"));
        assert_eq!(
            filter.is_ai_endpoint("api.anthropic.com"),
            Some("anthropic")
        );
        assert_eq!(filter.is_ai_endpoint("api.groq.com"), Some("groq"));

        // Non-AI endpoints
        assert_eq!(filter.is_ai_endpoint("google.com"), None);
        assert_eq!(filter.is_ai_endpoint("example.com"), None);
    }

    #[test]
    fn test_pattern_matching() {
        let filter = AiEndpointFilter::new().unwrap();

        // Azure OpenAI (wildcard pattern)
        assert!(filter
            .is_ai_endpoint("my-resource.openai.azure.com")
            .is_some());
        assert!(filter.is_ai_endpoint("another.openai.azure.com").is_some());

        // AWS Bedrock (wildcard pattern)
        assert!(filter
            .is_ai_endpoint("bedrock-runtime.us-east-1.amazonaws.com")
            .is_some());
    }

    #[test]
    fn test_localhost_endpoints() {
        let filter = AiEndpointFilter::new().unwrap();

        // Ollama
        assert_eq!(filter.is_ai_endpoint("localhost:11434"), Some("ollama"));
        assert_eq!(filter.is_ai_endpoint("127.0.0.1:11434"), Some("ollama"));

        // LM Studio
        assert_eq!(filter.is_ai_endpoint("localhost:1234"), Some("lmstudio"));
    }

    #[test]
    fn test_stats() {
        let filter = AiEndpointFilter::new().unwrap();
        let (domains, patterns) = filter.stats();
        assert!(domains > 0, "Should have domain entries");
        assert!(patterns > 0, "Should have pattern entries");
    }
}
