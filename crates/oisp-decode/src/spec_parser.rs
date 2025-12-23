//! Spec-driven AI request/response parsing
//!
//! Uses JSONPath extraction rules from the OISP spec bundle to parse
//! AI requests and responses dynamically. This allows adding new providers
//! without code changes.

use oisp_core::events::{
    AgentContext, AiRequestData, AiResponseData, Choice, ConversationContext, FinishReason,
    Message, MessageContent, MessageRole, ModelInfo, ModelParameters, ProviderInfo, RequestType,
    ThinkingBlock, ThinkingMode, ToolArguments, ToolCall, ToolDefinition, ToolType, Usage,
};
use oisp_core::spec::{DynamicProviderRegistry, EndpointRules, ExtractionRuleSet};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

/// Spec-driven AI parser
pub struct SpecDrivenParser {
    registry: Arc<DynamicProviderRegistry>,
}

impl SpecDrivenParser {
    /// Create a new parser with the given registry
    pub fn new(registry: Arc<DynamicProviderRegistry>) -> Self {
        Self { registry }
    }

    /// Parse an AI request using spec extraction rules
    pub fn parse_request(
        &self,
        provider_id: &str,
        endpoint_path: &str,
        body: &Value,
    ) -> Option<AiRequestData> {
        let rules = self.registry.get_extraction_rules(provider_id)?;
        let endpoint = self.find_matching_endpoint(rules, endpoint_path)?;

        let extraction = &endpoint.request_extraction;

        // Extract model
        let model_id = extract_string(body, extraction.get("model")?);
        let model = model_id.map(|id| {
            let model_info = self.registry.get_model(provider_id, &id);
            ModelInfo {
                id,
                name: model_info.and_then(|m| m.litellm_id.clone()),
                family: model_info.map(|m| m.provider.clone()),
                version: None,
                capabilities: None,
                context_window: model_info.and_then(|m| m.max_input_tokens),
                max_output_tokens: model_info.and_then(|m| m.max_output_tokens),
            }
        });

        // Extract messages
        let messages = extract_messages(body, extraction.get("messages"));

        // Extract streaming flag
        let streaming = extraction
            .get("stream")
            .and_then(|path| extract_bool(body, path));

        // Extract tools
        let tools = extract_tools(body, extraction.get("tools"));

        // Extract parameters
        let parameters = extract_parameters(body, extraction);

        // Detect request type
        let request_type = match endpoint.request_type.as_str() {
            "chat" => RequestType::Chat,
            "completion" => RequestType::Completion,
            "embedding" => RequestType::Embedding,
            "image" => RequestType::Image,
            "audio" => RequestType::Audio,
            _ => RequestType::Other,
        };

        let has_system_prompt = messages
            .iter()
            .any(|m| matches!(m.role, MessageRole::System));

        let system_prompt_hash = messages
            .iter()
            .find(|m| matches!(m.role, MessageRole::System))
            .and_then(|m| m.content_hash.clone());

        // Build conversation context
        let context_window = model.as_ref().and_then(|m| m.context_window);
        let conversation = Some(ConversationContext::from_messages(
            &messages,
            context_window,
        ));

        // Detect agent context
        let agent = AgentContext::detect(&tools, &messages);

        Some(AiRequestData {
            request_id: ulid::Ulid::new().to_string(),
            provider: Some(ProviderInfo {
                name: provider_id.to_string(),
                endpoint: Some(endpoint_path.to_string()),
                region: None,
                organization_id: None,
                project_id: None,
            }),
            model,
            auth: None,
            request_type: Some(request_type),
            streaming,
            messages: messages.clone(),
            messages_count: Some(messages.len()),
            has_system_prompt: Some(has_system_prompt),
            system_prompt_hash,
            tools: tools.clone(),
            tools_count: Some(tools.len()),
            tool_choice: extraction
                .get("tool_choice")
                .and_then(|path| extract_string(body, path)),
            parameters: Some(parameters),
            has_rag_context: None,
            has_images: Some(messages.iter().any(|m| m.has_images == Some(true))),
            image_count: messages
                .iter()
                .filter_map(|m| m.image_count)
                .sum::<usize>()
                .into(),
            estimated_tokens: None,
            conversation,
            agent,
        })
    }

    /// Parse an AI response using spec extraction rules
    pub fn parse_response(
        &self,
        provider_id: &str,
        endpoint_path: &str,
        body: &Value,
        request_id: &str,
    ) -> Option<AiResponseData> {
        let rules = self.registry.get_extraction_rules(provider_id)?;
        let endpoint = self.find_matching_endpoint(rules, endpoint_path)?;

        let extraction = &endpoint.response_extraction;

        // Extract model from response
        let model_id = extraction
            .get("model")
            .and_then(|path| extract_string(body, path));
        let model = model_id.map(|id| ModelInfo {
            id,
            name: None,
            family: None,
            version: None,
            capabilities: None,
            context_window: None,
            max_output_tokens: None,
        });

        // Extract usage
        let usage = extract_usage(body, extraction.get("usage"));

        // Extract choices (OpenAI style)
        let choices = extract_choices(body);

        // Extract tool calls
        let tool_calls = extract_tool_calls(body);

        // Extract finish reason
        let finish_reason = extraction
            .get("finish_reason")
            .and_then(|path| extract_string(body, path))
            .or_else(|| {
                // Try standard locations
                body.get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("finish_reason"))
                    .and_then(|f| f.as_str())
                    .map(String::from)
            })
            .or_else(|| {
                // Anthropic style
                body.get("stop_reason")
                    .and_then(|s| s.as_str())
                    .map(String::from)
            })
            .and_then(|r| parse_finish_reason(&r));

        // Extract thinking/reasoning blocks
        let thinking = extract_thinking_block(body, provider_id, usage.as_ref());

        Some(AiResponseData {
            request_id: request_id.to_string(),
            provider_request_id: body.get("id").and_then(|i| i.as_str()).map(String::from),
            provider: Some(ProviderInfo {
                name: provider_id.to_string(),
                endpoint: None,
                region: None,
                organization_id: None,
                project_id: None,
            }),
            model,
            status_code: None,
            success: Some(true),
            error: None,
            choices,
            tool_calls: tool_calls.clone(),
            tool_calls_count: Some(tool_calls.len()),
            usage,
            latency_ms: None,
            time_to_first_token_ms: None,
            was_cached: None,
            finish_reason,
            thinking,
        })
    }

    /// Find the matching endpoint for a given path
    fn find_matching_endpoint<'a>(
        &self,
        rules: &'a ExtractionRuleSet,
        path: &str,
    ) -> Option<&'a EndpointRules> {
        // First, try exact pattern matching
        for endpoint in rules.endpoints.values() {
            if pattern_matches(&endpoint.path, path) {
                return Some(endpoint);
            }
        }

        // Default fallback: return first chat endpoint
        rules.endpoints.values().find(|e| e.request_type == "chat")
    }
}

/// Check if a pattern matches a path (simple implementation)
fn pattern_matches(pattern: &str, path: &str) -> bool {
    // Handle patterns like "/v1/messages" or "/v1/chat/completions"
    let pattern_clean = pattern.trim_start_matches('/');
    let path_clean = path.trim_start_matches('/');

    // Exact match
    if pattern_clean == path_clean {
        return true;
    }

    // Pattern is a prefix
    if path_clean.starts_with(pattern_clean) {
        return true;
    }

    // Pattern contains placeholders like {model} or {model}:suffix
    if pattern.contains('{') {
        // Split into parts and match non-placeholder segments
        let pattern_parts: Vec<&str> = pattern_clean.split('/').collect();
        let path_parts: Vec<&str> = path_clean.split('/').collect();

        if pattern_parts.len() > path_parts.len() {
            return false;
        }

        for (p, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
            // Handle placeholders like {model} or {model}:generateContent
            if p.contains('{') && p.contains('}') {
                // Extract the suffix after the placeholder if any
                if let Some(brace_end) = p.find('}') {
                    let suffix = &p[brace_end + 1..];
                    if !suffix.is_empty() {
                        // Path part must end with this suffix
                        if !path_part.ends_with(suffix) {
                            return false;
                        }
                    }
                }
                // Placeholder part matches
                continue;
            }
            if *p != *path_part && !path_part.contains(p) {
                return false;
            }
        }
        return true;
    }

    false
}

/// Extract a string value using JSONPath
fn extract_string(body: &Value, path_value: &Value) -> Option<String> {
    let path_str = path_value.as_str()?;

    // Handle special case for path parameters
    if path_str.starts_with('{') && path_str.ends_with('}') {
        return None; // Path parameter, handled elsewhere
    }

    // Use jsonpath_lib selector
    let results = jsonpath_lib::select(body, path_str).ok()?;

    results.first().and_then(|v| v.as_str().map(String::from))
}

/// Extract a boolean value using JSONPath
fn extract_bool(body: &Value, path_value: &Value) -> Option<bool> {
    let path_str = path_value.as_str()?;

    let results = jsonpath_lib::select(body, path_str).ok()?;

    results.first().and_then(|v| v.as_bool())
}

/// Extract a u64 value using JSONPath
fn extract_u64(body: &Value, path_value: &Value) -> Option<u64> {
    let path_str = path_value.as_str()?;

    let results = jsonpath_lib::select(body, path_str).ok()?;

    results.first().and_then(|v| v.as_u64())
}

/// Extract messages from body
fn extract_messages(body: &Value, path: Option<&Value>) -> Vec<Message> {
    let messages_value = if let Some(path_value) = path {
        if let Some(path_str) = path_value.as_str() {
            if let Ok(results) = jsonpath_lib::select(body, path_str) {
                results.first().and_then(|v| {
                    if v.is_array() {
                        Some((*v).clone())
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        } else {
            None
        }
    } else {
        // Default path
        body.get("messages").cloned()
    };

    messages_value
        .and_then(|m| m.as_array().cloned())
        .map(|arr| arr.iter().map(parse_single_message).collect())
        .unwrap_or_default()
}

/// Parse a single message
fn parse_single_message(msg: &Value) -> Message {
    let role = msg
        .get("role")
        .and_then(|r| r.as_str())
        .map(parse_role)
        .unwrap_or(MessageRole::User);

    let content = msg.get("content");
    let content_str = content.and_then(|c| c.as_str());

    Message {
        role,
        content: content_str.map(|s| MessageContent::Text(s.to_string())),
        content_hash: content_str.map(hash_content),
        content_length: content_str.map(|s| s.len()),
        has_images: detect_images(msg),
        image_count: count_images(msg),
        tool_call_id: msg
            .get("tool_call_id")
            .and_then(|t| t.as_str())
            .map(String::from),
        name: msg.get("name").and_then(|n| n.as_str()).map(String::from),
    }
}

/// Parse role string to MessageRole
fn parse_role(role: &str) -> MessageRole {
    match role.to_lowercase().as_str() {
        "system" => MessageRole::System,
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "tool" => MessageRole::Tool,
        "function" => MessageRole::Function,
        _ => MessageRole::User,
    }
}

/// Extract tools from body
fn extract_tools(body: &Value, path: Option<&Value>) -> Vec<ToolDefinition> {
    let tools_value = if let Some(path_value) = path {
        if let Some(path_str) = path_value.as_str() {
            if let Ok(results) = jsonpath_lib::select(body, path_str) {
                results.first().and_then(|v| {
                    if v.is_array() {
                        Some((*v).clone())
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        } else {
            None
        }
    } else {
        body.get("tools").cloned()
    };

    tools_value
        .and_then(|t| t.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|tool| {
                    let name = tool
                        .get("function")
                        .and_then(|f| f.get("name"))
                        .or_else(|| tool.get("name"))
                        .and_then(|n| n.as_str())?;

                    Some(ToolDefinition {
                        name: name.to_string(),
                        tool_type: Some(ToolType::Function),
                        description: tool
                            .get("function")
                            .and_then(|f| f.get("description"))
                            .and_then(|d| d.as_str())
                            .map(String::from),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Extract model parameters
fn extract_parameters(body: &Value, extraction: &HashMap<String, Value>) -> ModelParameters {
    ModelParameters {
        temperature: extraction
            .get("temperature")
            .and_then(|path| extract_string(body, path))
            .and_then(|s| s.parse().ok())
            .or_else(|| body.get("temperature").and_then(|t| t.as_f64())),
        top_p: extraction
            .get("top_p")
            .and_then(|path| extract_string(body, path))
            .and_then(|s| s.parse().ok())
            .or_else(|| body.get("top_p").and_then(|t| t.as_f64())),
        max_tokens: extraction
            .get("max_tokens")
            .and_then(|path| extract_u64(body, path))
            .or_else(|| body.get("max_tokens").and_then(|t| t.as_u64())),
        frequency_penalty: body.get("frequency_penalty").and_then(|t| t.as_f64()),
        presence_penalty: body.get("presence_penalty").and_then(|t| t.as_f64()),
        stop: body
            .get("stop")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Extract usage information
fn extract_usage(body: &Value, usage_rules: Option<&Value>) -> Option<Usage> {
    // Try extraction rules first
    if let Some(rules) = usage_rules {
        if let Some(rules_obj) = rules.as_object() {
            return Some(Usage {
                prompt_tokens: rules_obj
                    .get("prompt_tokens")
                    .or(rules_obj.get("input_tokens"))
                    .and_then(|path| extract_u64(body, path)),
                completion_tokens: rules_obj
                    .get("completion_tokens")
                    .or(rules_obj.get("output_tokens"))
                    .and_then(|path| extract_u64(body, path)),
                total_tokens: rules_obj
                    .get("total_tokens")
                    .and_then(|path| extract_u64(body, path)),
                cached_tokens: rules_obj
                    .get("cached_tokens")
                    .or(rules_obj.get("cache_read_input_tokens"))
                    .and_then(|path| extract_u64(body, path)),
                reasoning_tokens: rules_obj
                    .get("reasoning_tokens")
                    .and_then(|path| extract_u64(body, path)),
                input_cost_usd: None,
                output_cost_usd: None,
                total_cost_usd: None,
            });
        }
    }

    // Default: try standard locations
    let u = body.get("usage")?;
    Some(Usage {
        prompt_tokens: u
            .get("prompt_tokens")
            .or(u.get("input_tokens"))
            .and_then(|t| t.as_u64()),
        completion_tokens: u
            .get("completion_tokens")
            .or(u.get("output_tokens"))
            .and_then(|t| t.as_u64()),
        total_tokens: u.get("total_tokens").and_then(|t| t.as_u64()),
        cached_tokens: u.get("cached_tokens").and_then(|t| t.as_u64()),
        reasoning_tokens: u
            .get("output_tokens_details")
            .and_then(|d| d.get("reasoning_tokens"))
            .and_then(|t| t.as_u64()),
        input_cost_usd: None,
        output_cost_usd: None,
        total_cost_usd: None,
    })
}

/// Extract choices from response
fn extract_choices(body: &Value) -> Vec<Choice> {
    // OpenAI style
    if let Some(choices) = body.get("choices").and_then(|c| c.as_array()) {
        return choices
            .iter()
            .enumerate()
            .map(|(idx, choice)| {
                let message = choice.get("message").map(parse_single_message);
                let finish_reason = choice
                    .get("finish_reason")
                    .and_then(|f| f.as_str())
                    .and_then(parse_finish_reason);

                Choice {
                    index: idx,
                    message,
                    finish_reason,
                }
            })
            .collect();
    }

    // Anthropic style
    if let Some(content) = body.get("content").and_then(|c| c.as_array()) {
        let mut text_content = String::new();
        for block in content {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    text_content.push_str(text);
                }
            }
        }

        let finish_reason = body
            .get("stop_reason")
            .and_then(|r| r.as_str())
            .map(|r| match r {
                "end_turn" => FinishReason::Stop,
                "max_tokens" => FinishReason::Length,
                "tool_use" => FinishReason::ToolCalls,
                _ => FinishReason::Other,
            });

        return vec![Choice {
            index: 0,
            message: Some(Message {
                role: MessageRole::Assistant,
                content: if text_content.is_empty() {
                    None
                } else {
                    Some(MessageContent::Text(text_content.clone()))
                },
                content_hash: if text_content.is_empty() {
                    None
                } else {
                    Some(hash_content(&text_content))
                },
                content_length: Some(text_content.len()),
                has_images: None,
                image_count: None,
                tool_call_id: None,
                name: None,
            }),
            finish_reason,
        }];
    }

    Vec::new()
}

/// Extract tool calls from response
fn extract_tool_calls(body: &Value) -> Vec<ToolCall> {
    // OpenAI style
    if let Some(tool_calls) = body
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("tool_calls"))
        .and_then(|tc| tc.as_array())
    {
        return tool_calls
            .iter()
            .filter_map(|tc| {
                let id = tc.get("id").and_then(|i| i.as_str()).map(String::from);
                let name = tc
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())?;
                let arguments = tc
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(|a| a.as_str())
                    .map(|s| ToolArguments::String(s.to_string()));

                Some(ToolCall {
                    id,
                    name: name.to_string(),
                    tool_type: Some(ToolType::Function),
                    arguments,
                    arguments_hash: None,
                })
            })
            .collect();
    }

    // Anthropic style
    if let Some(content) = body.get("content").and_then(|c| c.as_array()) {
        return content
            .iter()
            .filter_map(|block| {
                if block.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
                    return None;
                }

                let id = block.get("id").and_then(|i| i.as_str()).map(String::from);
                let name = block.get("name").and_then(|n| n.as_str())?;
                let input = block.get("input");

                Some(ToolCall {
                    id,
                    name: name.to_string(),
                    tool_type: Some(ToolType::Function),
                    arguments: input.map(|i| ToolArguments::String(i.to_string())),
                    arguments_hash: None,
                })
            })
            .collect();
    }

    Vec::new()
}

/// Extract thinking/reasoning blocks from response
fn extract_thinking_block(
    body: &Value,
    provider_id: &str,
    usage: Option<&Usage>,
) -> Option<ThinkingBlock> {
    // Check for reasoning tokens (OpenAI o1-style)
    let reasoning_tokens = usage.and_then(|u| u.reasoning_tokens);

    // Check for Anthropic extended thinking
    // Claude returns thinking blocks as content with type: "thinking"
    if let Some(content) = body.get("content").and_then(|c| c.as_array()) {
        for block in content {
            if block.get("type").and_then(|t| t.as_str()) == Some("thinking") {
                let thinking_text = block.get("thinking").and_then(|t| t.as_str());

                if let Some(text) = thinking_text {
                    return Some(ThinkingBlock {
                        enabled: Some(true),
                        content: Some(MessageContent::Text(text.to_string())),
                        content_hash: Some(hash_content(text)),
                        content_length: Some(text.len()),
                        tokens: None,
                        duration_ms: None,
                        mode: Some(ThinkingMode::ExtendedThinking),
                    });
                }
            }
        }
    }

    // Check for OpenAI reasoning tokens
    if let Some(tokens) = reasoning_tokens {
        if tokens > 0 {
            // OpenAI doesn't expose the reasoning content, only token count
            return Some(ThinkingBlock {
                enabled: Some(true),
                content: None, // OpenAI doesn't expose reasoning content
                content_hash: None,
                content_length: None,
                tokens: Some(tokens),
                duration_ms: None,
                mode: Some(ThinkingMode::Reasoning),
            });
        }
    }

    // Check for reasoning_content in choices (some models expose this)
    if let Some(reasoning) = body
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("reasoning_content"))
        .and_then(|r| r.as_str())
    {
        return Some(ThinkingBlock {
            enabled: Some(true),
            content: Some(MessageContent::Text(reasoning.to_string())),
            content_hash: Some(hash_content(reasoning)),
            content_length: Some(reasoning.len()),
            tokens: reasoning_tokens,
            duration_ms: None,
            mode: Some(ThinkingMode::Reasoning),
        });
    }

    // Check for DeepSeek R1-style thinking
    if provider_id == "deepseek" {
        if let Some(thinking) = body
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("thinking"))
            .and_then(|t| t.as_str())
        {
            return Some(ThinkingBlock {
                enabled: Some(true),
                content: Some(MessageContent::Text(thinking.to_string())),
                content_hash: Some(hash_content(thinking)),
                content_length: Some(thinking.len()),
                tokens: None,
                duration_ms: None,
                mode: Some(ThinkingMode::DeepThinking),
            });
        }
    }

    None
}

/// Parse finish reason string
fn parse_finish_reason(reason: &str) -> Option<FinishReason> {
    match reason {
        "stop" | "end_turn" | "stop_sequence" => Some(FinishReason::Stop),
        "length" | "max_tokens" => Some(FinishReason::Length),
        "tool_calls" | "function_call" | "tool_use" => Some(FinishReason::ToolCalls),
        "content_filter" => Some(FinishReason::ContentFilter),
        "error" => Some(FinishReason::Error),
        _ => Some(FinishReason::Other),
    }
}

/// Hash content for correlation
fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{}", hex::encode(&hasher.finalize()[..8]))
}

/// Detect if message contains images
fn detect_images(msg: &Value) -> Option<bool> {
    // Check for array content with image_url type
    if let Some(content) = msg.get("content").and_then(|c| c.as_array()) {
        let has_image = content
            .iter()
            .any(|part| part.get("type").and_then(|t| t.as_str()) == Some("image_url"));
        return Some(has_image);
    }
    None
}

/// Count images in message
fn count_images(msg: &Value) -> Option<usize> {
    if let Some(content) = msg.get("content").and_then(|c| c.as_array()) {
        let count = content
            .iter()
            .filter(|part| part.get("type").and_then(|t| t.as_str()) == Some("image_url"))
            .count();
        if count > 0 {
            return Some(count);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oisp_core::spec::OispSpecBundle;

    fn test_registry() -> Arc<DynamicProviderRegistry> {
        let bundle = Arc::new(OispSpecBundle::embedded());
        Arc::new(DynamicProviderRegistry::new(bundle))
    }

    #[test]
    fn test_parse_openai_request() {
        let registry = test_registry();
        let parser = SpecDrivenParser::new(registry);

        let body: Value = serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello!"}
            ],
            "temperature": 0.7,
            "max_tokens": 100,
            "stream": true
        });

        let request = parser
            .parse_request("openai", "/v1/chat/completions", &body)
            .unwrap();

        assert_eq!(request.model.as_ref().unwrap().id, "gpt-4");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.streaming, Some(true));
        assert_eq!(request.has_system_prompt, Some(true));
    }

    #[test]
    fn test_parse_anthropic_request() {
        let registry = test_registry();
        let parser = SpecDrivenParser::new(registry);

        let body: Value = serde_json::json!({
            "model": "claude-3-opus-20240229",
            "messages": [
                {"role": "user", "content": "Hello!"}
            ],
            "max_tokens": 1024,
            "stream": false
        });

        let request = parser
            .parse_request("anthropic", "/v1/messages", &body)
            .unwrap();

        assert!(request.model.as_ref().unwrap().id.starts_with("claude"));
    }

    #[test]
    fn test_pattern_matches() {
        assert!(pattern_matches("/v1/messages", "/v1/messages"));
        assert!(pattern_matches(
            "/v1/chat/completions",
            "/v1/chat/completions"
        ));
        assert!(pattern_matches(
            "/v1beta/models/{model}:generateContent",
            "/v1beta/models/gemini-pro:generateContent"
        ));
    }
}
