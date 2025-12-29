//! App identification enrichment
//!
//! Enriches events with application information by matching
//! process info against the app registry. Also enriches web context
//! with web app identification when Origin/Referer headers are present.

use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;

use crate::app_registry::AppRegistry;
use crate::events::{OispEvent, WebAppType};
use crate::plugins::{EnrichPlugin, Plugin, PluginInfo, PluginResult};

/// App enricher - identifies applications from process info
pub struct AppEnricher {
    registry: Arc<AppRegistry>,
}

impl AppEnricher {
    /// Create a new AppEnricher with the given registry
    pub fn new(registry: Arc<AppRegistry>) -> Self {
        Self { registry }
    }

    /// Create an AppEnricher with an empty registry
    pub fn empty() -> Self {
        Self {
            registry: Arc::new(AppRegistry::new()),
        }
    }

    /// Get the underlying registry
    pub fn registry(&self) -> &AppRegistry {
        &self.registry
    }
}

impl PluginInfo for AppEnricher {
    fn name(&self) -> &str {
        "app-enricher"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Enriches events with application identification"
    }
}

impl Plugin for AppEnricher {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl EnrichPlugin for AppEnricher {
    async fn enrich(&self, event: &mut OispEvent) -> PluginResult<()> {
        let envelope = match event {
            OispEvent::AiRequest(e) => &mut e.envelope,
            OispEvent::AiResponse(e) => &mut e.envelope,
            OispEvent::ProcessExec(e) => &mut e.envelope,
            OispEvent::NetworkConnect(e) => &mut e.envelope,
            OispEvent::FileWrite(e) => &mut e.envelope,
            _ => return Ok(()),
        };

        // Enrich app info if not already set
        if envelope.app.is_none() {
            // Need process info to match
            if let Some(ref process) = envelope.process {
                let match_result = self.registry.match_process(process);
                let app_info = match_result.to_app_info();

                // Only set app info if we found something
                // For Unknown tier, we still set it to indicate we tried
                envelope.app = Some(app_info);
            }
        }

        // Enrich web context with web app identification if Origin/Referer are present
        if let Some(ref mut web_context) = envelope.web_context {
            // Only enrich if web_app_id is not already set
            if web_context.web_app_id.is_none() {
                let origin = web_context.origin.as_deref();
                let referer = web_context.referer.as_deref();

                if let Some(web_match) = self.registry.match_web_app(origin, referer) {
                    web_context.web_app_id = Some(web_match.web_app_id);
                    web_context.web_app_name = Some(web_match.name);
                    web_context.web_app_type = match web_match.web_app_type.as_str() {
                        "direct" => Some(WebAppType::Direct),
                        "embedded" => Some(WebAppType::Embedded),
                        _ => None,
                    };
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_registry::{AppMetadata, AppProfile, AppSignatures, MacOSSignature};
    use crate::events::{
        AiRequestData, AiRequestEvent, AppTier, EventEnvelope, OispEvent, ProcessInfo,
    };

    fn create_test_registry() -> Arc<AppRegistry> {
        let mut registry = AppRegistry::new();

        registry.add_profile(AppProfile {
            app_id: "cursor".to_string(),
            name: "Cursor".to_string(),
            vendor: Some("Anysphere Inc.".to_string()),
            category: "dev_tools".to_string(),
            subcategory: None,
            description: None,
            website: None,
            signatures: AppSignatures {
                macos: Some(MacOSSignature {
                    bundle_id: Some("com.todesktop.230313mzl4w4u92".to_string()),
                    bundle_id_patterns: None,
                    team_id: Some("VDXQ22DGB9".to_string()),
                    paths: vec!["/Applications/Cursor.app".to_string()],
                    executable_name: Some("Cursor".to_string()),
                    helper_bundles: vec![],
                }),
                windows: None,
                linux: None,
            },
            traffic_patterns: None,
            metadata: Some(AppMetadata {
                icon_url: None,
                first_release: None,
                open_source: Some(false),
                pricing: None,
                is_ai_app: Some(true),
                is_ai_host: None,
            }),
            is_browser: false,
        });

        Arc::new(registry)
    }

    fn create_test_request_data() -> AiRequestData {
        AiRequestData {
            request_id: "test-req-123".to_string(),
            provider: None,
            model: None,
            auth: None,
            request_type: None,
            streaming: None,
            messages: vec![],
            messages_count: None,
            has_system_prompt: None,
            system_prompt_hash: None,
            tools: vec![],
            tools_count: None,
            tool_choice: None,
            parameters: None,
            has_rag_context: None,
            has_images: None,
            image_count: None,
            estimated_tokens: None,
            conversation: None,
            agent: None,
        }
    }

    fn create_test_event(process: ProcessInfo) -> OispEvent {
        let mut envelope = EventEnvelope::new("ai.request");
        envelope.process = Some(process);

        OispEvent::AiRequest(AiRequestEvent {
            envelope,
            data: create_test_request_data(),
        })
    }

    #[tokio::test]
    async fn test_enrich_with_bundle_id() {
        let registry = create_test_registry();
        let enricher = AppEnricher::new(registry);

        let process = ProcessInfo {
            pid: 1234,
            bundle_id: Some("com.todesktop.230313mzl4w4u92".to_string()),
            ..Default::default()
        };

        let mut event = create_test_event(process);
        enricher.enrich(&mut event).await.unwrap();

        if let OispEvent::AiRequest(e) = &event {
            let app = e.envelope.app.as_ref().unwrap();
            assert_eq!(app.tier, AppTier::Profiled);
            assert_eq!(app.app_id, Some("cursor".to_string()));
            assert_eq!(app.name, Some("Cursor".to_string()));
            assert_eq!(app.is_ai_app, Some(true));
        } else {
            panic!("Expected AiRequest event");
        }
    }

    #[tokio::test]
    async fn test_enrich_unknown_process() {
        let registry = create_test_registry();
        let enricher = AppEnricher::new(registry);

        let process = ProcessInfo {
            pid: 5678,
            exe: Some("/usr/bin/curl".to_string()),
            name: Some("curl".to_string()),
            ..Default::default()
        };

        let mut event = create_test_event(process);
        enricher.enrich(&mut event).await.unwrap();

        if let OispEvent::AiRequest(e) = &event {
            let app = e.envelope.app.as_ref().unwrap();
            assert_eq!(app.tier, AppTier::Unknown);
            assert_eq!(app.app_id, None);
        } else {
            panic!("Expected AiRequest event");
        }
    }

    #[tokio::test]
    async fn test_skip_if_app_already_set() {
        let registry = create_test_registry();
        let enricher = AppEnricher::new(registry);

        let process = ProcessInfo {
            pid: 1234,
            bundle_id: Some("com.todesktop.230313mzl4w4u92".to_string()),
            ..Default::default()
        };

        let mut envelope = EventEnvelope::new("ai.request");
        envelope.process = Some(process);
        envelope.app = Some(crate::events::AppInfo::identified("custom", "Custom App"));

        let mut event = OispEvent::AiRequest(AiRequestEvent {
            envelope,
            data: create_test_request_data(),
        });

        enricher.enrich(&mut event).await.unwrap();

        // Should not have been replaced
        if let OispEvent::AiRequest(e) = &event {
            let app = e.envelope.app.as_ref().unwrap();
            assert_eq!(app.app_id, Some("custom".to_string()));
        }
    }

    #[tokio::test]
    async fn test_enrich_web_context() {
        let mut registry = AppRegistry::new();
        registry.load_builtin_web_apps();
        let enricher = AppEnricher::new(Arc::new(registry));

        let process = ProcessInfo {
            pid: 1234,
            name: Some("Google Chrome".to_string()),
            ..Default::default()
        };

        let mut envelope = EventEnvelope::new("ai.request");
        envelope.process = Some(process);
        envelope.web_context = Some(crate::events::WebContext::from_headers(
            Some("https://chat.openai.com".to_string()),
            None,
            Some("Mozilla/5.0 Chrome".to_string()),
        ));

        let mut event = OispEvent::AiRequest(AiRequestEvent {
            envelope,
            data: create_test_request_data(),
        });

        enricher.enrich(&mut event).await.unwrap();

        // Should have enriched web context with ChatGPT identification
        if let OispEvent::AiRequest(e) = &event {
            let web_ctx = e.envelope.web_context.as_ref().unwrap();
            assert_eq!(web_ctx.web_app_id, Some("chatgpt-web".to_string()));
            assert_eq!(web_ctx.web_app_name, Some("ChatGPT".to_string()));
            assert_eq!(web_ctx.web_app_type, Some(WebAppType::Direct));
        } else {
            panic!("Expected AiRequest event");
        }
    }
}
