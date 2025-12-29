//! OISP Spec Bundle Loader
//!
//! Loads provider/model configuration dynamically from the OISP spec bundle.
//! This enables updating providers and models without recompiling the sensor.
//!
//! Loading strategy:
//! 1. Try cached bundle from disk
//! 2. Fetch latest from remote (if network enabled)
//! 3. Fall back to embedded bundle (compile-time)

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

/// Default URL for fetching the spec bundle
pub const DEFAULT_BUNDLE_URL: &str = "https://oisp.dev/spec/v0.1/bundle.json";

/// How often to check for bundle updates (1 hour)
pub const BUNDLE_REFRESH_INTERVAL: Duration = Duration::from_secs(3600);

/// Get the bundle URL from environment variable or use default
/// Supports OISP_BUNDLE_URL environment variable for custom bundle locations
pub fn bundle_url() -> String {
    std::env::var("OISP_BUNDLE_URL").unwrap_or_else(|_| DEFAULT_BUNDLE_URL.to_string())
}

/// Get the bundle refresh interval from environment variable or use default
/// Supports OISP_BUNDLE_REFRESH_SECS environment variable (in seconds)
pub fn bundle_refresh_interval() -> Duration {
    std::env::var("OISP_BUNDLE_REFRESH_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(BUNDLE_REFRESH_INTERVAL)
}

/// Embedded spec bundle (compile-time fallback)
/// This is updated when the sensor is built
const EMBEDDED_BUNDLE: &str = include_str!("../data/oisp-spec-bundle.json");

/// The complete OISP spec bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OispSpecBundle {
    /// Schema URL
    #[serde(rename = "$schema", default)]
    pub schema: String,

    /// Spec version
    pub version: String,

    /// Bundle format version
    pub bundle_version: String,

    /// When the bundle was generated
    pub generated_at: String,

    /// Source of the bundle
    pub source: String,

    /// Provider metadata
    pub providers: HashMap<String, ProviderSpec>,

    /// Quick domain -> provider lookup
    pub domain_index: HashMap<String, String>,

    /// Domain patterns for wildcard matching
    pub domain_patterns: Vec<DomainPattern>,

    /// Extraction rules for parsing requests/responses
    pub extraction_rules: HashMap<String, ExtractionRuleSet>,

    /// Fingerprinting rules
    #[serde(default)]
    pub fingerprints: HashMap<String, serde_json::Value>,

    /// Model registry (provider/model -> info)
    #[serde(default)]
    pub models: HashMap<String, ModelSpec>,

    /// Model statistics
    #[serde(default)]
    pub model_stats: ModelStats,
}

/// Provider specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSpec {
    /// Provider ID
    pub id: String,

    /// Human-readable name
    pub display_name: String,

    /// Provider type: cloud, local, self_hosted
    #[serde(rename = "type", default)]
    pub provider_type: String,

    /// Known domains
    #[serde(default)]
    pub domains: Vec<String>,

    /// Supported features
    #[serde(default)]
    pub features: Vec<String>,

    /// Authentication info
    #[serde(default)]
    pub auth: AuthSpec,

    /// LiteLLM provider name (for model lookup)
    #[serde(default)]
    pub litellm_provider: Option<String>,
}

/// Authentication specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthSpec {
    /// Auth type: api_key, oauth, etc.
    #[serde(rename = "type", default)]
    pub auth_type: Option<String>,

    /// Header name for API key
    #[serde(default)]
    pub header: Option<String>,

    /// Prefix for auth header value (e.g., "Bearer ")
    #[serde(default)]
    pub prefix: Option<String>,

    /// Known key prefixes (e.g., "sk-", "sk-proj-")
    #[serde(default)]
    pub key_prefixes: Vec<String>,
}

/// Domain pattern for wildcard matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainPattern {
    /// Glob pattern
    pub pattern: String,

    /// Provider ID
    pub provider: String,

    /// Compiled regex
    pub regex: String,
}

/// Set of extraction rules for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionRuleSet {
    /// Authentication rules
    #[serde(default)]
    pub auth: serde_json::Value,

    /// Endpoint-specific rules
    #[serde(default)]
    pub endpoints: HashMap<String, EndpointRules>,

    /// Response headers to extract
    #[serde(default)]
    pub response_headers: HashMap<String, String>,

    /// Model family patterns
    #[serde(default)]
    pub model_families: HashMap<String, serde_json::Value>,

    /// Provider-specific features
    #[serde(default)]
    pub features: HashMap<String, serde_json::Value>,
}

/// Extraction rules for a specific endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRules {
    /// URL path pattern
    pub path: String,

    /// HTTP method
    pub method: String,

    /// Request type: chat, completion, embedding, etc.
    pub request_type: String,

    /// Streaming indicator
    #[serde(default)]
    pub streaming: StreamingIndicator,

    /// Request field extraction (JSONPath -> field name)
    #[serde(default)]
    pub request_extraction: HashMap<String, serde_json::Value>,

    /// Response field extraction
    #[serde(default)]
    pub response_extraction: HashMap<String, serde_json::Value>,
}

/// Streaming indicator configuration
///
/// Can be deserialized from:
/// - `true` / `false` (boolean) - simple streaming flag
/// - `{}` (empty object) - default struct
/// - `{ "content_type": "...", "indicator": {...} }` - full struct
#[derive(Debug, Clone, Default, Serialize)]
pub struct StreamingIndicator {
    /// Content type for streaming responses
    #[serde(default)]
    pub content_type: Option<String>,

    /// How to detect streaming from request
    #[serde(default)]
    pub indicator: Option<StreamingCheck>,
}

impl<'de> Deserialize<'de> for StreamingIndicator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct StreamingIndicatorVisitor;

        impl<'de> Visitor<'de> for StreamingIndicatorVisitor {
            type Value = StreamingIndicator;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a boolean or streaming indicator object")
            }

            fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // Boolean true/false just means "streaming supported" - return default struct
                Ok(StreamingIndicator::default())
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut content_type = None;
                let mut indicator = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "content_type" => {
                            content_type = Some(map.next_value()?);
                        }
                        "indicator" => {
                            indicator = Some(map.next_value()?);
                        }
                        _ => {
                            // Skip unknown fields
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                Ok(StreamingIndicator {
                    content_type,
                    indicator,
                })
            }
        }

        deserializer.deserialize_any(StreamingIndicatorVisitor)
    }
}

/// How to check if a request is streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingCheck {
    /// JSON field to check
    pub body_field: String,

    /// Expected value
    pub value: serde_json::Value,
}

/// Model specification from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    /// Model ID
    pub id: String,

    /// LiteLLM model ID (may differ from id)
    #[serde(default)]
    pub litellm_id: Option<String>,

    /// Provider
    pub provider: String,

    /// Mode: chat, completion, embedding, etc.
    #[serde(default)]
    pub mode: Option<String>,

    /// Max input tokens (can be float in JSON, converted to u64)
    #[serde(default, deserialize_with = "deserialize_float_as_u64")]
    pub max_input_tokens: Option<u64>,

    /// Max output tokens (can be float in JSON, converted to u64)
    #[serde(default, deserialize_with = "deserialize_float_as_u64")]
    pub max_output_tokens: Option<u64>,

    /// Input cost per 1K tokens
    #[serde(default)]
    pub input_cost_per_1k: Option<f64>,

    /// Output cost per 1K tokens
    #[serde(default)]
    pub output_cost_per_1k: Option<f64>,

    /// Model capabilities
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Whether model is deprecated
    #[serde(default)]
    pub deprecated: bool,
}

/// Deserialize a number that might be a float (e.g., 2000000.0) as u64
fn deserialize_float_as_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct FloatOrU64Visitor;

    impl<'de> Visitor<'de> for FloatOrU64Visitor {
        type Value = Option<u64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a number (integer or float)")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as u64))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as u64))
        }
    }

    deserializer.deserialize_any(FloatOrU64Visitor)
}

/// Model registry statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelStats {
    pub total_models: usize,
    pub providers: usize,
}

impl OispSpecBundle {
    /// Load the embedded bundle (always available)
    pub fn embedded() -> Self {
        serde_json::from_str(EMBEDDED_BUNDLE).expect("embedded spec bundle should be valid JSON")
    }

    /// Load from a file path
    pub fn from_file(path: &Path) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save to a file path
    pub fn to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, content)
    }

    /// Load with fallback strategy: cache -> embedded
    /// (Network fetching is async and handled separately)
    pub fn load_with_fallback(cache_path: Option<&Path>) -> Self {
        // Try cached file first
        if let Some(path) = cache_path {
            if path.exists() {
                match Self::from_file(path) {
                    Ok(bundle) => {
                        info!("Loaded spec bundle from cache: {}", path.display());
                        return bundle;
                    }
                    Err(e) => {
                        warn!("Failed to load cached bundle: {}", e);
                    }
                }
            }
        }

        // Fall back to embedded
        info!("Using embedded spec bundle");
        Self::embedded()
    }

    /// Check if the bundle should be refreshed
    pub fn needs_refresh(cache_path: &Path) -> bool {
        if !cache_path.exists() {
            return true;
        }

        match std::fs::metadata(cache_path) {
            Ok(meta) => match meta.modified() {
                Ok(modified) => {
                    let age = SystemTime::now()
                        .duration_since(modified)
                        .unwrap_or(Duration::MAX);
                    age > BUNDLE_REFRESH_INTERVAL
                }
                Err(_) => true,
            },
            Err(_) => true,
        }
    }

    /// Get provider by ID
    pub fn get_provider(&self, id: &str) -> Option<&ProviderSpec> {
        self.providers.get(id)
    }

    /// Get model by key (provider/model_id)
    pub fn get_model(&self, provider: &str, model_id: &str) -> Option<&ModelSpec> {
        let key = format!("{}/{}", provider, model_id);
        self.models.get(&key)
    }

    /// Get extraction rules for a provider
    pub fn get_extraction_rules(&self, provider: &str) -> Option<&ExtractionRuleSet> {
        self.extraction_rules.get(provider)
    }
}

/// Spec bundle loader with caching and refresh
pub struct SpecLoader {
    /// Current bundle
    bundle: Arc<OispSpecBundle>,

    /// Cache file path
    cache_path: PathBuf,

    /// URL to fetch bundle from
    #[allow(dead_code)]
    bundle_url: String,

    /// Whether network fetching is enabled
    network_enabled: bool,
}

impl SpecLoader {
    /// Create a new loader with default settings
    /// Uses OISP_BUNDLE_URL environment variable if set, otherwise default URL
    pub fn new() -> Self {
        let cache_path = Self::default_cache_path();
        let url = bundle_url();
        let bundle = OispSpecBundle::load_with_fallback(Some(&cache_path));

        info!("SpecLoader initialized with bundle URL: {}", url);

        Self {
            bundle: Arc::new(bundle),
            cache_path,
            bundle_url: url,
            network_enabled: true,
        }
    }

    /// Create with custom settings
    pub fn with_config(cache_path: PathBuf, bundle_url: String, network_enabled: bool) -> Self {
        let bundle = OispSpecBundle::load_with_fallback(Some(&cache_path));

        Self {
            bundle: Arc::new(bundle),
            cache_path,
            bundle_url,
            network_enabled,
        }
    }

    /// Get the current bundle
    pub fn bundle(&self) -> Arc<OispSpecBundle> {
        Arc::clone(&self.bundle)
    }

    /// Get default cache path
    pub fn default_cache_path() -> PathBuf {
        // Try XDG cache dir, fall back to /tmp
        if let Ok(cache_dir) = std::env::var("XDG_CACHE_HOME") {
            PathBuf::from(cache_dir).join("oisp/spec-bundle.json")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".cache/oisp/spec-bundle.json")
        } else {
            PathBuf::from("/tmp/oisp-spec-bundle.json")
        }
    }

    /// Check if refresh is needed
    pub fn needs_refresh(&self) -> bool {
        self.network_enabled && OispSpecBundle::needs_refresh(&self.cache_path)
    }

    /// Refresh the bundle (sync, for use in blocking contexts)
    /// Returns true if bundle was updated
    pub fn refresh_sync(&mut self) -> bool {
        if !self.needs_refresh() {
            return false;
        }

        // In sync mode, just try to reload from cache
        // (actual network fetch should be done async)
        if self.cache_path.exists() {
            if let Ok(bundle) = OispSpecBundle::from_file(&self.cache_path) {
                self.bundle = Arc::new(bundle);
                return true;
            }
        }

        false
    }
}

impl Default for SpecLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Dynamic provider registry using the spec bundle
pub struct DynamicProviderRegistry {
    /// The spec bundle
    bundle: Arc<OispSpecBundle>,

    /// Compiled domain patterns
    compiled_patterns: Vec<(Regex, String)>,
}

impl DynamicProviderRegistry {
    /// Create from a spec bundle
    pub fn new(bundle: Arc<OispSpecBundle>) -> Self {
        let mut compiled_patterns = Vec::new();

        for pattern in &bundle.domain_patterns {
            if let Ok(re) = Regex::new(&pattern.regex) {
                compiled_patterns.push((re, pattern.provider.clone()));
            }
        }

        Self {
            bundle,
            compiled_patterns,
        }
    }

    /// Detect provider from domain
    pub fn detect_from_domain(&self, domain: &str) -> Option<&str> {
        // Exact match first (fast path)
        if let Some(provider) = self.bundle.domain_index.get(domain) {
            return Some(provider);
        }

        // Pattern matching
        for (re, provider) in &self.compiled_patterns {
            if re.is_match(domain) {
                return Some(provider);
            }
        }

        None
    }

    /// Get provider spec
    pub fn get_provider(&self, id: &str) -> Option<&ProviderSpec> {
        self.bundle.providers.get(id)
    }

    /// Detect provider from API key prefix
    pub fn detect_from_key_prefix(&self, key: &str) -> Option<&str> {
        // Find the longest matching prefix
        let mut best_match: Option<(&str, usize)> = None;

        for (provider_id, provider) in &self.bundle.providers {
            for prefix in &provider.auth.key_prefixes {
                if key.starts_with(prefix) {
                    let len = prefix.len();
                    if best_match.map(|(_, l)| len > l).unwrap_or(true) {
                        best_match = Some((provider_id.as_str(), len));
                    }
                }
            }
        }

        best_match.map(|(id, _)| id)
    }

    /// Check if domain is a known AI provider
    pub fn is_ai_domain(&self, domain: &str) -> bool {
        self.detect_from_domain(domain).is_some()
    }

    /// Get extraction rules for a provider
    pub fn get_extraction_rules(&self, provider_id: &str) -> Option<&ExtractionRuleSet> {
        self.bundle.extraction_rules.get(provider_id)
    }

    /// Get model info
    pub fn get_model(&self, provider: &str, model_id: &str) -> Option<&ModelSpec> {
        self.bundle.get_model(provider, model_id)
    }

    /// Estimate cost for a request
    pub fn estimate_cost(
        &self,
        provider: &str,
        model_id: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) -> Option<(f64, f64, f64)> {
        let model = self.get_model(provider, model_id)?;

        let input_cost = model.input_cost_per_1k? * (input_tokens as f64 / 1000.0);
        let output_cost = model.output_cost_per_1k? * (output_tokens as f64 / 1000.0);
        let total_cost = input_cost + output_cost;

        Some((input_cost, output_cost, total_cost))
    }

    /// Get all known provider IDs
    pub fn provider_ids(&self) -> Vec<&str> {
        self.bundle.providers.keys().map(|s| s.as_str()).collect()
    }

    /// Get display name for a provider
    pub fn display_name<'a>(&'a self, provider_id: &'a str) -> &'a str {
        self.bundle
            .providers
            .get(provider_id)
            .map(|p| p.display_name.as_str())
            .unwrap_or(provider_id)
    }

    /// Check if provider is local (Ollama, LM Studio, etc.)
    pub fn is_local(&self, provider_id: &str) -> bool {
        self.bundle
            .providers
            .get(provider_id)
            .map(|p| p.provider_type == "local")
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_bundle() -> OispSpecBundle {
        OispSpecBundle::embedded()
    }

    #[test]
    fn test_load_embedded() {
        let bundle = OispSpecBundle::embedded();
        assert!(!bundle.providers.is_empty());
        assert!(!bundle.domain_index.is_empty());
    }

    #[test]
    fn test_domain_detection() {
        let bundle = Arc::new(test_bundle());
        let registry = DynamicProviderRegistry::new(bundle);

        // Exact matches
        assert_eq!(
            registry.detect_from_domain("api.openai.com"),
            Some("openai")
        );
        assert_eq!(
            registry.detect_from_domain("api.anthropic.com"),
            Some("anthropic")
        );
        assert_eq!(
            registry.detect_from_domain("localhost:11434"),
            Some("ollama")
        );

        // Pattern matches (Azure)
        assert_eq!(
            registry.detect_from_domain("myinstance.openai.azure.com"),
            Some("azure_openai")
        );

        // Unknown
        assert_eq!(registry.detect_from_domain("example.com"), None);
    }

    #[test]
    fn test_key_prefix_detection() {
        let bundle = Arc::new(test_bundle());
        let registry = DynamicProviderRegistry::new(bundle);

        assert_eq!(
            registry.detect_from_key_prefix("sk-proj-abc123"),
            Some("openai")
        );
        assert_eq!(
            registry.detect_from_key_prefix("sk-ant-abc123"),
            Some("anthropic")
        );
    }

    #[test]
    fn test_extraction_rules() {
        let bundle = Arc::new(test_bundle());
        let registry = DynamicProviderRegistry::new(bundle);

        let rules = registry.get_extraction_rules("openai");
        assert!(rules.is_some());

        let rules = rules.unwrap();
        assert!(rules.endpoints.contains_key("chat_completions"));
    }
}
