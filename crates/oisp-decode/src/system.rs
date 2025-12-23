//! System event decoder for process, file, and network events
//!
//! This decoder handles non-HTTP events that come from eBPF tracepoints.

use async_trait::async_trait;
use oisp_core::events::envelope::{Actor, EventEnvelope, ProcessInfo};
use oisp_core::events::file::{FileAccess, FileOpenData, FileOpenEvent as OispFileOpenEvent};
use oisp_core::events::network::{
    Endpoint, NetworkConnectData, NetworkConnectEvent as OispNetworkConnectEvent, Protocol,
};
use oisp_core::events::process::{
    ProcessExecData, ProcessExecEvent as OispProcessExecEvent, ProcessExitData,
    ProcessExitEvent as OispProcessExitEvent, TerminationType,
};
use oisp_core::events::OispEvent;
use oisp_core::plugins::{
    DecodePlugin, Plugin, PluginInfo, PluginResult, RawCaptureEvent, RawEventKind,
};
use std::any::Any;
use tracing::debug;

/// System event decoder for process, file, and network events
pub struct SystemDecoder;

impl SystemDecoder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginInfo for SystemDecoder {
    fn name(&self) -> &str {
        "system-decoder"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Decodes process, file, and network events from eBPF tracepoints"
    }
}

impl Plugin for SystemDecoder {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl DecodePlugin for SystemDecoder {
    fn can_decode(&self, raw: &RawCaptureEvent) -> bool {
        matches!(
            raw.kind,
            RawEventKind::ProcessExec
                | RawEventKind::ProcessExit
                | RawEventKind::ProcessFork
                | RawEventKind::FileOpen
                | RawEventKind::FileRead
                | RawEventKind::FileWrite
                | RawEventKind::FileClose
                | RawEventKind::NetworkConnect
                | RawEventKind::NetworkAccept
        )
    }

    async fn decode(&self, raw: RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        let event = match raw.kind {
            RawEventKind::ProcessExec => self.decode_process_exec(&raw),
            RawEventKind::ProcessExit => self.decode_process_exit(&raw),
            RawEventKind::FileOpen => self.decode_file_open(&raw),
            RawEventKind::NetworkConnect => self.decode_network_connect(&raw),
            _ => {
                debug!("Unhandled system event kind: {:?}", raw.kind);
                return Ok(Vec::new());
            }
        };

        match event {
            Some(e) => Ok(vec![e]),
            None => Ok(Vec::new()),
        }
    }

    fn priority(&self) -> i32 {
        // Lower priority than HttpDecoder since SSL events go through HTTP decoder
        -10
    }
}

impl SystemDecoder {
    fn decode_process_exec(&self, raw: &RawCaptureEvent) -> Option<OispEvent> {
        let mut envelope = EventEnvelope::new("process.exec");

        // Set timestamp from raw event
        envelope.ts = timestamp_from_ns(raw.timestamp_ns);

        // Set process info
        envelope.process = Some(ProcessInfo {
            pid: raw.pid,
            ppid: raw.metadata.ppid,
            name: raw.metadata.comm.clone(),
            exe: raw.metadata.exe.clone(),
            cmdline: None,
            cwd: None,
            tid: raw.tid,
            container_id: None,
            hash: None,
            code_signature: None,
        });

        // Set actor info if we have uid
        if let Some(uid) = raw.metadata.uid {
            envelope.actor = Some(Actor {
                uid: Some(uid),
                user: None,
                gid: None,
                session_id: None,
                identity: None,
            });
        }

        let data = ProcessExecData {
            exe: raw.metadata.exe.clone().unwrap_or_default(),
            args: Vec::new(),
            cwd: None,
            env: std::collections::HashMap::new(),
            interpreter: None,
            script_path: None,
            is_shell: None,
            is_script: None,
            is_interactive: None,
            binary_hash: None,
            code_signature: None,
        };

        Some(OispEvent::ProcessExec(OispProcessExecEvent {
            envelope,
            data,
        }))
    }

    fn decode_process_exit(&self, raw: &RawCaptureEvent) -> Option<OispEvent> {
        let mut envelope = EventEnvelope::new("process.exit");

        envelope.ts = timestamp_from_ns(raw.timestamp_ns);

        // Set process info
        envelope.process = Some(ProcessInfo {
            pid: raw.pid,
            ppid: raw.metadata.ppid,
            name: raw.metadata.comm.clone(),
            exe: None,
            cmdline: None,
            cwd: None,
            tid: raw.tid,
            container_id: None,
            hash: None,
            code_signature: None,
        });

        // Get exit code from extra metadata
        let exit_code = raw
            .metadata
            .extra
            .get("exit_code")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let data = ProcessExitData {
            exit_code,
            signal: None,
            signal_name: None,
            runtime_ms: None,
            cpu_user_ms: None,
            cpu_system_ms: None,
            max_rss_kb: None,
            termination_type: Some(if exit_code == 0 {
                TerminationType::Normal
            } else {
                TerminationType::Unknown
            }),
        };

        Some(OispEvent::ProcessExit(OispProcessExitEvent {
            envelope,
            data,
        }))
    }

    fn decode_file_open(&self, raw: &RawCaptureEvent) -> Option<OispEvent> {
        let mut envelope = EventEnvelope::new("file.open");

        envelope.ts = timestamp_from_ns(raw.timestamp_ns);

        // Set process info
        envelope.process = Some(ProcessInfo {
            pid: raw.pid,
            ppid: raw.metadata.ppid,
            name: raw.metadata.comm.clone(),
            exe: None,
            cmdline: None,
            cwd: None,
            tid: raw.tid,
            container_id: None,
            hash: None,
            code_signature: None,
        });

        // Set actor info if we have uid
        if let Some(uid) = raw.metadata.uid {
            envelope.actor = Some(Actor {
                uid: Some(uid),
                user: None,
                gid: None,
                session_id: None,
                identity: None,
            });
        }

        let path = raw.metadata.path.clone().unwrap_or_default();
        let flags = raw
            .metadata
            .extra
            .get("flags")
            .and_then(|v| v.as_u64())
            .map(|f| f as u32);
        let mode = raw
            .metadata
            .extra
            .get("mode")
            .and_then(|v| v.as_u64())
            .map(|m| m as u32);

        // Determine access type from flags
        let access = flags.map(|f| {
            let access_mode = f & 3;
            if (f & 0o100) != 0 {
                // O_CREAT
                FileAccess::Create
            } else if (f & 0o1000) != 0 {
                // O_TRUNC
                FileAccess::Truncate
            } else if (f & 0o2000) != 0 {
                // O_APPEND
                FileAccess::Append
            } else if access_mode == 0 {
                FileAccess::Read
            } else if access_mode == 1 {
                FileAccess::Write
            } else {
                FileAccess::ReadWrite
            }
        });

        let data = FileOpenData {
            path,
            fd: raw.metadata.fd,
            flags,
            mode,
            access,
        };

        Some(OispEvent::FileOpen(OispFileOpenEvent { envelope, data }))
    }

    fn decode_network_connect(&self, raw: &RawCaptureEvent) -> Option<OispEvent> {
        let mut envelope = EventEnvelope::new("network.connect");

        envelope.ts = timestamp_from_ns(raw.timestamp_ns);

        // Set process info
        envelope.process = Some(ProcessInfo {
            pid: raw.pid,
            ppid: None,
            name: raw.metadata.comm.clone(),
            exe: None,
            cmdline: None,
            cwd: None,
            tid: raw.tid,
            container_id: None,
            hash: None,
            code_signature: None,
        });

        // Set actor info if we have uid
        if let Some(uid) = raw.metadata.uid {
            envelope.actor = Some(Actor {
                uid: Some(uid),
                user: None,
                gid: None,
                session_id: None,
                identity: None,
            });
        }

        let dest = Endpoint {
            ip: raw.metadata.remote_addr.clone(),
            port: raw.metadata.remote_port,
            domain: None,
            is_private: None,
            geo: None,
        };

        let src = if raw.metadata.local_addr.is_some() || raw.metadata.local_port.is_some() {
            Some(Endpoint {
                ip: raw.metadata.local_addr.clone(),
                port: raw.metadata.local_port,
                domain: None,
                is_private: None,
                geo: None,
            })
        } else {
            None
        };

        let data = NetworkConnectData {
            dest,
            src,
            protocol: Some(Protocol::Tcp),
            success: Some(true), // We only capture successful connections from eBPF
            error: None,
            latency_ms: None,
            tls: None,
        };

        Some(OispEvent::NetworkConnect(OispNetworkConnectEvent {
            envelope,
            data,
        }))
    }
}

/// Convert nanoseconds timestamp to chrono DateTime
fn timestamp_from_ns(ns: u64) -> chrono::DateTime<chrono::Utc> {
    use chrono::Utc;

    // eBPF ktime_get_ns returns monotonic clock, but for simplicity
    // we convert it relative to now.
    // In a production system, you'd want to capture boot time and add to it.

    // For now, just use current time - this is a simplification
    // The raw timestamp_ns is monotonic boot time, we'd need boot_time to convert
    // accurately. For UI purposes, using "now" is acceptable.
    let _ = ns;
    Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oisp_core::plugins::RawEventMetadata;

    #[tokio::test]
    async fn test_decode_process_exec() {
        let decoder = SystemDecoder::new();

        let raw = RawCaptureEvent {
            id: "test-1".to_string(),
            timestamp_ns: 1234567890,
            kind: RawEventKind::ProcessExec,
            pid: 1234,
            tid: Some(1234),
            data: Vec::new(),
            metadata: RawEventMetadata {
                comm: Some("python".to_string()),
                ppid: Some(1000),
                uid: Some(1000),
                exe: Some("/usr/bin/python3".to_string()),
                ..Default::default()
            },
        };

        assert!(decoder.can_decode(&raw));

        let events = decoder.decode(raw).await.unwrap();
        assert_eq!(events.len(), 1);

        if let OispEvent::ProcessExec(event) = &events[0] {
            let process = event.envelope.process.as_ref().unwrap();
            assert_eq!(process.pid, 1234);
            assert_eq!(process.ppid, Some(1000));
            assert_eq!(process.name, Some("python".to_string()));
            assert_eq!(event.data.exe, "/usr/bin/python3");
        } else {
            panic!("Expected ProcessExec event");
        }
    }

    #[tokio::test]
    async fn test_decode_file_open() {
        let decoder = SystemDecoder::new();

        let mut extra = std::collections::HashMap::new();
        extra.insert("flags".to_string(), serde_json::json!(0o100)); // O_CREAT
        extra.insert("mode".to_string(), serde_json::json!(0o644));

        let raw = RawCaptureEvent {
            id: "test-2".to_string(),
            timestamp_ns: 1234567890,
            kind: RawEventKind::FileOpen,
            pid: 1234,
            tid: Some(1234),
            data: Vec::new(),
            metadata: RawEventMetadata {
                comm: Some("vim".to_string()),
                ppid: Some(1000),
                uid: Some(1000),
                path: Some("/home/user/test.txt".to_string()),
                extra,
                ..Default::default()
            },
        };

        assert!(decoder.can_decode(&raw));

        let events = decoder.decode(raw).await.unwrap();
        assert_eq!(events.len(), 1);

        if let OispEvent::FileOpen(event) = &events[0] {
            assert_eq!(event.data.path, "/home/user/test.txt");
            assert_eq!(event.data.access, Some(FileAccess::Create));
        } else {
            panic!("Expected FileOpen event");
        }
    }

    #[tokio::test]
    async fn test_decode_network_connect() {
        let decoder = SystemDecoder::new();

        let raw = RawCaptureEvent {
            id: "test-3".to_string(),
            timestamp_ns: 1234567890,
            kind: RawEventKind::NetworkConnect,
            pid: 1234,
            tid: Some(1234),
            data: Vec::new(),
            metadata: RawEventMetadata {
                comm: Some("curl".to_string()),
                uid: Some(1000),
                remote_addr: Some("104.18.6.192".to_string()),
                remote_port: Some(443),
                fd: Some(5),
                ..Default::default()
            },
        };

        assert!(decoder.can_decode(&raw));

        let events = decoder.decode(raw).await.unwrap();
        assert_eq!(events.len(), 1);

        if let OispEvent::NetworkConnect(event) = &events[0] {
            assert_eq!(event.data.dest.ip, Some("104.18.6.192".to_string()));
            assert_eq!(event.data.dest.port, Some(443));
        } else {
            panic!("Expected NetworkConnect event");
        }
    }
}
