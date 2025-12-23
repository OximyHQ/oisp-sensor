//! Event correlation and trace building
//!
//! Re-exports the trace builder from oisp-core and provides additional
//! correlation utilities.

pub use oisp_core::trace::{AgentTrace, Span, SpanKind, SpanStatus, TraceBuilder};

/// Correlation configuration
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

