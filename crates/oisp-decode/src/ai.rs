//! AI request/response parsing

use oisp_core::events::{
    AgentContext, AiRequestData, AiResponseData, Choice, ConversationContext, FinishReason,
    Message, MessageContent, MessageRole, ModelInfo, ModelParameters, ProviderInfo, RequestType,
    ThinkingBlock, ThinkingMode, ToolArguments, ToolCall, ToolDefinition, ToolType, Usage,
};
use oisp_core::providers::Provider;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Parse an AI request from JSON body
pub fn parse_ai_request(body: &Value, provider: Provider, endpoint: &str) -> Option<AiRequestData> {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .map(|id| ModelInfo {
            id: id.to_string(),
            name: None,
            family: extract_model_family(id),
            version: None,
            capabilities: None,
            context_window: None,
            max_output_tokens: None,
        });

    let messages = parse_messages(body.get("messages"));
    let tools = parse_tools(body.get("tools"));

    let streaming = body
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    let parameters = parse_parameters(body);

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
            name: format!("{:?}", provider).to_lowercase(),
            endpoint: Some(endpoint.to_string()),
            region: None,
            organization_id: None,
            project_id: None,
        }),
        model,
        auth: None,
        request_type: Some(detect_request_type(body)),
        streaming: Some(streaming),
        messages: messages.clone(),
        messages_count: Some(messages.len()),
        has_system_prompt: Some(has_system_prompt),
        system_prompt_hash,
        tools: tools.clone(),
        tools_count: Some(tools.len()),
        tool_choice: body.get("tool_choice").map(|tc| format!("{}", tc)),
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

/// Parse an AI response from JSON body
pub fn parse_ai_response(
    body: &Value,
    request_id: &str,
    provider: Provider,
) -> Option<AiResponseData> {
    let choices = body
        .get("choices")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
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
                .collect()
        })
        .unwrap_or_default();

    let tool_calls = extract_tool_calls(body);
    let usage = parse_usage(body.get("usage"));

    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .map(|id| ModelInfo {
            id: id.to_string(),
            name: None,
            family: extract_model_family(id),
            version: None,
            capabilities: None,
            context_window: None,
            max_output_tokens: None,
        });

    // Extract thinking/reasoning blocks
    let thinking = extract_thinking_block(
        body,
        &format!("{:?}", provider).to_lowercase(),
        usage.as_ref(),
    );

    Some(AiResponseData {
        request_id: request_id.to_string(),
        provider_request_id: body.get("id").and_then(|i| i.as_str()).map(String::from),
        provider: Some(ProviderInfo {
            name: format!("{:?}", provider).to_lowercase(),
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
        finish_reason: body
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("finish_reason"))
            .and_then(|f| f.as_str())
            .and_then(parse_finish_reason),
        thinking,
    })
}

fn parse_messages(messages: Option<&Value>) -> Vec<Message> {
    messages
        .and_then(|m| m.as_array())
        .map(|arr| arr.iter().map(parse_single_message).collect())
        .unwrap_or_default()
}

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
        has_images: None,
        image_count: None,
        tool_call_id: msg
            .get("tool_call_id")
            .and_then(|t| t.as_str())
            .map(String::from),
        name: msg.get("name").and_then(|n| n.as_str()).map(String::from),
    }
}

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

fn parse_tools(tools: Option<&Value>) -> Vec<ToolDefinition> {
    tools
        .and_then(|t| t.as_array())
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

fn extract_tool_calls(body: &Value) -> Vec<ToolCall> {
    body.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("tool_calls"))
        .and_then(|tc| tc.as_array())
        .map(|arr| {
            arr.iter()
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
                .collect()
        })
        .unwrap_or_default()
}

fn parse_usage(usage: Option<&Value>) -> Option<Usage> {
    let u = usage?;
    Some(Usage {
        prompt_tokens: u.get("prompt_tokens").and_then(|t| t.as_u64()),
        completion_tokens: u.get("completion_tokens").and_then(|t| t.as_u64()),
        total_tokens: u.get("total_tokens").and_then(|t| t.as_u64()),
        cached_tokens: u.get("cached_tokens").and_then(|t| t.as_u64()),
        reasoning_tokens: u.get("reasoning_tokens").and_then(|t| t.as_u64()),
        input_cost_usd: None,
        output_cost_usd: None,
        total_cost_usd: None,
    })
}

fn parse_parameters(body: &Value) -> ModelParameters {
    ModelParameters {
        temperature: body.get("temperature").and_then(|t| t.as_f64()),
        top_p: body.get("top_p").and_then(|t| t.as_f64()),
        max_tokens: body.get("max_tokens").and_then(|t| t.as_u64()),
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

fn parse_finish_reason(reason: &str) -> Option<FinishReason> {
    match reason {
        "stop" => Some(FinishReason::Stop),
        "length" => Some(FinishReason::Length),
        "tool_calls" | "function_call" => Some(FinishReason::ToolCalls),
        "content_filter" => Some(FinishReason::ContentFilter),
        "error" => Some(FinishReason::Error),
        _ => Some(FinishReason::Other),
    }
}

fn detect_request_type(body: &Value) -> RequestType {
    if body.get("messages").is_some() {
        RequestType::Chat
    } else if body.get("prompt").is_some() {
        RequestType::Completion
    } else if body.get("input").is_some() {
        RequestType::Embedding
    } else {
        RequestType::Other
    }
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

fn extract_model_family(model_id: &str) -> Option<String> {
    // Extract family from model ID
    if model_id.starts_with("gpt-4") {
        Some("gpt-4".to_string())
    } else if model_id.starts_with("gpt-3.5") {
        Some("gpt-3.5".to_string())
    } else if model_id.starts_with("claude-3") {
        Some("claude-3".to_string())
    } else if model_id.starts_with("claude-2") {
        Some("claude-2".to_string())
    } else if model_id.starts_with("gemini") {
        Some("gemini".to_string())
    } else {
        None
    }
}

fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{}", hex::encode(&hasher.finalize()[..8]))
}

/// Detect if a request body looks like an AI/LLM request
pub fn is_ai_request(body: &Value) -> bool {
    // Check for common AI API patterns
    let has_messages = body.get("messages").is_some();
    let has_model = body.get("model").is_some();
    let has_prompt = body.get("prompt").is_some();

    (has_prompt || has_messages) && has_model
}

/// Detect provider from request/response shape
pub fn detect_provider_from_body(body: &Value) -> Option<Provider> {
    // Anthropic uses "claude" models and has specific fields
    if let Some(model) = body.get("model").and_then(|m| m.as_str()) {
        if model.starts_with("claude") {
            return Some(Provider::Anthropic);
        }
        if model.starts_with("gpt") || model.starts_with("o1") || model.starts_with("chatgpt") {
            return Some(Provider::OpenAI);
        }
        if model.starts_with("gemini") {
            return Some(Provider::Google);
        }
        if model.starts_with("llama")
            || model.starts_with("mixtral")
            || model.starts_with("mistral")
        {
            // Could be various providers, check other hints
        }
    }

    // Check for Anthropic-specific fields
    if body.get("anthropic_version").is_some() {
        return Some(Provider::Anthropic);
    }

    // Check for OpenAI response structure
    if body.get("choices").is_some()
        && body.get("object").and_then(|o| o.as_str()) == Some("chat.completion")
    {
        return Some(Provider::OpenAI);
    }

    // Check for Anthropic response structure
    if body.get("content").is_some() && body.get("type").and_then(|t| t.as_str()) == Some("message")
    {
        return Some(Provider::Anthropic);
    }

    None
}

/// Parse Anthropic-style AI request
pub fn parse_anthropic_request(body: &Value, endpoint: &str) -> Option<AiRequestData> {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .map(|id| ModelInfo {
            id: id.to_string(),
            name: None,
            family: extract_model_family(id),
            version: None,
            capabilities: None,
            context_window: None,
            max_output_tokens: None,
        });

    // Anthropic uses a different message structure
    let messages: Vec<Message> = body
        .get("messages")
        .and_then(|m| m.as_array())
        .map(|arr| arr.iter().map(parse_single_message).collect())
        .unwrap_or_default();

    let system = body.get("system").and_then(|s| s.as_str());
    let has_system_prompt = system.is_some();
    let system_prompt_hash = system.map(hash_content);

    let streaming = body
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    let tools = parse_tools(body.get("tools"));

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
            name: "anthropic".to_string(),
            endpoint: Some(endpoint.to_string()),
            region: None,
            organization_id: None,
            project_id: None,
        }),
        model,
        auth: None,
        request_type: Some(RequestType::Chat),
        streaming: Some(streaming),
        messages: messages.clone(),
        messages_count: Some(messages.len()),
        has_system_prompt: Some(has_system_prompt),
        system_prompt_hash,
        tools,
        tools_count: body
            .get("tools")
            .and_then(|t| t.as_array())
            .map(|a| a.len()),
        tool_choice: body.get("tool_choice").map(|tc| format!("{}", tc)),
        parameters: Some(ModelParameters {
            temperature: body.get("temperature").and_then(|t| t.as_f64()),
            top_p: body.get("top_p").and_then(|t| t.as_f64()),
            max_tokens: body.get("max_tokens").and_then(|t| t.as_u64()),
            frequency_penalty: None,
            presence_penalty: None,
            stop: body
                .get("stop_sequences")
                .and_then(|s| s.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        }),
        has_rag_context: None,
        has_images: None,
        image_count: None,
        estimated_tokens: None,
        conversation,
        agent,
    })
}

/// Parse Anthropic-style AI response
pub fn parse_anthropic_response(body: &Value, request_id: &str) -> Option<AiResponseData> {
    let content = body.get("content").and_then(|c| c.as_array())?;

    let mut text_content = String::new();
    let mut tool_calls = Vec::new();

    for block in content {
        let block_type = block.get("type").and_then(|t| t.as_str());
        match block_type {
            Some("text") => {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    text_content.push_str(text);
                }
            }
            Some("tool_use") => {
                let id = block.get("id").and_then(|i| i.as_str()).map(String::from);
                if let Some(name) = block.get("name").and_then(|n| n.as_str()) {
                    let input = block.get("input");
                    tool_calls.push(ToolCall {
                        id,
                        name: name.to_string(),
                        tool_type: Some(ToolType::Function),
                        arguments: input.map(|i| ToolArguments::String(i.to_string())),
                        arguments_hash: None,
                    });
                }
            }
            _ => {}
        }
    }

    let finish_reason = body
        .get("stop_reason")
        .and_then(|r| r.as_str())
        .map(|r| match r {
            "end_turn" => FinishReason::Stop,
            "max_tokens" => FinishReason::Length,
            "tool_use" => FinishReason::ToolCalls,
            "stop_sequence" => FinishReason::Stop,
            _ => FinishReason::Other,
        });

    let usage = body.get("usage").map(|u| Usage {
        prompt_tokens: u.get("input_tokens").and_then(|t| t.as_u64()),
        completion_tokens: u.get("output_tokens").and_then(|t| t.as_u64()),
        total_tokens: None,
        cached_tokens: u.get("cache_read_input_tokens").and_then(|t| t.as_u64()),
        reasoning_tokens: None,
        input_cost_usd: None,
        output_cost_usd: None,
        total_cost_usd: None,
    });

    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .map(|id| ModelInfo {
            id: id.to_string(),
            name: None,
            family: extract_model_family(id),
            version: None,
            capabilities: None,
            context_window: None,
            max_output_tokens: None,
        });

    // Extract thinking/reasoning blocks (Anthropic extended thinking)
    let thinking = extract_thinking_block(body, "anthropic", usage.as_ref());

    Some(AiResponseData {
        request_id: request_id.to_string(),
        provider_request_id: body.get("id").and_then(|i| i.as_str()).map(String::from),
        provider: Some(ProviderInfo {
            name: "anthropic".to_string(),
            endpoint: None,
            region: None,
            organization_id: None,
            project_id: None,
        }),
        model,
        status_code: None,
        success: Some(true),
        error: None,
        choices: vec![Choice {
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
        }],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ai_request_openai() {
        let body: Value = serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });
        assert!(is_ai_request(&body));
    }

    #[test]
    fn test_is_ai_request_anthropic() {
        let body: Value = serde_json::json!({
            "model": "claude-3-opus-20240229",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "max_tokens": 1024
        });
        assert!(is_ai_request(&body));
    }

    #[test]
    fn test_is_ai_request_embedding() {
        let body: Value = serde_json::json!({
            "model": "text-embedding-ada-002",
            "input": "Hello world"
        });
        // This should NOT be detected as AI request (no messages)
        assert!(!is_ai_request(&body));
    }

    #[test]
    fn test_detect_provider_openai() {
        let body: Value = serde_json::json!({
            "model": "gpt-4-turbo",
            "choices": [{"message": {"content": "Hi"}}]
        });
        assert_eq!(detect_provider_from_body(&body), Some(Provider::OpenAI));
    }

    #[test]
    fn test_detect_provider_anthropic() {
        let body: Value = serde_json::json!({
            "model": "claude-3-sonnet",
            "content": [{"type": "text", "text": "Hi"}],
            "type": "message"
        });
        assert_eq!(detect_provider_from_body(&body), Some(Provider::Anthropic));
    }

    #[test]
    fn test_parse_openai_request() {
        let body: Value = serde_json::json!({
            "model": "gpt-4-turbo",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello!"}
            ],
            "temperature": 0.7,
            "max_tokens": 100,
            "stream": true
        });

        let request = parse_ai_request(
            &body,
            Provider::OpenAI,
            "https://api.openai.com/v1/chat/completions",
        )
        .unwrap();

        assert_eq!(request.model.as_ref().unwrap().id, "gpt-4-turbo");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.streaming, Some(true));
        assert_eq!(request.has_system_prompt, Some(true));
        assert!(request.system_prompt_hash.is_some());
        assert_eq!(request.parameters.as_ref().unwrap().temperature, Some(0.7));
    }

    #[test]
    fn test_parse_openai_response() {
        let body: Value = serde_json::json!({
            "id": "chatcmpl-abc123",
            "object": "chat.completion",
            "model": "gpt-4-turbo",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        });

        let response = parse_ai_response(&body, "test-request-id", Provider::OpenAI).unwrap();

        assert_eq!(
            response.provider_request_id,
            Some("chatcmpl-abc123".to_string())
        );
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.finish_reason, Some(FinishReason::Stop));
        assert_eq!(response.usage.as_ref().unwrap().prompt_tokens, Some(10));
        assert_eq!(response.usage.as_ref().unwrap().completion_tokens, Some(8));
    }

    #[test]
    fn test_parse_openai_tool_call_response() {
        let body: Value = serde_json::json!({
            "id": "chatcmpl-abc123",
            "model": "gpt-4-turbo",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"San Francisco\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });

        let response = parse_ai_response(&body, "test-request-id", Provider::OpenAI).unwrap();

        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].name, "get_weather");
        assert_eq!(response.finish_reason, Some(FinishReason::ToolCalls));
    }

    #[test]
    fn test_parse_anthropic_request() {
        let body: Value = serde_json::json!({
            "model": "claude-3-opus-20240229",
            "system": "You are a helpful assistant.",
            "messages": [
                {"role": "user", "content": "Hello!"}
            ],
            "max_tokens": 1024,
            "temperature": 0.8,
            "stream": false
        });

        let request =
            parse_anthropic_request(&body, "https://api.anthropic.com/v1/messages").unwrap();

        assert_eq!(request.model.as_ref().unwrap().id, "claude-3-opus-20240229");
        assert_eq!(request.has_system_prompt, Some(true));
        assert!(request.system_prompt_hash.is_some());
        assert_eq!(request.parameters.as_ref().unwrap().max_tokens, Some(1024));
    }

    #[test]
    fn test_parse_anthropic_response() {
        let body: Value = serde_json::json!({
            "id": "msg_abc123",
            "type": "message",
            "role": "assistant",
            "model": "claude-3-opus-20240229",
            "content": [
                {"type": "text", "text": "Hello! How can I help you today?"}
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 25,
                "output_tokens": 15
            }
        });

        let response = parse_anthropic_response(&body, "test-request-id").unwrap();

        assert_eq!(response.provider_request_id, Some("msg_abc123".to_string()));
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.finish_reason, Some(FinishReason::Stop));
        assert_eq!(response.usage.as_ref().unwrap().prompt_tokens, Some(25));
    }

    #[test]
    fn test_parse_anthropic_tool_response() {
        let body: Value = serde_json::json!({
            "id": "msg_abc123",
            "type": "message",
            "model": "claude-3-opus-20240229",
            "content": [
                {"type": "tool_use", "id": "toolu_abc", "name": "calculator", "input": {"expression": "2+2"}}
            ],
            "stop_reason": "tool_use"
        });

        let response = parse_anthropic_response(&body, "test-request-id").unwrap();

        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].name, "calculator");
        assert_eq!(response.finish_reason, Some(FinishReason::ToolCalls));
    }

    #[test]
    fn test_detect_request_type() {
        assert_eq!(
            detect_request_type(&serde_json::json!({"messages": []})),
            RequestType::Chat
        );
        assert_eq!(
            detect_request_type(&serde_json::json!({"prompt": "Hello"})),
            RequestType::Completion
        );
        assert_eq!(
            detect_request_type(&serde_json::json!({"input": "Hello"})),
            RequestType::Embedding
        );
        assert_eq!(
            detect_request_type(&serde_json::json!({})),
            RequestType::Other
        );
    }

    #[test]
    fn test_extract_model_family() {
        assert_eq!(
            extract_model_family("gpt-4-turbo-preview"),
            Some("gpt-4".to_string())
        );
        assert_eq!(
            extract_model_family("gpt-3.5-turbo"),
            Some("gpt-3.5".to_string())
        );
        assert_eq!(
            extract_model_family("claude-3-opus-20240229"),
            Some("claude-3".to_string())
        );
        assert_eq!(
            extract_model_family("claude-2.1"),
            Some("claude-2".to_string())
        );
        assert_eq!(
            extract_model_family("gemini-pro"),
            Some("gemini".to_string())
        );
        assert_eq!(extract_model_family("some-custom-model"), None);
    }

    #[test]
    fn test_parse_messages() {
        let messages_json = serde_json::json!([
            {"role": "system", "content": "You are helpful."},
            {"role": "user", "content": "Hi!"},
            {"role": "assistant", "content": "Hello!"},
            {"role": "tool", "content": "result", "tool_call_id": "call_123"}
        ]);

        let messages = parse_messages(Some(&messages_json));

        assert_eq!(messages.len(), 4);
        assert!(matches!(messages[0].role, MessageRole::System));
        assert!(matches!(messages[1].role, MessageRole::User));
        assert!(matches!(messages[2].role, MessageRole::Assistant));
        assert!(matches!(messages[3].role, MessageRole::Tool));
        assert_eq!(messages[3].tool_call_id, Some("call_123".to_string()));
    }

    #[test]
    fn test_parse_tools() {
        let tools_json = serde_json::json!([
            {
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get the current weather"
                }
            }
        ]);

        let tools = parse_tools(Some(&tools_json));

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "get_weather");
        assert_eq!(
            tools[0].description,
            Some("Get the current weather".to_string())
        );
    }
}
