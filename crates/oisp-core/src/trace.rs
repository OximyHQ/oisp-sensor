//! Trace building - correlating events into agent traces
//!
//! This module provides trace building and correlation functionality
//! for connecting related events into complete agent traces.

use crate::events::{
    AgentToolCallEvent, AgentToolResultEvent, AiRequestEvent, AiResponseEvent, FileWriteEvent,
    NetworkConnectEvent, OispEvent, ProcessExecEvent,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A complete agent trace from initial prompt to final result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTrace {
    /// Unique trace ID
    pub trace_id: String,

    /// When the trace started
    pub started_at: DateTime<Utc>,

    /// When the trace ended (if complete)
    pub ended_at: Option<DateTime<Utc>>,

    /// Process that initiated the trace
    pub process_pid: u32,

    /// Process name
    pub process_name: Option<String>,

    /// Process executable
    pub process_exe: Option<String>,

    /// Root span ID
    pub root_span_id: Option<String>,

    /// All spans in this trace
    pub spans: Vec<Span>,

    /// Total token count
    pub total_tokens: u64,

    /// Total cost in USD
    pub total_cost_usd: f64,

    /// Number of LLM calls
    pub llm_call_count: u32,

    /// Number of tool calls
    pub tool_call_count: u32,

    /// Files accessed
    pub files_accessed: Vec<String>,

    /// Files modified
    pub files_modified: Vec<String>,

    /// Processes spawned
    pub processes_spawned: Vec<SpawnedProcess>,

    /// Network connections made
    pub connections_made: Vec<TraceConnection>,

    /// Whether trace is complete
    pub is_complete: bool,

    /// Summary (generated)
    pub summary: Option<String>,
}

impl AgentTrace {
    /// Create a new trace
    pub fn new(process_pid: u32) -> Self {
        Self {
            trace_id: ulid::Ulid::new().to_string(),
            started_at: Utc::now(),
            ended_at: None,
            process_pid,
            process_name: None,
            process_exe: None,
            root_span_id: None,
            spans: Vec::new(),
            total_tokens: 0,
            total_cost_usd: 0.0,
            llm_call_count: 0,
            tool_call_count: 0,
            files_accessed: Vec::new(),
            files_modified: Vec::new(),
            processes_spawned: Vec::new(),
            connections_made: Vec::new(),
            is_complete: false,
            summary: None,
        }
    }

    /// Get duration of the trace
    pub fn duration(&self) -> Duration {
        let end = self.ended_at.unwrap_or_else(Utc::now);
        end - self.started_at
    }

    /// Mark trace as complete
    pub fn complete(&mut self) {
        self.ended_at = Some(Utc::now());
        self.is_complete = true;
    }
}

/// A span within a trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Span ID
    pub span_id: String,

    /// Parent span ID
    pub parent_id: Option<String>,

    /// Span kind
    pub kind: SpanKind,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time
    pub end_time: Option<DateTime<Utc>>,

    /// Duration in milliseconds
    pub duration_ms: Option<u64>,

    /// Related request ID (for AI spans)
    pub request_id: Option<String>,

    /// Tool call ID (for tool spans)
    pub tool_call_id: Option<String>,

    /// Tool name (for tool spans)
    pub tool_name: Option<String>,

    /// Model used (for AI spans)
    pub model: Option<String>,

    /// Provider (for AI spans)
    pub provider: Option<String>,

    /// Token count
    pub tokens: Option<u64>,

    /// Summary/description
    pub summary: Option<String>,

    /// Child event IDs
    pub event_ids: Vec<String>,

    /// Status
    pub status: SpanStatus,
}

impl Span {
    pub fn new(kind: SpanKind) -> Self {
        Self {
            span_id: ulid::Ulid::new().to_string(),
            parent_id: None,
            kind,
            start_time: Utc::now(),
            end_time: None,
            duration_ms: None,
            request_id: None,
            tool_call_id: None,
            tool_name: None,
            model: None,
            provider: None,
            tokens: None,
            summary: None,
            event_ids: Vec::new(),
            status: SpanStatus::InProgress,
        }
    }

    pub fn complete(&mut self, status: SpanStatus) {
        self.end_time = Some(Utc::now());
        self.duration_ms = Some((Utc::now() - self.start_time).num_milliseconds() as u64);
        self.status = status;
    }
}

/// Kind of span
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanKind {
    /// User prompt to agent
    UserPrompt,
    /// Agent calling LLM
    LlmCall,
    /// Tool execution
    ToolExecution,
    /// Tool result submission
    ToolResultSubmission,
    /// Internal agent reasoning
    AgentReasoning,
    /// System event (file, process, network)
    SystemEvent,
}

/// Span status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    InProgress,
    Success,
    Error,
    Cancelled,
}

/// Spawned process info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedProcess {
    pub pid: u32,
    pub exe: String,
    pub args: Vec<String>,
    pub exit_code: Option<i32>,
    pub spawned_at: DateTime<Utc>,
}

/// Network connection in trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConnection {
    pub domain: Option<String>,
    pub ip: Option<String>,
    pub port: u16,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Builds traces from events
pub struct TraceBuilder {
    /// Active traces by process PID
    active_traces: HashMap<u32, AgentTrace>,

    /// Completed traces
    completed_traces: Vec<AgentTrace>,

    /// Pending AI requests (request_id -> span info)
    pending_requests: HashMap<String, PendingRequest>,

    /// Pending tool calls (tool_call_id -> span info)
    pending_tool_calls: HashMap<String, PendingToolCall>,

    /// Trace timeout (complete trace if no activity)
    trace_timeout: Duration,

    /// Maximum completed traces to keep
    max_completed: usize,
}

#[allow(dead_code)]
struct PendingRequest {
    trace_id: String,
    span_id: String,
    pid: u32,
    started_at: DateTime<Utc>,
}

#[allow(dead_code)]
struct PendingToolCall {
    trace_id: String,
    span_id: String,
    request_id: Option<String>,
    tool_name: String,
    started_at: DateTime<Utc>,
}

impl TraceBuilder {
    pub fn new() -> Self {
        Self {
            active_traces: HashMap::new(),
            completed_traces: Vec::new(),
            pending_requests: HashMap::new(),
            pending_tool_calls: HashMap::new(),
            trace_timeout: Duration::seconds(300), // 5 minutes
            max_completed: 100,
        }
    }

    /// Add an event and update traces
    pub fn add_event(&mut self, event: OispEvent) {
        match event {
            OispEvent::AiRequest(ref e) => self.handle_ai_request(e),
            OispEvent::AiResponse(ref e) => self.handle_ai_response(e),
            OispEvent::AgentToolCall(ref e) => self.handle_tool_call(e),
            OispEvent::AgentToolResult(ref e) => self.handle_tool_result(e),
            OispEvent::ProcessExec(ref e) => self.handle_process_exec(e),
            OispEvent::FileWrite(ref e) => self.handle_file_write(e),
            OispEvent::NetworkConnect(ref e) => self.handle_network_connect(e),
            _ => {}
        }

        // Cleanup old traces
        self.cleanup_stale_traces();
    }

    fn handle_ai_request(&mut self, event: &AiRequestEvent) {
        let pid = event.envelope.process.as_ref().map(|p| p.pid).unwrap_or(0);

        // Get or create trace for this process
        let trace = self.active_traces.entry(pid).or_insert_with(|| {
            let mut t = AgentTrace::new(pid);
            if let Some(proc) = &event.envelope.process {
                t.process_name = proc.name.clone();
                t.process_exe = proc.exe.clone();
            }
            t
        });

        // Create span for this LLM call
        let mut span = Span::new(SpanKind::LlmCall);
        span.request_id = Some(event.data.request_id.clone());
        span.model = event.data.model.as_ref().map(|m| m.id.clone());
        span.provider = event.data.provider.as_ref().map(|p| p.name.clone());
        span.event_ids.push(event.envelope.event_id.clone());

        // Track pending request
        self.pending_requests.insert(
            event.data.request_id.clone(),
            PendingRequest {
                trace_id: trace.trace_id.clone(),
                span_id: span.span_id.clone(),
                pid,
                started_at: event.envelope.ts,
            },
        );

        trace.spans.push(span);
        trace.llm_call_count += 1;
    }

    fn handle_ai_response(&mut self, event: &AiResponseEvent) {
        if let Some(pending) = self.pending_requests.remove(&event.data.request_id) {
            if let Some(trace) = self.active_traces.get_mut(&pending.pid) {
                // Find and complete the span
                if let Some(span) = trace
                    .spans
                    .iter_mut()
                    .find(|s| s.span_id == pending.span_id)
                {
                    span.complete(if event.data.success.unwrap_or(true) {
                        SpanStatus::Success
                    } else {
                        SpanStatus::Error
                    });
                    span.event_ids.push(event.envelope.event_id.clone());

                    // Update token counts
                    if let Some(usage) = &event.data.usage {
                        if let Some(total) = usage.total_tokens {
                            span.tokens = Some(total);
                            trace.total_tokens += total;
                        }
                        if let Some(cost) = usage.total_cost_usd {
                            trace.total_cost_usd += cost;
                        }
                    }
                }

                // Handle tool calls from response
                for tool_call in &event.data.tool_calls {
                    if let Some(id) = &tool_call.id {
                        let mut tool_span = Span::new(SpanKind::ToolExecution);
                        tool_span.tool_call_id = Some(id.clone());
                        tool_span.tool_name = Some(tool_call.name.clone());
                        tool_span.parent_id = Some(pending.span_id.clone());

                        self.pending_tool_calls.insert(
                            id.clone(),
                            PendingToolCall {
                                trace_id: trace.trace_id.clone(),
                                span_id: tool_span.span_id.clone(),
                                request_id: Some(event.data.request_id.clone()),
                                tool_name: tool_call.name.clone(),
                                started_at: event.envelope.ts,
                            },
                        );

                        trace.spans.push(tool_span);
                        trace.tool_call_count += 1;
                    }
                }
            }
        }
    }

    fn handle_tool_call(&mut self, event: &AgentToolCallEvent) {
        // Tool call events give us more detail about the tool execution
        if let Some(_pending) = self.pending_tool_calls.get_mut(&event.data.tool_call_id) {
            if let Some(trace) = self
                .active_traces
                .get_mut(&event.envelope.process.as_ref().map(|p| p.pid).unwrap_or(0))
            {
                if let Some(span) = trace
                    .spans
                    .iter_mut()
                    .find(|s| s.tool_call_id.as_ref() == Some(&event.data.tool_call_id))
                {
                    span.event_ids.push(event.envelope.event_id.clone());

                    // Generate summary based on tool type
                    if let Some(parsed) = &event.data.parsed_arguments {
                        span.summary = Some(format!("{}: {:?}", event.data.tool_name, parsed));
                    }
                }
            }
        }
    }

    fn handle_tool_result(&mut self, event: &AgentToolResultEvent) {
        if let Some(pending) = self.pending_tool_calls.remove(&event.data.tool_call_id) {
            if let Some(trace) = self
                .active_traces
                .get_mut(&event.envelope.process.as_ref().map(|p| p.pid).unwrap_or(0))
            {
                if let Some(span) = trace
                    .spans
                    .iter_mut()
                    .find(|s| s.span_id == pending.span_id)
                {
                    span.complete(if event.data.success {
                        SpanStatus::Success
                    } else {
                        SpanStatus::Error
                    });
                    span.duration_ms = event.data.duration_ms;
                    span.event_ids.push(event.envelope.event_id.clone());
                }
            }
        }
    }

    fn handle_process_exec(&mut self, event: &ProcessExecEvent) {
        let ppid = event.envelope.process.as_ref().and_then(|p| p.ppid);

        // Check if parent process has an active trace
        if let Some(ppid) = ppid {
            if let Some(trace) = self.active_traces.get_mut(&ppid) {
                trace.processes_spawned.push(SpawnedProcess {
                    pid: event.envelope.process.as_ref().map(|p| p.pid).unwrap_or(0),
                    exe: event.data.exe.clone(),
                    args: event.data.args.clone(),
                    exit_code: None,
                    spawned_at: event.envelope.ts,
                });
            }
        }
    }

    fn handle_file_write(&mut self, event: &FileWriteEvent) {
        let pid = event.envelope.process.as_ref().map(|p| p.pid).unwrap_or(0);

        if let Some(trace) = self.active_traces.get_mut(&pid) {
            if !trace.files_modified.contains(&event.data.path) {
                trace.files_modified.push(event.data.path.clone());
            }
        }
    }

    fn handle_network_connect(&mut self, event: &NetworkConnectEvent) {
        let pid = event.envelope.process.as_ref().map(|p| p.pid).unwrap_or(0);

        if let Some(trace) = self.active_traces.get_mut(&pid) {
            trace.connections_made.push(TraceConnection {
                domain: event.data.dest.domain.clone(),
                ip: event.data.dest.ip.clone(),
                port: event.data.dest.port.unwrap_or(0),
                bytes_sent: 0,
                bytes_received: 0,
            });
        }
    }

    fn cleanup_stale_traces(&mut self) {
        let now = Utc::now();
        let timeout = self.trace_timeout;

        let stale_pids: Vec<u32> = self
            .active_traces
            .iter()
            .filter(|(_, trace)| {
                // Check last activity
                let last_span_time = trace
                    .spans
                    .last()
                    .and_then(|s| s.end_time.or(Some(s.start_time)))
                    .unwrap_or(trace.started_at);
                now - last_span_time > timeout
            })
            .map(|(pid, _)| *pid)
            .collect();

        for pid in stale_pids {
            if let Some(mut trace) = self.active_traces.remove(&pid) {
                trace.complete();
                self.completed_traces.push(trace);
            }
        }

        // Trim completed traces
        while self.completed_traces.len() > self.max_completed {
            self.completed_traces.remove(0);
        }
    }

    /// Get active traces
    pub fn active_traces(&self) -> &HashMap<u32, AgentTrace> {
        &self.active_traces
    }

    /// Get completed traces
    pub fn completed_traces(&self) -> &[AgentTrace] {
        &self.completed_traces
    }

    /// Get all traces (active + completed)
    pub fn all_traces(&self) -> Vec<&AgentTrace> {
        self.active_traces
            .values()
            .chain(self.completed_traces.iter())
            .collect()
    }
}

impl Default for TraceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Correlation configuration
///
/// Settings for how events are correlated into traces.
#[derive(Debug, Clone)]
pub struct CorrelationConfig {
    /// Time window for correlating events (ms)
    pub time_window_ms: u64,

    /// Maximum trace duration before auto-complete (ms)
    pub max_trace_duration_ms: u64,

    /// Maximum traces to keep in memory
    pub max_traces: usize,
}

impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            time_window_ms: 5000,
            max_trace_duration_ms: 300000, // 5 minutes
            max_traces: 100,
        }
    }
}
