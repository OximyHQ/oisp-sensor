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

    // --- Conversation Tracking ---
    /// Conversation context for multi-turn tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ConversationContext>,

    // --- Agentic Context ---
    /// Agent/SDK information (inferred from patterns)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentContext>,
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

    // --- Reasoning/Thinking ---
    /// Thinking/reasoning blocks (Claude extended thinking, OpenAI o1, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingBlock>,
}

/// Streaming chunk event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStreamingChunkEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,

    /// Chunk-specific data
    #[serde(flatten)]
    pub data: AiStreamingChunkData,
}

/// AI streaming chunk data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStreamingChunkData {
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

    /// Embedding-specific data
    #[serde(flatten)]
    pub data: AiEmbeddingData,
}

/// AI embedding data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEmbeddingData {
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

// =============================================================================
// Conversation Tracking
// =============================================================================

/// Conversation context for multi-turn tracking
///
/// We infer conversation state from message patterns:
/// - Message count growth indicates conversation continuation
/// - System prompt hash helps identify the same conversation
/// - Turn detection based on user/assistant message pairs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    /// Conversation ID (derived from system prompt hash + process)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,

    /// Current turn number in the conversation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_number: Option<usize>,

    /// Number of user messages seen so far
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_message_count: Option<usize>,

    /// Number of assistant messages seen so far
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_message_count: Option<usize>,

    /// Number of tool result messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result_count: Option<usize>,

    /// Whether this appears to be the first message in conversation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_first_turn: Option<bool>,

    /// Estimated conversation length (based on token count growth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_context_tokens: Option<u64>,

    /// How much of context window is used (0.0 - 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_utilization: Option<f64>,
}

impl ConversationContext {
    /// Create from message analysis
    pub fn from_messages(messages: &[Message], context_window: Option<u64>) -> Self {
        let user_count = messages
            .iter()
            .filter(|m| matches!(m.role, MessageRole::User))
            .count();
        let assistant_count = messages
            .iter()
            .filter(|m| matches!(m.role, MessageRole::Assistant))
            .count();
        let tool_count = messages
            .iter()
            .filter(|m| matches!(m.role, MessageRole::Tool | MessageRole::Function))
            .count();

        // Turn is typically user + assistant pair
        let turn_number = user_count;

        // Estimate tokens from content lengths
        let estimated_tokens: u64 = messages
            .iter()
            .filter_map(|m| m.content_length)
            .map(|len| (len as u64) / 4) // ~4 chars per token
            .sum();

        let context_utilization = context_window.map(|cw| {
            if cw > 0 {
                (estimated_tokens as f64) / (cw as f64)
            } else {
                0.0
            }
        });

        Self {
            conversation_id: None, // Set by caller based on process + system prompt hash
            turn_number: Some(turn_number),
            user_message_count: Some(user_count),
            assistant_message_count: Some(assistant_count),
            tool_result_count: Some(tool_count),
            is_first_turn: Some(user_count <= 1 && assistant_count == 0),
            estimated_context_tokens: Some(estimated_tokens),
            context_utilization,
        }
    }
}

// =============================================================================
// Thinking/Reasoning
// =============================================================================

/// Thinking/reasoning block for extended reasoning models
///
/// Captures:
/// - Claude extended thinking (<thinking> blocks)
/// - OpenAI o1/o3 reasoning (reasoning_tokens, reasoning_content)
/// - DeepSeek R1 thinking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingBlock {
    /// Whether thinking/reasoning was enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Thinking content (may be redacted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,

    /// Hash of thinking content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,

    /// Length of thinking content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_length: Option<usize>,

    /// Tokens used for thinking/reasoning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,

    /// Duration of thinking phase (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Model-specific thinking mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ThinkingMode>,
}

/// Thinking mode variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingMode {
    /// Claude extended thinking
    ExtendedThinking,
    /// OpenAI o1-style reasoning
    Reasoning,
    /// DeepSeek R1-style
    DeepThinking,
    /// Other/unknown
    Other,
}

// =============================================================================
// Agent Context (Inferred)
// =============================================================================

/// Agent/SDK context inferred from request patterns
///
/// We detect agent frameworks from patterns like:
/// - System prompt templates (e.g., "You are Claude, a helpful AI assistant")
/// - Tool naming conventions (e.g., "mcp_*", "langchain_*")
/// - Message structure patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    /// Detected agent framework
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework: Option<AgentFramework>,

    /// Agent name (from system prompt or tool patterns)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Agent version (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// MCP servers in use (detected from tool prefixes)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<String>,

    /// Whether this is part of an agentic loop
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_agentic: Option<bool>,

    /// Current step in agent loop (if detectable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_step: Option<usize>,
}

/// Known agent frameworks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentFramework {
    /// Anthropic Claude (Cursor, etc.)
    Claude,
    /// OpenAI Agents SDK
    OpenAiAgents,
    /// LangChain/LangGraph
    LangChain,
    /// AutoGen
    AutoGen,
    /// CrewAI
    CrewAi,
    /// Vercel AI SDK
    VercelAi,
    /// LlamaIndex
    LlamaIndex,
    /// Haystack
    Haystack,
    /// MCP-based agent
    Mcp,
    /// Custom/unknown
    Custom(String),
}

impl AgentContext {
    /// Detect agent context from tools and messages
    pub fn detect(tools: &[ToolDefinition], messages: &[Message]) -> Option<Self> {
        let mut context = AgentContext {
            framework: None,
            name: None,
            version: None,
            mcp_servers: Vec::new(),
            is_agentic: None,
            loop_step: None,
        };

        // Detect MCP servers from tool prefixes
        for tool in tools {
            if tool.name.starts_with("mcp_") {
                // Extract server name: mcp_figma_get_screenshot -> figma
                let parts: Vec<&str> = tool.name.split('_').collect();
                if parts.len() >= 2 {
                    let server = parts[1].to_string();
                    if !context.mcp_servers.contains(&server) {
                        context.mcp_servers.push(server);
                    }
                }
            }
        }

        // If MCP tools detected, mark as MCP agent
        if !context.mcp_servers.is_empty() {
            context.framework = Some(AgentFramework::Mcp);
            context.is_agentic = Some(true);
        }

        // Detect from tool naming patterns
        let has_langchain = tools.iter().any(|t| t.name.contains("langchain"));
        let has_llamaindex = tools.iter().any(|t| t.name.contains("llama_index"));
        let has_function_calls = !tools.is_empty();

        if has_langchain {
            context.framework = Some(AgentFramework::LangChain);
            context.is_agentic = Some(true);
        } else if has_llamaindex {
            context.framework = Some(AgentFramework::LlamaIndex);
            context.is_agentic = Some(true);
        }

        // Detect agentic loop from message patterns
        // Multiple tool result messages indicate an agent loop
        let tool_results = messages
            .iter()
            .filter(|m| matches!(m.role, MessageRole::Tool | MessageRole::Function))
            .count();

        if tool_results > 0 {
            context.is_agentic = Some(true);
            context.loop_step = Some(tool_results + 1);
        }

        // Only return if we detected something meaningful
        if context.framework.is_some()
            || !context.mcp_servers.is_empty()
            || context.is_agentic == Some(true)
        {
            Some(context)
        } else if has_function_calls {
            // Has tools but no specific framework detected
            context.is_agentic = Some(true);
            Some(context)
        } else {
            None
        }
    }
}
