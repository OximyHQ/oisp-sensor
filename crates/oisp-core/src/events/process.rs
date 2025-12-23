//! Process lifecycle events

use super::envelope::{EventEnvelope, CodeSignature};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Process execution event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessExecEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    #[serde(flatten)]
    pub data: ProcessExecData,
}

/// Process exec data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessExecData {
    /// Executable path
    pub exe: String,
    
    /// Command line arguments
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    
    /// Current working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    
    /// Selected environment variables
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    
    /// Script interpreter if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpreter: Option<String>,
    
    /// Script path if running a script
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
    
    /// Whether this is a shell invocation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_shell: Option<bool>,
    
    /// Whether this is a script execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_script: Option<bool>,
    
    /// Whether this is an interactive session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_interactive: Option<bool>,
    
    /// Binary hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_hash: Option<String>,
    
    /// Code signature information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_signature: Option<CodeSignature>,
}

/// Process exit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessExitEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    #[serde(flatten)]
    pub data: ProcessExitData,
}

/// Process exit data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessExitData {
    /// Exit code
    pub exit_code: i32,
    
    /// Signal that caused termination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<i32>,
    
    /// Signal name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_name: Option<String>,
    
    /// Process runtime in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_ms: Option<u64>,
    
    /// User CPU time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_user_ms: Option<u64>,
    
    /// System CPU time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_system_ms: Option<u64>,
    
    /// Maximum RSS in KB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_rss_kb: Option<u64>,
    
    /// How the process terminated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination_type: Option<TerminationType>,
}

/// How a process terminated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminationType {
    Normal,
    Signaled,
    Coredump,
    Unknown,
}

/// Process fork event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessForkEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    #[serde(flatten)]
    pub data: ProcessForkData,
}

/// Process fork data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessForkData {
    /// Child process ID
    pub child_pid: u32,
    
    /// Clone flags (Linux)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clone_flags: Option<u64>,
    
    /// Whether this is a thread creation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_thread: Option<bool>,
}

