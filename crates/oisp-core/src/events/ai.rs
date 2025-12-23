//! AI-related events - requests, responses, streaming, embeddings

use super::envelope::EventEnvelope;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AI request event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequestEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    /// Request-specific data
    #[serde(flatten)]
    pub data: AiRequestData,
}

/// AI request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequestData {
    /// Unique request ID for correlation
    pub request_id: String,
    
    /// Provider information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderInfo>,
    
    /// Model information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelInfo>,
    
    /// Authentication information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthInfo>,
    
    /// Request type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_type: Option<RequestType>,
    
    /// Whether streaming was requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,
    
    /// Messages in the request
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<Message>,
    
    /// Number of messages (when messages are redacted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages_count: Option<usize>,
    
    /// Whether a system prompt was included
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_system_prompt: Option<bool>,
    
    /// Hash of system prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt_hash: Option<String>,
    
    /// Tools available to the model
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    
    /// Number of tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools_count: Option<usize>,
    
    /// Tool choice setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    
    /// Model parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<ModelParameters>,
    
    /// Whether RAG context was detected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_rag_context: Option<bool>,
    
    /// Whether images were included
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_images: Option<bool>,
    
    /// Number of images
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_count: Option<usize>,
    
    /// Estimated token count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_tokens: Option<u64>,
}

/// AI response event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponseEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    #[serde(flatten)]
    pub data: AiResponseData,
}

/// AI response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponseData {
    /// Links back to the request
    pub request_id: String,
    
    /// Request ID from provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_request_id: Option<String>,
    
    /// Provider information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderInfo>,
    
    /// Model information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelInfo>,
    
    /// HTTP status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    
    /// Whether the request succeeded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    
    /// Error information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
    
    /// Response choices
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Choice>,
    
    /// Tool calls made by the model
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    
    /// Number of tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls_count: Option<usize>,
    
    /// Token usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    
    /// Total latency in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    
    /// Time to first token (streaming)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_to_first_token_ms: Option<u64>,
    
    /// Whether response was cached
    #[serde(skip_serializing_if = "Option::is_none")]
    pub was_cached: Option<bool>,
    
    /// Finish reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
}

/// Streaming chunk event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStreamingChunkEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    /// Links back to the request
    pub request_id: String,
    
    /// Chunk index
    pub chunk_index: usize,
    
    /// Delta content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<ChunkDelta>,
    
    /// Finish reason (on final chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
}

/// Chunk delta content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkDelta {
    /// Content delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    
    /// Role (usually only in first chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    
    /// Tool call deltas
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
}

/// Embedding event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEmbeddingEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    /// Provider information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderInfo>,
    
    /// Model information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelInfo>,
    
    /// Number of inputs embedded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_count: Option<usize>,
    
    /// Total tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    
    /// Embedding dimensions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
    
    /// Latency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

/// Provider information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider name
    pub name: String,
    
    /// API endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    
    /// Region
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    
    /// Organization ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    
    /// Project ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model ID
    pub id: String,
    
    /// Human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// Model family
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    
    /// Version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    
    /// Capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ModelCapabilities>,
    
    /// Context window size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    
    /// Max output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
}

/// Model capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_calling: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_mode: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_messages: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_execution: Option<bool>,
}

/// Authentication information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthInfo {
    /// Auth type
    #[serde(rename = "type")]
    pub auth_type: AuthType,
    
    /// Account type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_type: Option<AccountType>,
    
    /// API key prefix (for identification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_prefix: Option<String>,
    
    /// Hash of full API key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_hash: Option<String>,
}

/// Authentication type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    ApiKey,
    OAuth,
    ServiceAccount,
    Session,
    None,
    Unknown,
}

/// Account type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Personal,
    Corporate,
    Shared,
    Unknown,
}

/// Message in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role
    pub role: MessageRole,
    
    /// Content (may be redacted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,
    
    /// Content hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    
    /// Content length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_length: Option<usize>,
    
    /// Whether contains images
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_images: Option<bool>,
    
    /// Image count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_count: Option<usize>,
    
    /// Tool call ID this responds to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    
    /// Function/tool name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
    Function,
}

/// Message content - can be plain text or redacted
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Plain text content
    Text(String),
    /// Redacted content
    Redacted(RedactedContent),
}

/// Marker for redacted content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactedContent {
    #[serde(rename = "$redacted")]
    pub redacted: RedactionInfo,
}

/// Redaction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionInfo {
    /// Why it was redacted
    pub reason: String,
    
    /// Detector that triggered it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detector: Option<String>,
    
    /// Original length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_length: Option<usize>,
    
    /// Hash of original
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    
    /// Safe preview
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    
    /// Redaction profile used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redaction_profile: Option<String>,
    
    /// What was found
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<Finding>,
}

/// A finding that triggered redaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    #[serde(rename = "type")]
    pub finding_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    
    /// Tool type
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<ToolType>,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Tool type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    Function,
    CodeInterpreter,
    FileSearch,
    ComputerUse,
    Other,
}

/// Tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool call ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    
    /// Tool/function name
    pub name: String,
    
    /// Tool type
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<ToolType>,
    
    /// Arguments (may be redacted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<ToolArguments>,
    
    /// Hash of arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments_hash: Option<String>,
}

/// Tool arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolArguments {
    /// Raw JSON string
    String(String),
    /// Parsed JSON object
    Object(HashMap<String, serde_json::Value>),
    /// Redacted
    Redacted(RedactedContent),
}

/// Model parameters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
}

/// Response choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// Choice index
    pub index: usize,
    
    /// Response message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    
    /// Finish reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
}

/// Why generation stopped
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Error,
    Other,
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
}

/// Error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Request type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestType {
    Chat,
    Completion,
    Embedding,
    Image,
    Audio,
    Moderation,
    Other,
}

