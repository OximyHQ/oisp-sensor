//! Redaction patterns and safe defaults

use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::LazyLock;

/// Redaction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum RedactionMode {
    /// Safe mode - hash content, redact secrets
    #[default]
    Safe,
    /// Full capture - no redaction (except explicit patterns)
    Full,
    /// Minimal - only metadata, no content at all
    Minimal,
}

/// Redaction configuration
#[derive(Debug, Clone)]
pub struct RedactionConfig {
    pub mode: RedactionMode,
    pub redact_api_keys: bool,
    pub redact_emails: bool,
    pub redact_credit_cards: bool,
    pub redact_ssn: bool,
    pub redact_phone_numbers: bool,
    pub custom_patterns: Vec<String>,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            mode: RedactionMode::Safe,
            redact_api_keys: true,
            redact_emails: true,
            redact_credit_cards: true,
            redact_ssn: true,
            redact_phone_numbers: false,
            custom_patterns: Vec::new(),
        }
    }
}

/// Built-in redaction patterns
pub struct RedactionPatterns {
    pub api_keys: Vec<Regex>,
    pub emails: Regex,
    pub credit_cards: Regex,
    pub ssn: Regex,
    pub phone_numbers: Regex,
    pub jwt: Regex,
    pub aws_keys: Regex,
    pub github_tokens: Regex,
    pub slack_tokens: Regex,
}

static PATTERNS: LazyLock<RedactionPatterns> = LazyLock::new(|| {
    RedactionPatterns {
        api_keys: vec![
            // OpenAI
            Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(),
            Regex::new(r"sk-proj-[a-zA-Z0-9]{20,}").unwrap(),
            // Anthropic
            Regex::new(r"sk-ant-[a-zA-Z0-9-]{20,}").unwrap(),
            // Generic API key patterns
            Regex::new(r#"(?i)api[_-]?key['"]?\s*[:=]\s*['"]?([a-zA-Z0-9_-]{20,})['"]?"#).unwrap(),
            Regex::new(r#"(?i)secret[_-]?key['"]?\s*[:=]\s*['"]?([a-zA-Z0-9_-]{20,})['"]?"#)
                .unwrap(),
            // Bearer tokens
            Regex::new(r"Bearer\s+[a-zA-Z0-9_.=-]{20,}").unwrap(),
        ],
        emails: Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
        credit_cards: Regex::new(r"\b(?:\d{4}[- ]?){3}\d{4}\b").unwrap(),
        ssn: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
        phone_numbers: Regex::new(
            r"\b(?:\+?1[-.]?)?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}\b",
        )
        .unwrap(),
        jwt: Regex::new(r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*").unwrap(),
        aws_keys: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        github_tokens: Regex::new(r"gh[pousr]_[a-zA-Z0-9]{36,}").unwrap(),
        slack_tokens: Regex::new(r"xox[baprs]-[0-9a-zA-Z-]+").unwrap(),
    }
});

/// Redaction result
#[derive(Debug, Clone)]
pub struct RedactionResult {
    /// The redacted content
    pub content: String,
    /// What was found
    pub findings: Vec<RedactionFinding>,
    /// Original content hash
    pub hash: String,
    /// Original length
    pub original_length: usize,
}

/// A finding that was redacted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionFinding {
    /// Type of finding
    pub finding_type: String,
    /// Count of occurrences
    pub count: usize,
}

/// Redact sensitive content from a string
pub fn redact(content: &str, config: &RedactionConfig) -> RedactionResult {
    let original_length = content.len();
    let hash = hash_content(content);

    if config.mode == RedactionMode::Full {
        return RedactionResult {
            content: content.to_string(),
            findings: Vec::new(),
            hash,
            original_length,
        };
    }

    if config.mode == RedactionMode::Minimal {
        return RedactionResult {
            content: "[REDACTED]".to_string(),
            findings: vec![RedactionFinding {
                finding_type: "full_content".to_string(),
                count: 1,
            }],
            hash,
            original_length,
        };
    }

    let mut result = content.to_string();
    let mut findings = Vec::new();

    // API keys
    if config.redact_api_keys {
        for pattern in &PATTERNS.api_keys {
            let count = pattern.find_iter(&result).count();
            if count > 0 {
                result = pattern
                    .replace_all(&result, "[API_KEY_REDACTED]")
                    .to_string();
                findings.push(RedactionFinding {
                    finding_type: "api_key".to_string(),
                    count,
                });
            }
        }

        // JWT
        let count = PATTERNS.jwt.find_iter(&result).count();
        if count > 0 {
            result = PATTERNS
                .jwt
                .replace_all(&result, "[JWT_REDACTED]")
                .to_string();
            findings.push(RedactionFinding {
                finding_type: "jwt".to_string(),
                count,
            });
        }

        // AWS keys
        let count = PATTERNS.aws_keys.find_iter(&result).count();
        if count > 0 {
            result = PATTERNS
                .aws_keys
                .replace_all(&result, "[AWS_KEY_REDACTED]")
                .to_string();
            findings.push(RedactionFinding {
                finding_type: "aws_key".to_string(),
                count,
            });
        }

        // GitHub tokens
        let count = PATTERNS.github_tokens.find_iter(&result).count();
        if count > 0 {
            result = PATTERNS
                .github_tokens
                .replace_all(&result, "[GITHUB_TOKEN_REDACTED]")
                .to_string();
            findings.push(RedactionFinding {
                finding_type: "github_token".to_string(),
                count,
            });
        }

        // Slack tokens
        let count = PATTERNS.slack_tokens.find_iter(&result).count();
        if count > 0 {
            result = PATTERNS
                .slack_tokens
                .replace_all(&result, "[SLACK_TOKEN_REDACTED]")
                .to_string();
            findings.push(RedactionFinding {
                finding_type: "slack_token".to_string(),
                count,
            });
        }
    }

    // Emails
    if config.redact_emails {
        let count = PATTERNS.emails.find_iter(&result).count();
        if count > 0 {
            result = PATTERNS
                .emails
                .replace_all(&result, "[EMAIL_REDACTED]")
                .to_string();
            findings.push(RedactionFinding {
                finding_type: "email".to_string(),
                count,
            });
        }
    }

    // Credit cards
    if config.redact_credit_cards {
        let count = PATTERNS.credit_cards.find_iter(&result).count();
        if count > 0 {
            result = PATTERNS
                .credit_cards
                .replace_all(&result, "[CREDIT_CARD_REDACTED]")
                .to_string();
            findings.push(RedactionFinding {
                finding_type: "credit_card".to_string(),
                count,
            });
        }
    }

    // SSN
    if config.redact_ssn {
        let count = PATTERNS.ssn.find_iter(&result).count();
        if count > 0 {
            result = PATTERNS
                .ssn
                .replace_all(&result, "[SSN_REDACTED]")
                .to_string();
            findings.push(RedactionFinding {
                finding_type: "ssn".to_string(),
                count,
            });
        }
    }

    // Phone numbers
    if config.redact_phone_numbers {
        let count = PATTERNS.phone_numbers.find_iter(&result).count();
        if count > 0 {
            result = PATTERNS
                .phone_numbers
                .replace_all(&result, "[PHONE_REDACTED]")
                .to_string();
            findings.push(RedactionFinding {
                finding_type: "phone".to_string(),
                count,
            });
        }
    }

    // Custom patterns
    for pattern_str in &config.custom_patterns {
        if let Ok(pattern) = Regex::new(pattern_str) {
            let count = pattern.find_iter(&result).count();
            if count > 0 {
                result = pattern
                    .replace_all(&result, "[CUSTOM_REDACTED]")
                    .to_string();
                findings.push(RedactionFinding {
                    finding_type: "custom".to_string(),
                    count,
                });
            }
        }
    }

    RedactionResult {
        content: result,
        findings,
        hash,
        original_length,
    }
}

/// Hash content for correlation
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

/// Extract API key prefix safely
pub fn extract_key_prefix(key: &str, max_len: usize) -> String {
    if key.len() <= max_len {
        return key.to_string();
    }

    // Find common prefix patterns
    let prefixes = ["sk-proj-", "sk-ant-", "sk-", "gsk_", "hf_", "r8_", "pplx-"];
    for prefix in prefixes {
        if key.starts_with(prefix) {
            return format!("{}...", prefix);
        }
    }

    // Default: first N chars
    format!("{}...", &key[..max_len.min(key.len())])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_redaction() {
        let config = RedactionConfig::default();
        let content = "My API key is sk-proj-abc123def456ghi789jkl012";
        let result = redact(content, &config);

        assert!(result.content.contains("[API_KEY_REDACTED]"));
        assert!(!result.content.contains("sk-proj-"));
        assert!(!result.findings.is_empty());
    }

    #[test]
    fn test_email_redaction() {
        let config = RedactionConfig::default();
        let content = "Contact me at user@example.com for help";
        let result = redact(content, &config);

        assert!(result.content.contains("[EMAIL_REDACTED]"));
        assert!(!result.content.contains("user@example.com"));
    }

    #[test]
    fn test_full_mode() {
        let config = RedactionConfig {
            mode: RedactionMode::Full,
            ..Default::default()
        };
        let content = "My API key is sk-proj-abc123def456ghi789jkl012";
        let result = redact(content, &config);

        assert_eq!(result.content, content);
    }

    #[test]
    fn test_minimal_mode() {
        let config = RedactionConfig {
            mode: RedactionMode::Minimal,
            ..Default::default()
        };
        let content = "Some content here";
        let result = redact(content, &config);

        assert_eq!(result.content, "[REDACTED]");
    }
}
