//! Test Event Generator
//!
//! Generates fake AI events for testing the pipeline and UI without needing eBPF.
//! This is crucial for development on macOS and CI testing.

use async_trait::async_trait;
use oisp_core::plugins::{
    CapturePlugin, CaptureStats, Plugin, PluginConfig, PluginInfo, PluginResult, RawCaptureEvent,
    RawEventKind, RawEventMetadata,
};
use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

/// Configuration for test event generation
#[derive(Debug, Clone)]
pub struct TestGeneratorConfig {
    /// Interval between events in milliseconds
    pub interval_ms: u64,

    /// Number of events to generate (0 = infinite)
    pub event_count: u64,

    /// Generate AI request/response pairs
    pub generate_ai_events: bool,

    /// Generate process events
    pub generate_process_events: bool,

    /// Generate file events
    pub generate_file_events: bool,

    /// Simulate specific process
    pub process_name: String,

    /// Simulate specific PID
    pub pid: u32,
}

impl Default for TestGeneratorConfig {
    fn default() -> Self {
        Self {
            interval_ms: 1000,
            event_count: 0, // infinite
            generate_ai_events: true,
            generate_process_events: true,
            generate_file_events: true,
            process_name: "cursor".to_string(),
            pid: 12345,
        }
    }
}

/// Test event generator plugin
pub struct TestGenerator {
    config: TestGeneratorConfig,
    running: Arc<AtomicBool>,
    stats: Arc<TestGeneratorStats>,
}

struct TestGeneratorStats {
    events_generated: AtomicU64,
}

impl TestGenerator {
    pub fn new() -> Self {
        Self::with_config(TestGeneratorConfig::default())
    }

    pub fn with_config(config: TestGeneratorConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(TestGeneratorStats {
                events_generated: AtomicU64::new(0),
            }),
        }
    }

    /// Create a sample OpenAI chat completion request
    fn create_ai_request(&self, request_id: &str) -> RawCaptureEvent {
        let request_body = serde_json::json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "You are a helpful coding assistant."},
                {"role": "user", "content": "Fix the bug in main.rs that causes a panic on line 42"}
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "read_file",
                        "description": "Read a file from the filesystem",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "path": {"type": "string"}
                            }
                        }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "write_file",
                        "description": "Write content to a file",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "path": {"type": "string"},
                                "content": {"type": "string"}
                            }
                        }
                    }
                }
            ],
            "stream": true
        });

        let http_request = format!(
            "POST /v1/chat/completions HTTP/1.1\r\n\
             Host: api.openai.com\r\n\
             Content-Type: application/json\r\n\
             Authorization: Bearer sk-proj-REDACTED\r\n\
             X-Request-ID: {}\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            request_id,
            request_body.to_string().len(),
            request_body
        );

        RawCaptureEvent {
            id: format!("test-req-{}", ulid::Ulid::new()),
            kind: RawEventKind::SslWrite,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            pid: self.config.pid,
            tid: Some(self.config.pid),
            data: http_request.into_bytes(),
            metadata: RawEventMetadata {
                comm: Some(self.config.process_name.clone()),
                exe: Some(format!("/usr/bin/{}", self.config.process_name)),
                ppid: Some(1),
                uid: Some(1000),
                fd: Some(42),
                path: None,
                remote_addr: Some("104.18.7.192".to_string()),
                remote_port: Some(443),
                local_addr: Some("192.168.1.100".to_string()),
                local_port: Some(54321),
                extra: HashMap::new(),
            },
        }
    }

    /// Create a sample OpenAI streaming response
    fn create_ai_response(&self, request_id: &str) -> RawCaptureEvent {
        let response_chunks = [
            r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#,
            r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"I'll"},"finish_reason":null}]}"#,
            r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":" read"},"finish_reason":null}]}"#,
            r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":" the"},"finish_reason":null}]}"#,
            r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":" file"},"finish_reason":null}]}"#,
            r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_abc123","type":"function","function":{"name":"read_file","arguments":"{\"path\":\"/src/main.rs\"}"}}]},"finish_reason":"tool_calls"}]}"#,
            r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4o","usage":{"prompt_tokens":150,"completion_tokens":25,"total_tokens":175}}"#,
            "data: [DONE]",
        ];

        let sse_body = response_chunks.join("\n\n");

        let http_response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: text/event-stream\r\n\
             X-Request-ID: {}\r\n\
             \r\n\
             {}",
            request_id, sse_body
        );

        RawCaptureEvent {
            id: format!("test-resp-{}", ulid::Ulid::new()),
            kind: RawEventKind::SslRead,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            pid: self.config.pid,
            tid: Some(self.config.pid),
            data: http_response.into_bytes(),
            metadata: RawEventMetadata {
                comm: Some(self.config.process_name.clone()),
                exe: Some(format!("/usr/bin/{}", self.config.process_name)),
                ppid: Some(1),
                uid: Some(1000),
                fd: Some(42),
                path: None,
                remote_addr: Some("104.18.7.192".to_string()),
                remote_port: Some(443),
                local_addr: Some("192.168.1.100".to_string()),
                local_port: Some(54321),
                extra: HashMap::new(),
            },
        }
    }

    /// Create an Anthropic Claude request
    fn create_anthropic_request(&self, request_id: &str) -> RawCaptureEvent {
        let request_body = serde_json::json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 4096,
            "messages": [
                {"role": "user", "content": "Explain how eBPF works for SSL interception"}
            ],
            "stream": true
        });

        let http_request = format!(
            "POST /v1/messages HTTP/1.1\r\n\
             Host: api.anthropic.com\r\n\
             Content-Type: application/json\r\n\
             x-api-key: sk-ant-REDACTED\r\n\
             anthropic-version: 2023-06-01\r\n\
             X-Request-ID: {}\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            request_id,
            request_body.to_string().len(),
            request_body
        );

        RawCaptureEvent {
            id: format!("test-anthropic-{}", ulid::Ulid::new()),
            kind: RawEventKind::SslWrite,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            pid: self.config.pid + 1,
            tid: Some(self.config.pid + 1),
            data: http_request.into_bytes(),
            metadata: RawEventMetadata {
                comm: Some("claude-cli".to_string()),
                exe: Some("/usr/local/bin/claude".to_string()),
                ppid: Some(1),
                uid: Some(1000),
                fd: Some(43),
                path: None,
                remote_addr: Some("104.18.8.192".to_string()),
                remote_port: Some(443),
                local_addr: Some("192.168.1.100".to_string()),
                local_port: Some(54322),
                extra: HashMap::new(),
            },
        }
    }

    /// Create a process exec event
    fn create_process_exec(&self) -> RawCaptureEvent {
        RawCaptureEvent {
            id: format!("test-proc-{}", ulid::Ulid::new()),
            kind: RawEventKind::ProcessExec,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            pid: self.config.pid + 10,
            tid: Some(self.config.pid + 10),
            data: b"cargo build --release".to_vec(),
            metadata: RawEventMetadata {
                comm: Some("cargo".to_string()),
                exe: Some("/usr/bin/cargo".to_string()),
                ppid: Some(self.config.pid),
                uid: Some(1000),
                fd: None,
                path: None,
                remote_addr: None,
                remote_port: None,
                local_addr: None,
                local_port: None,
                extra: HashMap::new(),
            },
        }
    }
}

impl Default for TestGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginInfo for TestGenerator {
    fn name(&self) -> &str {
        "test-generator"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Generates test events for pipeline and UI testing"
    }

    fn is_available(&self) -> bool {
        true // Always available
    }
}

impl Plugin for TestGenerator {
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()> {
        if let Some(interval) = config.get::<u64>("interval_ms") {
            self.config.interval_ms = interval;
        }
        if let Some(count) = config.get::<u64>("event_count") {
            self.config.event_count = count;
        }
        if let Some(process) = config.get::<String>("process_name") {
            self.config.process_name = process;
        }
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl CapturePlugin for TestGenerator {
    async fn start(&mut self, tx: mpsc::Sender<RawCaptureEvent>) -> PluginResult<()> {
        if self.running.load(Ordering::SeqCst) {
            return Err(oisp_core::plugins::PluginError::OperationFailed(
                "Already running".into(),
            ));
        }

        self.running.store(true, Ordering::SeqCst);
        info!("Starting test event generator");

        let running = self.running.clone();
        let stats = self.stats.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut event_num = 0u64;
            let mut cycle = 0u64;

            while running.load(Ordering::SeqCst) {
                if config.event_count > 0 && event_num >= config.event_count {
                    break;
                }

                // Generate a cycle of events
                let request_id = format!("req_{}", cycle);

                // OpenAI request/response pair
                if config.generate_ai_events {
                    let generator = TestGenerator::with_config(config.clone());

                    // Send request
                    let request = generator.create_ai_request(&request_id);
                    if tx.send(request).await.is_err() {
                        break;
                    }
                    stats.events_generated.fetch_add(1, Ordering::Relaxed);
                    event_num += 1;

                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    // Send response
                    let response = generator.create_ai_response(&request_id);
                    if tx.send(response).await.is_err() {
                        break;
                    }
                    stats.events_generated.fetch_add(1, Ordering::Relaxed);
                    event_num += 1;

                    // Occasionally send Anthropic events
                    if cycle.is_multiple_of(3) {
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                        let anthropic_req =
                            generator.create_anthropic_request(&format!("anthropic_{}", cycle));
                        if tx.send(anthropic_req).await.is_err() {
                            break;
                        }
                        stats.events_generated.fetch_add(1, Ordering::Relaxed);
                        event_num += 1;
                    }
                }

                // Process exec event
                if config.generate_process_events && cycle.is_multiple_of(2) {
                    let generator = TestGenerator::with_config(config.clone());
                    let exec_event = generator.create_process_exec();
                    if tx.send(exec_event).await.is_err() {
                        break;
                    }
                    stats.events_generated.fetch_add(1, Ordering::Relaxed);
                    event_num += 1;
                }

                cycle += 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(config.interval_ms)).await;
            }

            info!(
                "Test generator stopped after {} events",
                stats.events_generated.load(Ordering::Relaxed)
            );
        });

        Ok(())
    }

    async fn stop(&mut self) -> PluginResult<()> {
        info!("Stopping test generator...");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn stats(&self) -> CaptureStats {
        CaptureStats {
            events_captured: self.stats.events_generated.load(Ordering::Relaxed),
            events_dropped: 0,
            bytes_captured: 0,
            errors: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generator_creates_valid_events() {
        let generator = TestGenerator::new();

        let request = generator.create_ai_request("test-123");
        assert!(matches!(request.kind, RawEventKind::SslWrite));
        assert!(!request.data.is_empty());

        let response = generator.create_ai_response("test-123");
        assert!(matches!(response.kind, RawEventKind::SslRead));
        assert!(!response.data.is_empty());
    }

    #[tokio::test]
    async fn test_generator_runs() {
        let (tx, mut rx) = mpsc::channel(100);
        let mut generator = TestGenerator::with_config(TestGeneratorConfig {
            interval_ms: 10,
            event_count: 5,
            ..Default::default()
        });

        generator.start(tx).await.unwrap();

        // Collect some events
        let mut events = Vec::new();
        while let Ok(event) =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), rx.recv()).await
        {
            if let Some(e) = event {
                events.push(e);
            } else {
                break;
            }
        }

        assert!(!events.is_empty());
    }
}
