//! File operation events

use super::envelope::EventEnvelope;
use serde::{Deserialize, Serialize};

/// File open event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOpenEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    #[serde(flatten)]
    pub data: FileOpenData,
}

/// File open data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOpenData {
    /// File path
    pub path: String,
    
    /// File descriptor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fd: Option<i32>,
    
    /// Open flags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
    
    /// Open mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<u32>,
    
    /// Access type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access: Option<FileAccess>,
}

/// File access type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileAccess {
    Read,
    Write,
    ReadWrite,
    Append,
    Create,
    Truncate,
}

/// File read event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    #[serde(flatten)]
    pub data: FileReadData,
}

/// File read data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadData {
    /// File path
    pub path: String,
    
    /// File descriptor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fd: Option<i32>,
    
    /// Bytes read
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_read: Option<u64>,
    
    /// Offset in file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    
    /// Content hash (if captured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

/// File write event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWriteEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    #[serde(flatten)]
    pub data: FileWriteData,
}

/// File write data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWriteData {
    /// File path
    pub path: String,
    
    /// File descriptor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fd: Option<i32>,
    
    /// Bytes written
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_written: Option<u64>,
    
    /// Offset in file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    
    /// Content hash (if captured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    
    /// Whether this created the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<bool>,
    
    /// Whether this truncated the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

/// File close event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCloseEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    #[serde(flatten)]
    pub data: FileCloseData,
}

/// File close data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCloseData {
    /// File path
    pub path: String,
    
    /// File descriptor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fd: Option<i32>,
    
    /// Total bytes read during open
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes_read: Option<u64>,
    
    /// Total bytes written during open
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes_written: Option<u64>,
    
    /// Duration file was open
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_duration_ms: Option<u64>,
}

