//! AI Provider detection and metadata

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Known AI providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    OpenAI,
    Anthropic,
    Google,
    AzureOpenAI,
    AwsBedrock,
    Cohere,
    Mistral,
    Groq,
    Together,
    Fireworks,
    Replicate,
    HuggingFace,
    Perplexity,
    DeepSeek,
    Ollama,
    LmStudio,
    Vllm,
    OpenAICompatible,
    Unknown,
}

impl Provider {
    pub fn display_name(&self) -> &'static str {
        match self {
            Provider::OpenAI => "OpenAI",
            Provider::Anthropic => "Anthropic",
            Provider::Google => "Google AI (Gemini)",
            Provider::AzureOpenAI => "Azure OpenAI",
            Provider::AwsBedrock => "AWS Bedrock",
            Provider::Cohere => "Cohere",
            Provider::Mistral => "Mistral AI",
            Provider::Groq => "Groq",
            Provider::Together => "Together AI",
            Provider::Fireworks => "Fireworks AI",
            Provider::Replicate => "Replicate",
            Provider::HuggingFace => "Hugging Face",
            Provider::Perplexity => "Perplexity",
            Provider::DeepSeek => "DeepSeek",
            Provider::Ollama => "Ollama",
            Provider::LmStudio => "LM Studio",
            Provider::Vllm => "vLLM",
            Provider::OpenAICompatible => "OpenAI Compatible",
            Provider::Unknown => "Unknown",
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Provider::Ollama | Provider::LmStudio | Provider::Vllm)
    }
}

/// Provider configuration for detection
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider: Provider,
    pub domains: Vec<String>,
    pub domain_patterns: Vec<String>,
    pub api_key_prefixes: Vec<String>,
    pub auth_header: Option<String>,
}

/// Registry of provider configurations
pub struct ProviderRegistry {
    providers: Vec<ProviderConfig>,
    domain_lookup: HashMap<String, Provider>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            providers: Vec::new(),
            domain_lookup: HashMap::new(),
        };
        registry.load_defaults();
        registry
    }

    fn load_defaults(&mut self) {
        // OpenAI
        self.register(ProviderConfig {
            provider: Provider::OpenAI,
            domains: vec!["api.openai.com".into()],
            domain_patterns: vec![],
            api_key_prefixes: vec!["sk-".into(), "sk-proj-".into(), "sk-svcacct-".into()],
            auth_header: Some("Authorization".into()),
        });

        // Anthropic
        self.register(ProviderConfig {
            provider: Provider::Anthropic,
            domains: vec!["api.anthropic.com".into()],
            domain_patterns: vec![],
            api_key_prefixes: vec!["sk-ant-".into()],
            auth_header: Some("x-api-key".into()),
        });

        // Google
        self.register(ProviderConfig {
            provider: Provider::Google,
            domains: vec![
                "generativelanguage.googleapis.com".into(),
                "aiplatform.googleapis.com".into(),
            ],
            domain_patterns: vec![],
            api_key_prefixes: vec![],
            auth_header: None,
        });

        // Azure OpenAI
        self.register(ProviderConfig {
            provider: Provider::AzureOpenAI,
            domains: vec![],
            domain_patterns: vec!["*.openai.azure.com".into()],
            api_key_prefixes: vec![],
            auth_header: Some("api-key".into()),
        });

        // AWS Bedrock
        self.register(ProviderConfig {
            provider: Provider::AwsBedrock,
            domains: vec![],
            domain_patterns: vec![
                "bedrock-runtime.*.amazonaws.com".into(),
                "bedrock.*.amazonaws.com".into(),
            ],
            api_key_prefixes: vec![],
            auth_header: None,
        });

        // Cohere
        self.register(ProviderConfig {
            provider: Provider::Cohere,
            domains: vec!["api.cohere.ai".into(), "api.cohere.com".into()],
            domain_patterns: vec![],
            api_key_prefixes: vec![],
            auth_header: Some("Authorization".into()),
        });

        // Mistral
        self.register(ProviderConfig {
            provider: Provider::Mistral,
            domains: vec!["api.mistral.ai".into()],
            domain_patterns: vec![],
            api_key_prefixes: vec![],
            auth_header: Some("Authorization".into()),
        });

        // Groq
        self.register(ProviderConfig {
            provider: Provider::Groq,
            domains: vec!["api.groq.com".into()],
            domain_patterns: vec![],
            api_key_prefixes: vec!["gsk_".into()],
            auth_header: Some("Authorization".into()),
        });

        // Together
        self.register(ProviderConfig {
            provider: Provider::Together,
            domains: vec!["api.together.xyz".into()],
            domain_patterns: vec![],
            api_key_prefixes: vec![],
            auth_header: Some("Authorization".into()),
        });

        // Fireworks
        self.register(ProviderConfig {
            provider: Provider::Fireworks,
            domains: vec!["api.fireworks.ai".into()],
            domain_patterns: vec![],
            api_key_prefixes: vec![],
            auth_header: Some("Authorization".into()),
        });

        // Replicate
        self.register(ProviderConfig {
            provider: Provider::Replicate,
            domains: vec!["api.replicate.com".into()],
            domain_patterns: vec![],
            api_key_prefixes: vec!["r8_".into()],
            auth_header: Some("Authorization".into()),
        });

        // Hugging Face
        self.register(ProviderConfig {
            provider: Provider::HuggingFace,
            domains: vec!["api-inference.huggingface.co".into()],
            domain_patterns: vec![],
            api_key_prefixes: vec!["hf_".into()],
            auth_header: Some("Authorization".into()),
        });

        // Perplexity
        self.register(ProviderConfig {
            provider: Provider::Perplexity,
            domains: vec!["api.perplexity.ai".into()],
            domain_patterns: vec![],
            api_key_prefixes: vec!["pplx-".into()],
            auth_header: Some("Authorization".into()),
        });

        // DeepSeek
        self.register(ProviderConfig {
            provider: Provider::DeepSeek,
            domains: vec!["api.deepseek.com".into()],
            domain_patterns: vec![],
            api_key_prefixes: vec![],
            auth_header: Some("Authorization".into()),
        });

        // Ollama
        self.register(ProviderConfig {
            provider: Provider::Ollama,
            domains: vec!["localhost:11434".into(), "127.0.0.1:11434".into()],
            domain_patterns: vec!["*.local:11434".into()],
            api_key_prefixes: vec![],
            auth_header: None,
        });

        // LM Studio
        self.register(ProviderConfig {
            provider: Provider::LmStudio,
            domains: vec!["localhost:1234".into(), "127.0.0.1:1234".into()],
            domain_patterns: vec![],
            api_key_prefixes: vec![],
            auth_header: None,
        });
    }

    fn register(&mut self, config: ProviderConfig) {
        for domain in &config.domains {
            self.domain_lookup.insert(domain.clone(), config.provider);
        }
        self.providers.push(config);
    }

    /// Detect provider from domain
    pub fn detect_from_domain(&self, domain: &str) -> Option<Provider> {
        // Direct lookup
        if let Some(provider) = self.domain_lookup.get(domain) {
            return Some(*provider);
        }

        // Pattern matching
        for config in &self.providers {
            for pattern in &config.domain_patterns {
                if matches_pattern(pattern, domain) {
                    return Some(config.provider);
                }
            }
        }

        None
    }

    /// Detect provider from API key prefix
    pub fn detect_from_key_prefix(&self, key: &str) -> Option<Provider> {
        // Collect all matching prefixes with their length and provider
        let mut matches: Vec<(usize, Provider)> = Vec::new();

        for config in &self.providers {
            for prefix in &config.api_key_prefixes {
                if key.starts_with(prefix) {
                    matches.push((prefix.len(), config.provider));
                }
            }
        }

        // Return the provider with the longest matching prefix
        matches
            .into_iter()
            .max_by_key(|(len, _)| *len)
            .map(|(_, provider)| provider)
    }

    /// Get config for a provider
    pub fn get_config(&self, provider: Provider) -> Option<&ProviderConfig> {
        self.providers.iter().find(|c| c.provider == provider)
    }

    /// Check if a domain is a known AI provider
    pub fn is_ai_domain(&self, domain: &str) -> bool {
        self.detect_from_domain(domain).is_some()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple glob-style pattern matching
fn matches_pattern(pattern: &str, value: &str) -> bool {
    if pattern.starts_with("*.") {
        let suffix = &pattern[1..]; // Keep the dot
        value.ends_with(suffix)
    } else if pattern.contains('*') {
        // Handle patterns like "bedrock-runtime.*.amazonaws.com"
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            value.starts_with(parts[0]) && value.ends_with(parts[1])
        } else {
            false
        }
    } else {
        pattern == value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        assert!(matches_pattern(
            "*.openai.azure.com",
            "myinstance.openai.azure.com"
        ));
        assert!(matches_pattern(
            "bedrock-runtime.*.amazonaws.com",
            "bedrock-runtime.us-east-1.amazonaws.com"
        ));
        assert!(!matches_pattern("*.openai.azure.com", "api.openai.com"));
    }

    #[test]
    fn test_provider_detection() {
        let registry = ProviderRegistry::new();

        assert_eq!(
            registry.detect_from_domain("api.openai.com"),
            Some(Provider::OpenAI)
        );
        assert_eq!(
            registry.detect_from_domain("api.anthropic.com"),
            Some(Provider::Anthropic)
        );
        assert_eq!(
            registry.detect_from_domain("localhost:11434"),
            Some(Provider::Ollama)
        );
        assert_eq!(
            registry.detect_from_domain("myinstance.openai.azure.com"),
            Some(Provider::AzureOpenAI)
        );
    }

    #[test]
    fn test_key_prefix_detection() {
        let registry = ProviderRegistry::new();

        assert_eq!(
            registry.detect_from_key_prefix("sk-proj-abc123"),
            Some(Provider::OpenAI)
        );
        assert_eq!(
            registry.detect_from_key_prefix("sk-ant-abc123"),
            Some(Provider::Anthropic)
        );
        assert_eq!(
            registry.detect_from_key_prefix("gsk_abc123"),
            Some(Provider::Groq)
        );
    }
}
