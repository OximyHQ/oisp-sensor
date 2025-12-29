//! App Registry - Application identification and matching
//!
//! This module provides:
//! - Loading app profiles from YAML/JSON
//! - Matching processes to known applications
//! - Three-tier classification (Unknown, Identified, Profiled)

use crate::events::{AppInfo, ProcessInfo};
#[cfg(test)]
use crate::events::{AppTier, CodeSignature};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

/// App Registry - holds all known app profiles and provides matching
#[derive(Debug, Clone)]
pub struct AppRegistry {
    /// All app profiles by app_id
    apps: HashMap<String, AppProfile>,
    /// Index: bundle_id -> app_id (for fast macOS lookup)
    bundle_id_index: HashMap<String, String>,
    /// Index: team_id -> app_id (for code signature lookup)
    team_id_index: HashMap<String, String>,
    /// Path patterns for matching (pattern, app_id)
    path_patterns: Vec<(PathPattern, String)>,
    /// Process name patterns (name, app_id)
    name_patterns: Vec<(String, String)>,
    /// Web app profiles for browser traffic identification
    web_apps: Vec<WebAppProfile>,
    /// Browser app IDs for quick lookup
    browser_app_ids: std::collections::HashSet<String>,
}

/// A single app profile from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppProfile {
    pub app_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(default)]
    pub signatures: AppSignatures,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traffic_patterns: Option<TrafficPatterns>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<AppMetadata>,
    /// Whether this app is a web browser (requires web_context extraction)
    #[serde(default)]
    pub is_browser: bool,
}

fn default_category() -> String {
    "other".to_string()
}

/// Platform-specific signatures
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSignatures {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macos: Option<MacOSSignature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub windows: Option<WindowsSignature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linux: Option<LinuxSignature>,
}

/// macOS-specific identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacOSSignature {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id_patterns: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_name: Option<String>,
    #[serde(default)]
    pub helper_bundles: Vec<String>,
}

/// Windows-specific identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsSignature {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msix_package_family: Option<String>,
}

/// Linux-specific identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxSignature {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_name: Option<String>,
    #[serde(default)]
    pub package_names: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desktop_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flatpak_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snap_name: Option<String>,
}

/// Traffic pattern expectations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficPatterns {
    #[serde(rename = "type")]
    pub pattern_type: Option<String>,
    #[serde(default)]
    pub expected_providers: Vec<String>,
    #[serde(default)]
    pub expected_models: Vec<String>,
    #[serde(default)]
    pub backend_domains: Vec<String>,
    /// Whether this app extracts web context from HTTP headers (browsers)
    #[serde(default)]
    pub extracts_web_context: bool,
}

/// Web app profile - for identifying web applications via Origin/Referer headers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAppProfile {
    /// Unique web app identifier
    pub web_app_id: String,
    /// Human-readable name
    pub name: String,
    /// Origin patterns that identify this web app (e.g., "https://chat.openai.com")
    #[serde(default)]
    pub origin_patterns: Vec<String>,
    /// Referer patterns (if Origin not present)
    #[serde(default)]
    pub referer_patterns: Vec<String>,
    /// Type of web app: "direct" (calls AI API directly) or "embedded" (AI via backend)
    pub web_app_type: String,
    /// Category (e.g., "chat", "productivity", "creative")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Which AI providers this web app uses
    #[serde(default)]
    pub providers: Vec<String>,
}

/// App metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_release: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_source: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_ai_app: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_ai_host: Option<bool>,
}

/// Path pattern for matching
#[derive(Debug, Clone)]
pub struct PathPattern {
    /// The pattern string (supports * and **)
    pub pattern: String,
    /// Whether this is a prefix match, suffix match, or contains
    pub match_type: PathMatchType,
}

#[derive(Debug, Clone)]
pub enum PathMatchType {
    Exact,
    Prefix,
    Suffix,
    Contains,
    Glob,
}

/// Result of matching a process against the registry
#[derive(Debug, Clone)]
pub enum MatchResult {
    /// Full profile found (Tier 2)
    Profiled(Box<AppProfile>),
    /// Basic identification only (Tier 1)
    Identified {
        app_id: String,
        name: String,
        reason: String,
    },
    /// No match found (Tier 0)
    Unknown,
}

impl AppRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            apps: HashMap::new(),
            bundle_id_index: HashMap::new(),
            team_id_index: HashMap::new(),
            path_patterns: Vec::new(),
            name_patterns: Vec::new(),
            web_apps: Vec::new(),
            browser_app_ids: std::collections::HashSet::new(),
        }
    }

    /// Load profiles from a directory of YAML files
    pub fn load_from_directory(dir: impl AsRef<Path>) -> Result<Self, AppRegistryError> {
        let dir = dir.as_ref();
        let mut registry = Self::new();

        if !dir.exists() {
            return Err(AppRegistryError::DirectoryNotFound(
                dir.display().to_string(),
            ));
        }

        // Load all .yaml and .yml files
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                match registry.load_profile_from_file(&path) {
                    Ok(_) => debug!("Loaded app profile from {}", path.display()),
                    Err(e) => warn!("Failed to load {}: {}", path.display(), e),
                }
            }
        }

        info!(
            "Loaded {} app profiles ({} bundle_ids, {} team_ids, {} path patterns)",
            registry.apps.len(),
            registry.bundle_id_index.len(),
            registry.team_id_index.len(),
            registry.path_patterns.len()
        );

        Ok(registry)
    }

    /// Load a single profile from a YAML file
    pub fn load_profile_from_file(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<(), AppRegistryError> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let profile: AppProfile = serde_yaml::from_str(&content)?;
        self.add_profile(profile);
        Ok(())
    }

    /// Load profiles from a JSON bundle
    pub fn load_from_json(json: &str) -> Result<Self, AppRegistryError> {
        let profiles: Vec<AppProfile> = serde_json::from_str(json)?;
        let mut registry = Self::new();
        for profile in profiles {
            registry.add_profile(profile);
        }
        Ok(registry)
    }

    /// Add a profile to the registry
    pub fn add_profile(&mut self, profile: AppProfile) {
        let app_id = profile.app_id.clone();

        // Track if this is a browser
        if profile.is_browser {
            self.browser_app_ids.insert(app_id.clone());
        }

        // Index macOS signatures
        if let Some(ref macos) = profile.signatures.macos {
            if let Some(ref bundle_id) = macos.bundle_id {
                self.bundle_id_index
                    .insert(bundle_id.clone(), app_id.clone());
            }
            // Also index helper bundles
            for helper in &macos.helper_bundles {
                self.bundle_id_index.insert(helper.clone(), app_id.clone());
            }
            if let Some(ref team_id) = macos.team_id {
                self.team_id_index.insert(team_id.clone(), app_id.clone());
            }
            // Index paths
            for path in &macos.paths {
                self.path_patterns
                    .push((PathPattern::from_string(path), app_id.clone()));
            }
            // Index executable name
            if let Some(ref exe_name) = macos.executable_name {
                self.name_patterns.push((exe_name.clone(), app_id.clone()));
            }
        }

        // Index Windows signatures
        if let Some(ref windows) = profile.signatures.windows {
            for path in &windows.paths {
                self.path_patterns
                    .push((PathPattern::from_string(path), app_id.clone()));
            }
            if let Some(ref exe_name) = windows.executable_name {
                self.name_patterns.push((exe_name.clone(), app_id.clone()));
            }
        }

        // Index Linux signatures
        if let Some(ref linux) = profile.signatures.linux {
            for path in &linux.paths {
                self.path_patterns
                    .push((PathPattern::from_string(path), app_id.clone()));
            }
            if let Some(ref exe_name) = linux.executable_name {
                self.name_patterns.push((exe_name.clone(), app_id.clone()));
            }
        }

        self.apps.insert(app_id, profile);
    }

    /// Match a process against the registry
    ///
    /// Matching priority:
    /// 1. Bundle ID (exact match) - strongest, macOS only
    /// 2. Team ID (code signature) - strong, macOS only
    /// 3. Path pattern match - medium
    /// 4. Process name - weak (high false positive potential)
    pub fn match_process(&self, process: &ProcessInfo) -> MatchResult {
        // 1. Try bundle ID match (strongest)
        if let Some(ref bundle_id) = process.bundle_id {
            if let Some(app_id) = self.bundle_id_index.get(bundle_id) {
                if let Some(profile) = self.apps.get(app_id) {
                    debug!(
                        "Matched process {} to app {} via bundle_id {}",
                        process.pid, app_id, bundle_id
                    );
                    return MatchResult::Profiled(Box::new(profile.clone()));
                }
            }
        }

        // 2. Try team ID match (code signature)
        if let Some(ref code_sig) = process.code_signature {
            if let Some(ref team_id) = code_sig.team_id {
                if let Some(app_id) = self.team_id_index.get(team_id) {
                    if let Some(profile) = self.apps.get(app_id) {
                        debug!(
                            "Matched process {} to app {} via team_id {}",
                            process.pid, app_id, team_id
                        );
                        return MatchResult::Profiled(Box::new(profile.clone()));
                    }
                }
            }
        }

        // 3. Try path pattern match
        if let Some(ref exe) = process.exe {
            for (pattern, app_id) in &self.path_patterns {
                if pattern.matches(exe) {
                    if let Some(profile) = self.apps.get(app_id) {
                        debug!(
                            "Matched process {} to app {} via path pattern {}",
                            process.pid, app_id, pattern.pattern
                        );
                        return MatchResult::Profiled(Box::new(profile.clone()));
                    }
                }
            }
        }

        // 4. Try process name match (weak)
        if let Some(ref name) = process.name {
            for (pattern_name, app_id) in &self.name_patterns {
                if name.eq_ignore_ascii_case(pattern_name) {
                    if let Some(profile) = self.apps.get(app_id) {
                        debug!(
                            "Matched process {} to app {} via process name {}",
                            process.pid, app_id, name
                        );
                        // Return as Identified (Tier 1) since name matching is weaker
                        return MatchResult::Identified {
                            app_id: profile.app_id.clone(),
                            name: profile.name.clone(),
                            reason: format!("process name match: {}", name),
                        };
                    }
                }
            }
        }

        // No match found
        MatchResult::Unknown
    }

    /// Get a profile by app_id
    pub fn get_profile(&self, app_id: &str) -> Option<&AppProfile> {
        self.apps.get(app_id)
    }

    /// Get all app IDs
    pub fn app_ids(&self) -> impl Iterator<Item = &String> {
        self.apps.keys()
    }

    /// Number of profiles in the registry
    pub fn len(&self) -> usize {
        self.apps.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }

    /// Check if an app_id corresponds to a browser
    pub fn is_browser(&self, app_id: &str) -> bool {
        self.browser_app_ids.contains(app_id)
    }

    /// Check if a matched process is a browser
    pub fn is_browser_match(&self, result: &MatchResult) -> bool {
        match result {
            MatchResult::Profiled(profile) => profile.is_browser,
            MatchResult::Identified { app_id, .. } => self.browser_app_ids.contains(app_id),
            MatchResult::Unknown => false,
        }
    }

    /// Add a web app profile for origin-based matching
    pub fn add_web_app(&mut self, profile: WebAppProfile) {
        self.web_apps.push(profile);
    }

    /// Match a web app by Origin or Referer header
    /// Returns (web_app_id, name, web_app_type) if matched
    pub fn match_web_app(
        &self,
        origin: Option<&str>,
        referer: Option<&str>,
    ) -> Option<WebAppMatch> {
        // Try Origin first (more reliable)
        if let Some(origin) = origin {
            for web_app in &self.web_apps {
                for pattern in &web_app.origin_patterns {
                    if origin_matches(origin, pattern) {
                        return Some(WebAppMatch {
                            web_app_id: web_app.web_app_id.clone(),
                            name: web_app.name.clone(),
                            web_app_type: web_app.web_app_type.clone(),
                        });
                    }
                }
            }
        }

        // Fall back to Referer
        if let Some(referer) = referer {
            for web_app in &self.web_apps {
                for pattern in &web_app.referer_patterns {
                    if referer_matches(referer, pattern) {
                        return Some(WebAppMatch {
                            web_app_id: web_app.web_app_id.clone(),
                            name: web_app.name.clone(),
                            web_app_type: web_app.web_app_type.clone(),
                        });
                    }
                }
                // Also check origin patterns against referer
                for pattern in &web_app.origin_patterns {
                    if referer_matches(referer, pattern) {
                        return Some(WebAppMatch {
                            web_app_id: web_app.web_app_id.clone(),
                            name: web_app.name.clone(),
                            web_app_type: web_app.web_app_type.clone(),
                        });
                    }
                }
            }
        }

        None
    }

    /// Load built-in web app profiles
    pub fn load_builtin_web_apps(&mut self) {
        // ChatGPT Web
        self.add_web_app(WebAppProfile {
            web_app_id: "chatgpt-web".to_string(),
            name: "ChatGPT".to_string(),
            origin_patterns: vec![
                "https://chat.openai.com".to_string(),
                "https://chatgpt.com".to_string(),
            ],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("chat".to_string()),
            providers: vec!["openai".to_string()],
        });

        // Claude Web
        self.add_web_app(WebAppProfile {
            web_app_id: "claude-web".to_string(),
            name: "Claude".to_string(),
            origin_patterns: vec!["https://claude.ai".to_string()],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("chat".to_string()),
            providers: vec!["anthropic".to_string()],
        });

        // Google Gemini
        self.add_web_app(WebAppProfile {
            web_app_id: "gemini-web".to_string(),
            name: "Google Gemini".to_string(),
            origin_patterns: vec![
                "https://gemini.google.com".to_string(),
                "https://bard.google.com".to_string(),
            ],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("chat".to_string()),
            providers: vec!["google".to_string()],
        });

        // Notion AI
        self.add_web_app(WebAppProfile {
            web_app_id: "notion-ai".to_string(),
            name: "Notion AI".to_string(),
            origin_patterns: vec![
                "https://www.notion.so".to_string(),
                "https://notion.so".to_string(),
            ],
            referer_patterns: vec![],
            web_app_type: "embedded".to_string(),
            category: Some("productivity".to_string()),
            providers: vec!["openai".to_string(), "anthropic".to_string()],
        });

        // Perplexity
        self.add_web_app(WebAppProfile {
            web_app_id: "perplexity-web".to_string(),
            name: "Perplexity".to_string(),
            origin_patterns: vec![
                "https://www.perplexity.ai".to_string(),
                "https://perplexity.ai".to_string(),
            ],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("search".to_string()),
            providers: vec!["openai".to_string(), "anthropic".to_string()],
        });

        // GitHub Copilot Web
        self.add_web_app(WebAppProfile {
            web_app_id: "github-copilot-web".to_string(),
            name: "GitHub Copilot Chat".to_string(),
            origin_patterns: vec!["https://github.com".to_string()],
            referer_patterns: vec![],
            web_app_type: "embedded".to_string(),
            category: Some("dev_tools".to_string()),
            providers: vec!["openai".to_string()],
        });

        // Poe
        self.add_web_app(WebAppProfile {
            web_app_id: "poe-web".to_string(),
            name: "Poe".to_string(),
            origin_patterns: vec!["https://poe.com".to_string()],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("chat".to_string()),
            providers: vec![
                "openai".to_string(),
                "anthropic".to_string(),
                "google".to_string(),
            ],
        });

        // You.com
        self.add_web_app(WebAppProfile {
            web_app_id: "you-web".to_string(),
            name: "You.com".to_string(),
            origin_patterns: vec!["https://you.com".to_string()],
            referer_patterns: vec![],
            web_app_type: "embedded".to_string(),
            category: Some("search".to_string()),
            providers: vec!["openai".to_string()],
        });

        // Hugging Face
        self.add_web_app(WebAppProfile {
            web_app_id: "huggingface-web".to_string(),
            name: "Hugging Face".to_string(),
            origin_patterns: vec!["https://huggingface.co".to_string()],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("dev_tools".to_string()),
            providers: vec!["huggingface".to_string()],
        });

        // Replicate
        self.add_web_app(WebAppProfile {
            web_app_id: "replicate-web".to_string(),
            name: "Replicate".to_string(),
            origin_patterns: vec!["https://replicate.com".to_string()],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("creative".to_string()),
            providers: vec!["replicate".to_string()],
        });

        // v0 by Vercel
        self.add_web_app(WebAppProfile {
            web_app_id: "v0-web".to_string(),
            name: "v0".to_string(),
            origin_patterns: vec!["https://v0.dev".to_string()],
            referer_patterns: vec![],
            web_app_type: "embedded".to_string(),
            category: Some("dev_tools".to_string()),
            providers: vec!["openai".to_string()],
        });

        // Midjourney (Discord-based, but has web interface)
        self.add_web_app(WebAppProfile {
            web_app_id: "midjourney-web".to_string(),
            name: "Midjourney".to_string(),
            origin_patterns: vec![
                "https://www.midjourney.com".to_string(),
                "https://midjourney.com".to_string(),
            ],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("creative".to_string()),
            providers: vec!["midjourney".to_string()],
        });

        // OpenAI Playground
        self.add_web_app(WebAppProfile {
            web_app_id: "openai-playground".to_string(),
            name: "OpenAI Playground".to_string(),
            origin_patterns: vec!["https://platform.openai.com".to_string()],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("dev_tools".to_string()),
            providers: vec!["openai".to_string()],
        });

        // Anthropic Console
        self.add_web_app(WebAppProfile {
            web_app_id: "anthropic-console".to_string(),
            name: "Anthropic Console".to_string(),
            origin_patterns: vec!["https://console.anthropic.com".to_string()],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("dev_tools".to_string()),
            providers: vec!["anthropic".to_string()],
        });

        // Grammarly
        self.add_web_app(WebAppProfile {
            web_app_id: "grammarly-web".to_string(),
            name: "Grammarly".to_string(),
            origin_patterns: vec![
                "https://app.grammarly.com".to_string(),
                "https://www.grammarly.com".to_string(),
            ],
            referer_patterns: vec![],
            web_app_type: "embedded".to_string(),
            category: Some("productivity".to_string()),
            providers: vec!["openai".to_string()],
        });

        // Jasper
        self.add_web_app(WebAppProfile {
            web_app_id: "jasper-web".to_string(),
            name: "Jasper".to_string(),
            origin_patterns: vec![
                "https://app.jasper.ai".to_string(),
                "https://www.jasper.ai".to_string(),
            ],
            referer_patterns: vec![],
            web_app_type: "embedded".to_string(),
            category: Some("productivity".to_string()),
            providers: vec!["openai".to_string()],
        });

        // Copy.ai
        self.add_web_app(WebAppProfile {
            web_app_id: "copyai-web".to_string(),
            name: "Copy.ai".to_string(),
            origin_patterns: vec![
                "https://app.copy.ai".to_string(),
                "https://www.copy.ai".to_string(),
            ],
            referer_patterns: vec![],
            web_app_type: "embedded".to_string(),
            category: Some("productivity".to_string()),
            providers: vec!["openai".to_string()],
        });

        // Codeium
        self.add_web_app(WebAppProfile {
            web_app_id: "codeium-web".to_string(),
            name: "Codeium".to_string(),
            origin_patterns: vec!["https://codeium.com".to_string()],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("dev_tools".to_string()),
            providers: vec!["codeium".to_string()],
        });

        // Character.AI
        self.add_web_app(WebAppProfile {
            web_app_id: "characterai-web".to_string(),
            name: "Character.AI".to_string(),
            origin_patterns: vec![
                "https://character.ai".to_string(),
                "https://beta.character.ai".to_string(),
            ],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("chat".to_string()),
            providers: vec!["characterai".to_string()],
        });

        // Runway
        self.add_web_app(WebAppProfile {
            web_app_id: "runway-web".to_string(),
            name: "Runway".to_string(),
            origin_patterns: vec![
                "https://app.runwayml.com".to_string(),
                "https://runwayml.com".to_string(),
            ],
            referer_patterns: vec![],
            web_app_type: "direct".to_string(),
            category: Some("creative".to_string()),
            providers: vec!["runway".to_string()],
        });

        info!("Loaded {} built-in web app profiles", self.web_apps.len());
    }

    /// Get all web apps
    pub fn web_apps(&self) -> &[WebAppProfile] {
        &self.web_apps
    }
}

/// Result of matching a web app
#[derive(Debug, Clone)]
pub struct WebAppMatch {
    pub web_app_id: String,
    pub name: String,
    pub web_app_type: String,
}

/// Check if an origin matches a pattern
fn origin_matches(origin: &str, pattern: &str) -> bool {
    // Exact match or pattern match
    origin == pattern || origin.starts_with(pattern)
}

/// Check if a referer matches a pattern
fn referer_matches(referer: &str, pattern: &str) -> bool {
    // Referer might have a path, so check if it starts with the pattern
    referer.starts_with(pattern) || referer.contains(pattern)
}

impl Default for AppRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PathPattern {
    /// Create a pattern from a string
    pub fn from_string(s: &str) -> Self {
        let match_type = if s.contains("**") || s.contains('*') {
            PathMatchType::Glob
        } else if s.starts_with('/') || s.starts_with('~') || s.starts_with('%') {
            PathMatchType::Prefix
        } else {
            PathMatchType::Contains
        };

        Self {
            pattern: s.to_string(),
            match_type,
        }
    }

    /// Check if a path matches this pattern
    pub fn matches(&self, path: &str) -> bool {
        match self.match_type {
            PathMatchType::Exact => path == self.pattern,
            PathMatchType::Prefix => {
                let pattern = self.normalize_path(&self.pattern);
                let normalized = self.normalize_path(path);
                normalized.starts_with(&pattern)
            }
            PathMatchType::Suffix => path.ends_with(&self.pattern),
            PathMatchType::Contains => {
                let pattern = self.normalize_path(&self.pattern);
                let normalized = self.normalize_path(path);
                normalized.contains(&pattern)
            }
            PathMatchType::Glob => self.glob_match(path),
        }
    }

    /// Normalize path for comparison
    fn normalize_path(&self, path: &str) -> String {
        // Expand ~ to home directory conceptually (for matching purposes)
        // In actual use, we compare against the actual expanded path
        let path = if path.starts_with("~/") {
            // This is a pattern with ~, it will match against expanded paths
            path.to_string()
        } else {
            path.to_string()
        };

        // Normalize Windows-style paths
        path.replace('\\', "/")
    }

    /// Simple glob matching (supports * and **)
    fn glob_match(&self, path: &str) -> bool {
        let pattern = self.normalize_path(&self.pattern);
        let path = self.normalize_path(path);

        // Simple glob implementation
        // ** matches any number of path segments
        // * matches anything except /

        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        self.glob_match_parts(&pattern_parts, &path_parts)
    }

    fn glob_match_parts(&self, pattern: &[&str], path: &[&str]) -> bool {
        if pattern.is_empty() {
            return path.is_empty();
        }

        let p = pattern[0];

        if p == "**" {
            // ** can match zero or more path segments
            // Try matching rest of pattern at each position
            for i in 0..=path.len() {
                if self.glob_match_parts(&pattern[1..], &path[i..]) {
                    return true;
                }
            }
            return false;
        }

        if path.is_empty() {
            return false;
        }

        // Single segment matching with * support
        if self.segment_matches(p, path[0]) {
            self.glob_match_parts(&pattern[1..], &path[1..])
        } else {
            false
        }
    }

    fn segment_matches(&self, pattern: &str, segment: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if !pattern.contains('*') {
            return pattern == segment;
        }

        // Simple * matching within segment
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let (prefix, suffix) = (parts[0], parts[1]);
            return segment.starts_with(prefix) && segment.ends_with(suffix);
        }

        // For more complex patterns, fall back to exact match
        pattern == segment
    }
}

impl MatchResult {
    /// Convert match result to AppInfo
    pub fn to_app_info(&self) -> AppInfo {
        match self {
            MatchResult::Profiled(profile) => {
                let mut app_info = AppInfo::profiled(&profile.app_id, &profile.name);

                if let Some(ref vendor) = profile.vendor {
                    app_info = app_info.with_vendor(vendor);
                }

                app_info = app_info.with_category(&profile.category);

                // Set bundle_id from macOS signature if available
                if let Some(ref macos) = profile.signatures.macos {
                    if let Some(ref bundle_id) = macos.bundle_id {
                        app_info = app_info.with_bundle_id(bundle_id);
                    }
                }

                // Set AI flags from metadata
                if let Some(ref metadata) = profile.metadata {
                    if metadata.is_ai_app == Some(true) {
                        app_info = app_info.as_ai_app();
                    }
                    if metadata.is_ai_host == Some(true) {
                        app_info = app_info.as_ai_host();
                    }
                }

                app_info
            }
            MatchResult::Identified { app_id, name, .. } => AppInfo::identified(app_id, name),
            MatchResult::Unknown => AppInfo::unknown(),
        }
    }
}

/// Errors that can occur when working with the app registry
#[derive(Debug, thiserror::Error)]
pub enum AppRegistryError {
    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_profile() -> AppProfile {
        AppProfile {
            app_id: "cursor".to_string(),
            name: "Cursor".to_string(),
            vendor: Some("Anysphere Inc.".to_string()),
            category: "dev_tools".to_string(),
            subcategory: Some("ide".to_string()),
            description: None,
            website: None,
            signatures: AppSignatures {
                macos: Some(MacOSSignature {
                    bundle_id: Some("com.todesktop.230313mzl4w4u92".to_string()),
                    bundle_id_patterns: None,
                    team_id: Some("VDXQ22DGB9".to_string()),
                    paths: vec!["/Applications/Cursor.app".to_string()],
                    executable_name: Some("Cursor".to_string()),
                    helper_bundles: vec!["com.todesktop.230313mzl4w4u92.helper".to_string()],
                }),
                windows: None,
                linux: None,
            },
            traffic_patterns: None,
            metadata: Some(AppMetadata {
                icon_url: None,
                first_release: None,
                open_source: Some(false),
                pricing: Some("freemium".to_string()),
                is_ai_app: Some(true),
                is_ai_host: None,
            }),
            is_browser: false,
        }
    }

    #[test]
    fn test_match_by_bundle_id() {
        let mut registry = AppRegistry::new();
        registry.add_profile(create_test_profile());

        let process = ProcessInfo {
            pid: 1234,
            bundle_id: Some("com.todesktop.230313mzl4w4u92".to_string()),
            ..Default::default()
        };

        let result = registry.match_process(&process);
        assert!(matches!(result, MatchResult::Profiled(_)));

        if let MatchResult::Profiled(profile) = result {
            assert_eq!(profile.app_id, "cursor");
            assert_eq!(profile.name, "Cursor");
        }
    }

    #[test]
    fn test_match_by_helper_bundle_id() {
        let mut registry = AppRegistry::new();
        registry.add_profile(create_test_profile());

        let process = ProcessInfo {
            pid: 1234,
            bundle_id: Some("com.todesktop.230313mzl4w4u92.helper".to_string()),
            ..Default::default()
        };

        let result = registry.match_process(&process);
        assert!(matches!(result, MatchResult::Profiled(_)));
    }

    #[test]
    fn test_match_by_team_id() {
        let mut registry = AppRegistry::new();
        registry.add_profile(create_test_profile());

        let process = ProcessInfo {
            pid: 1234,
            code_signature: Some(CodeSignature {
                signed: true,
                signer: None,
                team_id: Some("VDXQ22DGB9".to_string()),
                valid: Some(true),
            }),
            ..Default::default()
        };

        let result = registry.match_process(&process);
        assert!(matches!(result, MatchResult::Profiled(_)));
    }

    #[test]
    fn test_match_by_path() {
        let mut registry = AppRegistry::new();
        registry.add_profile(create_test_profile());

        let process = ProcessInfo {
            pid: 1234,
            exe: Some("/Applications/Cursor.app/Contents/MacOS/Cursor".to_string()),
            ..Default::default()
        };

        let result = registry.match_process(&process);
        assert!(matches!(result, MatchResult::Profiled(_)));
    }

    #[test]
    fn test_no_match() {
        let mut registry = AppRegistry::new();
        registry.add_profile(create_test_profile());

        let process = ProcessInfo {
            pid: 1234,
            exe: Some("/usr/bin/curl".to_string()),
            name: Some("curl".to_string()),
            ..Default::default()
        };

        let result = registry.match_process(&process);
        assert!(matches!(result, MatchResult::Unknown));
    }

    #[test]
    fn test_to_app_info() {
        let mut registry = AppRegistry::new();
        registry.add_profile(create_test_profile());

        let process = ProcessInfo {
            pid: 1234,
            bundle_id: Some("com.todesktop.230313mzl4w4u92".to_string()),
            ..Default::default()
        };

        let result = registry.match_process(&process);
        let app_info = result.to_app_info();

        assert_eq!(app_info.tier, AppTier::Profiled);
        assert_eq!(app_info.app_id, Some("cursor".to_string()));
        assert_eq!(app_info.name, Some("Cursor".to_string()));
        assert_eq!(app_info.vendor, Some("Anysphere Inc.".to_string()));
        assert_eq!(app_info.is_ai_app, Some(true));
    }

    #[test]
    fn test_path_pattern_glob() {
        let pattern = PathPattern::from_string("**/Cursor.app/**");
        assert!(pattern.matches("/Applications/Cursor.app/Contents/MacOS/Cursor"));
        assert!(pattern.matches("/Users/test/Applications/Cursor.app/Contents/MacOS/Cursor"));
        assert!(!pattern.matches("/Applications/VSCode.app/Contents/MacOS/VSCode"));
    }

    #[test]
    fn test_web_app_matching_by_origin() {
        let mut registry = AppRegistry::new();
        registry.load_builtin_web_apps();

        // Test ChatGPT origin
        let result = registry.match_web_app(Some("https://chat.openai.com"), None);
        assert!(result.is_some());
        let web_match = result.unwrap();
        assert_eq!(web_match.web_app_id, "chatgpt-web");
        assert_eq!(web_match.name, "ChatGPT");
        assert_eq!(web_match.web_app_type, "direct");

        // Test Claude origin
        let result = registry.match_web_app(Some("https://claude.ai"), None);
        assert!(result.is_some());
        let web_match = result.unwrap();
        assert_eq!(web_match.web_app_id, "claude-web");
        assert_eq!(web_match.name, "Claude");

        // Test unknown origin
        let result = registry.match_web_app(Some("https://unknown-site.com"), None);
        assert!(result.is_none());
    }

    #[test]
    fn test_web_app_matching_by_referer() {
        let mut registry = AppRegistry::new();
        registry.load_builtin_web_apps();

        // Test with referer (full URL with path)
        let result = registry.match_web_app(None, Some("https://chat.openai.com/c/123456"));
        assert!(result.is_some());
        let web_match = result.unwrap();
        assert_eq!(web_match.web_app_id, "chatgpt-web");
    }

    #[test]
    fn test_browser_detection() {
        let mut registry = AppRegistry::new();

        // Add a browser profile
        let browser_profile = AppProfile {
            app_id: "chrome".to_string(),
            name: "Google Chrome".to_string(),
            vendor: Some("Google LLC".to_string()),
            category: "browser".to_string(),
            subcategory: None,
            description: None,
            website: None,
            signatures: AppSignatures {
                macos: Some(MacOSSignature {
                    bundle_id: Some("com.google.Chrome".to_string()),
                    bundle_id_patterns: None,
                    team_id: Some("EQHXZ8M8AV".to_string()),
                    paths: vec![],
                    executable_name: None,
                    helper_bundles: vec![],
                }),
                windows: None,
                linux: None,
            },
            traffic_patterns: None,
            metadata: None,
            is_browser: true,
        };
        registry.add_profile(browser_profile);

        // Test browser detection
        assert!(registry.is_browser("chrome"));
        assert!(!registry.is_browser("cursor"));

        // Test with match result
        let process = ProcessInfo {
            pid: 1234,
            bundle_id: Some("com.google.Chrome".to_string()),
            ..Default::default()
        };
        let result = registry.match_process(&process);
        assert!(registry.is_browser_match(&result));
    }
}
