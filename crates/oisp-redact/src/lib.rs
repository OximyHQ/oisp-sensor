//! Redaction action plugin

use oisp_core::events::OispEvent;
use oisp_core::plugins::{
    ActionPlugin, Plugin, PluginInfo, PluginConfig, PluginResult, EventAction,
};
use oisp_core::redaction::{RedactionConfig, RedactionMode};
use async_trait::async_trait;
use std::any::Any;

/// Redaction action plugin
pub struct RedactionPlugin {
    config: RedactionConfig,
}

impl RedactionPlugin {
    pub fn new(config: RedactionConfig) -> Self {
        Self { config }
    }
    
    pub fn safe_mode() -> Self {
        Self::new(RedactionConfig {
            mode: RedactionMode::Safe,
            ..Default::default()
        })
    }
    
    pub fn full_capture() -> Self {
        Self::new(RedactionConfig {
            mode: RedactionMode::Full,
            ..Default::default()
        })
    }
    
    pub fn minimal() -> Self {
        Self::new(RedactionConfig {
            mode: RedactionMode::Minimal,
            ..Default::default()
        })
    }
}

impl Default for RedactionPlugin {
    fn default() -> Self {
        Self::safe_mode()
    }
}

impl PluginInfo for RedactionPlugin {
    fn name(&self) -> &str {
        "redaction"
    }
    
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    
    fn description(&self) -> &str {
        "Redacts sensitive information from events"
    }
}

impl Plugin for RedactionPlugin {
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()> {
        if let Some(mode) = config.get::<String>("mode") {
            self.config.mode = match mode.as_str() {
                "safe" => RedactionMode::Safe,
                "full" => RedactionMode::Full,
                "minimal" => RedactionMode::Minimal,
                _ => RedactionMode::Safe,
            };
        }
        if let Some(redact_api_keys) = config.get::<bool>("redact_api_keys") {
            self.config.redact_api_keys = redact_api_keys;
        }
        if let Some(redact_emails) = config.get::<bool>("redact_emails") {
            self.config.redact_emails = redact_emails;
        }
        if let Some(patterns) = config.get::<Vec<String>>("custom_patterns") {
            self.config.custom_patterns = patterns;
        }
        Ok(())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl ActionPlugin for RedactionPlugin {
    async fn process(&self, event: OispEvent) -> PluginResult<(OispEvent, EventAction)> {
        // In minimal mode, we might want to drop content entirely
        // In safe mode, we redact sensitive patterns
        // In full mode, we pass through
        
        if self.config.mode == RedactionMode::Full {
            return Ok((event, EventAction::Pass));
        }
        
        // For now, just pass through - actual redaction would be implemented
        // by walking the event structure and applying redaction to string fields
        
        Ok((event, EventAction::Pass))
    }
    
    fn applies_to(&self, event: &OispEvent) -> bool {
        // Apply to AI events which contain potentially sensitive content
        event.is_ai_event()
    }
}

