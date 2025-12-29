//! Agent-related events - tool calls and results
//!
//! These structs MUST match the OISP spec exactly.
//! Spec: oisp-spec/schema/v0.1/events/agent.schema.json

use super::ai::{RedactedContent, ToolArguments};
use super::envelope::EventEnvelope;
use serde::{Deserialize, Serialize};

// =============================================================================
// Agent Tool Call Event
// =============================================================================

/// Agent tool call event - when an agent invokes a tool
/// Spec: agent.schema.json#/$defs/tool_call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCallEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,

    /// The data field containing tool call data
    pub data: AgentToolCallData,
}

/// Agent tool call data - matches spec exactly
/// Required fields: tool (per spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCallData {
    /// Agent information (optional per spec)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentInfo>,

    /// Tool information (required per spec)
    pub tool: ToolInfo,

    /// Unique identifier for this tool call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,

    /// What triggered this tool call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggered_by: Option<TriggeredBy>,

    /// Tool arguments (may be partially redacted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<ToolArguments>,

    /// Hash of arguments for correlation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments_hash: Option<String>,

    /// Whether this tool call required user approval
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_approval: Option<bool>,

    /// Whether user approved (if approval was required)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved: Option<bool>,

    /// Who approved the action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approver: Option<String>,

    /// Assessed risk level of this tool call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_level: Option<RiskLevel>,

    /// Reasons for the risk assessment
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub risk_reasons: Vec<String>,
}

// =============================================================================
// Agent Tool Result Event
// =============================================================================

/// Agent tool result event - result of tool execution
/// Spec: agent.schema.json#/$defs/tool_result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolResultEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,

    /// The data field containing tool result data
    pub data: AgentToolResultData,
}

/// Agent tool result data - matches spec exactly
/// Required fields: call_id (per spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolResultData {
    /// Agent information (optional per spec)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentInfo>,

    /// Tool information (optional per spec)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolInfo>,

    /// Links back to the tool_call event (required per spec)
    pub call_id: String,

    /// Whether execution succeeded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,

    /// Error information if request failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ToolError>,

    /// Tool result (may be redacted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ToolResultContent>,

    /// Hash of result for correlation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_hash: Option<String>,

    /// Result size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_size_bytes: Option<usize>,

    /// Tool execution time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Known side effects of this tool execution
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<SideEffect>,
}

// =============================================================================
// Shared Types
// =============================================================================

/// Information about the AI agent
/// Spec: agent.schema.json#/$defs/agent_info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Agent name or identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Type of agent
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<AgentType>,

    /// Agent version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Agent framework if known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,

    /// Agent session identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Current task identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

/// Type of agent
/// Spec: agent.schema.json#/$defs/agent_info/properties/type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    Ide,
    Cli,
    Browser,
    Server,
    Autonomous,
    Workflow,
    Other,
}

/// Information about a tool
/// Spec: agent.schema.json#/$defs/tool_info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Tool category
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<ToolCategory>,

    /// Tool provider (e.g., 'mcp', 'langchain', 'native')
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    /// MCP server name if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,

    /// Tool description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Tool category
/// Spec: agent.schema.json#/$defs/tool_info/properties/type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    FileRead,
    FileWrite,
    FileEdit,
    Terminal,
    Shell,
    Browser,
    Http,
    Database,
    CodeExecution,
    Search,
    Retrieval,
    ApiCall,
    ComputerUse,
    Other,
}

/// What triggered the tool call
/// Spec: agent.schema.json#/$defs/tool_call/properties/triggered_by
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggeredBy {
    LlmDecision,
    UserApproval,
    Automatic,
    Workflow,
    Retry,
}

/// Risk level assessment
/// Spec: agent.schema.json#/$defs/tool_call/properties/risk_level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Tool error information
/// Spec: agent.schema.json#/$defs/tool_result/properties/error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolError {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Tool result content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// Plain text result
    Text(String),
    /// Structured result
    Structured(serde_json::Value),
    /// Array result
    Array(Vec<serde_json::Value>),
    /// Redacted result
    Redacted(RedactedContent),
}

/// Side effect of tool execution
/// Spec: agent.schema.json#/$defs/tool_result/properties/side_effects/items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideEffect {
    /// Type of side effect
    #[serde(rename = "type")]
    pub effect_type: SideEffectType,

    /// What was affected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// Types of side effects
/// Spec: agent.schema.json#/$defs/tool_result/properties/side_effects/items/properties/type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectType {
    FileCreated,
    FileModified,
    FileDeleted,
    ProcessSpawned,
    NetworkRequest,
    DatabaseWrite,
    Other,
}

// =============================================================================
// Additional Agent Events (from spec)
// =============================================================================

/// Agent plan step event
/// Spec: agent.schema.json#/$defs/plan_step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlanStepEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,

    pub data: AgentPlanStepData,
}

/// Plan step data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlanStepData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentInfo>,

    /// Step number in the plan
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_index: Option<usize>,

    /// Step type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_type: Option<PlanStepType>,

    /// Human-readable description of this step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Actions the agent plans to take
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub planned_actions: Vec<PlannedAction>,

    /// Files the agent is considering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_files: Vec<String>,

    /// Iteration number (for loops/retries)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iteration: Option<usize>,
}

/// Plan step type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepType {
    Planning,
    Reasoning,
    Decision,
    Reflection,
    Revision,
}

/// A planned action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedAction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

/// Agent RAG retrieve event
/// Spec: agent.schema.json#/$defs/rag_retrieve
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRagRetrieveEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,

    pub data: AgentRagRetrieveData,
}

/// RAG retrieve data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRagRetrieveData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentInfo>,

    /// RAG source information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<RagSource>,

    /// Search query (may be redacted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// Hash of query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_hash: Option<String>,

    /// Number of results returned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results_count: Option<usize>,

    /// Retrieved results
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<RagResult>,

    /// Latency in ms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,

    /// Estimated tokens in retrieved context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_retrieved: Option<u64>,
}

/// RAG source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSource {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub source_type: Option<RagSourceType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// RAG source type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RagSourceType {
    VectorDb,
    KnowledgeBase,
    FileSearch,
    WebSearch,
    Database,
    Api,
    Other,
}

/// A RAG retrieval result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_preview: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Agent session event
/// Spec: agent.schema.json#/$defs/session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,

    pub data: AgentSessionData,
}

/// Agent session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentInfo>,

    /// Session lifecycle action (required per spec)
    pub action: SessionAction,

    /// Session ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// High-level task description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_description: Option<String>,

    /// Session duration in ms (for end events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Session statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<SessionStats>,
}

/// Session lifecycle action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionAction {
    Start,
    End,
    Pause,
    Resume,
    Error,
}

/// Session statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_calls: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_read: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_written: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
}
