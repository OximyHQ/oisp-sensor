//! AI request/response parsing

use oisp_core::events::{
    AiRequestData, AiResponseData, Choice, FinishReason, Message, MessageContent, MessageRole,
    ModelInfo, ModelParameters, ProviderInfo, RequestType, ToolArguments, ToolCall, ToolDefinition,
    ToolType, Usage,
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
        if model.starts_with("gpt") || model.starts_with("o1") {
            return Some(Provider::OpenAI);
        }
        if model.starts_with("gemini") {
            return Some(Provider::Google);
        }
    }

    // Check for Anthropic-specific fields
    if body.get("anthropic_version").is_some() {
        return Some(Provider::Anthropic);
    }

    None
}
