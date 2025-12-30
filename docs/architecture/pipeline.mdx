---
title: Pipeline Architecture
description: Event processing pipeline and plugin system
---

The OISP Sensor pipeline is a modular, plugin-based system for processing events from capture to export.

## Pipeline Overview

```rust
pub struct Pipeline {
    capture_plugins: Vec<Box<dyn CapturePlugin>>,
    decode_plugins: Vec<Box<dyn DecodePlugin>>,
    enrich_plugins: Vec<Box<dyn EnrichPlugin>>,
    action_plugins: Vec<Box<dyn ActionPlugin>>,
    export_plugins: Vec<Box<dyn ExportPlugin>>,
}
```

Events flow through stages in order:

```
Capture → Decode → Enrich → Action → Export
                              ↓
                         Trace Builder
                              ↓
                          Broadcast (WebSocket)
```

## Plugin Traits

All plugins implement a base trait:

```rust
pub trait Plugin: PluginInfo + Send + Sync {
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()>;
    fn shutdown(&mut self) -> PluginResult<()>;
}

pub trait PluginInfo {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn is_available(&self) -> bool;
}
```

### Capture Plugin

```rust
#[async_trait]
pub trait CapturePlugin: Plugin {
    async fn start(&mut self, tx: mpsc::Sender<RawCaptureEvent>) -> PluginResult<()>;
    async fn stop(&mut self) -> PluginResult<()>;
    fn is_running(&self) -> bool;
    fn stats(&self) -> CaptureStats;
}
```

Capture plugins:
- **EbpfCapture**: Linux eBPF-based capture
- **TestGenerator**: Synthetic events for testing

### Decode Plugin

```rust
#[async_trait]
pub trait DecodePlugin: Plugin {
    fn can_decode(&self, raw: &RawCaptureEvent) -> bool;
    async fn decode(&self, raw: RawCaptureEvent) -> PluginResult<Vec<OispEvent>>;
    fn priority(&self) -> i32;
}
```

Decode plugins:
- **HttpDecoder**: SSL bytes → HTTP → AI events
- **SystemDecoder**: Process/file/network events

### Enrich Plugin

```rust
#[async_trait]
pub trait EnrichPlugin: Plugin {
    async fn enrich(&self, event: &mut OispEvent) -> PluginResult<()>;
    fn applies_to(&self, event: &OispEvent) -> bool;
}
```

Enrich plugins:
- **HostEnricher**: Adds hostname, OS, architecture
- **ProcessTreeEnricher**: Builds parent-child relationships

### Action Plugin

```rust
#[async_trait]
pub trait ActionPlugin: Plugin {
    async fn process(&self, event: OispEvent) -> PluginResult<(OispEvent, EventAction)>;
    fn applies_to(&self, event: &OispEvent) -> bool;
}

pub enum EventAction {
    Pass,       // Continue unchanged
    Modified,   // Event was mutated
    Drop,       // Discard event
    Replace(Vec<OispEvent>), // Replace with new events
}
```

Action plugins:
- **RedactionPlugin**: Masks sensitive data

### Export Plugin

```rust
#[async_trait]
pub trait ExportPlugin: Plugin {
    async fn export(&self, event: &OispEvent) -> PluginResult<()>;
    async fn export_batch(&self, events: &[OispEvent]) -> PluginResult<()>;
    async fn flush(&self) -> PluginResult<()>;
}
```

Export plugins:
- **JsonlExporter**: File output
- **WebSocketExporter**: Real-time streaming
- **OtlpExporter**: OpenTelemetry Protocol
- **KafkaExporter**: Apache Kafka
- **WebhookExporter**: HTTP endpoints

## Event Types

### Raw Capture Events

```rust
pub struct RawCaptureEvent {
    pub id: String,
    pub timestamp_ns: u64,
    pub kind: RawEventKind,
    pub pid: u32,
    pub tid: Option<u32>,
    pub data: Vec<u8>,
    pub metadata: RawEventMetadata,
}

pub enum RawEventKind {
    SslWrite,
    SslRead,
    ProcessExec,
    ProcessExit,
    FileOpen,
    NetworkConnect,
    // ...
}
```

### OISP Events

```rust
pub enum OispEvent {
    AiRequest(AiRequestEvent),
    AiResponse(AiResponseEvent),
    AiStreamingChunk(AiStreamingChunkEvent),
    AgentToolCall(AgentToolCallEvent),
    ProcessExec(ProcessExecEvent),
    ProcessExit(ProcessExitEvent),
    FileOpen(FileOpenEvent),
    NetworkConnect(NetworkConnectEvent),
    // ...
}
```

## Processing Flow

```rust
async fn process_raw_event(&mut self, raw: RawCaptureEvent) {
    // 1. Find matching decoder
    for decoder in &self.decode_plugins {
        if decoder.can_decode(&raw) {
            let events = decoder.decode(raw).await?;
            
            for mut event in events {
                // 2. Apply enrichers
                for enricher in &self.enrich_plugins {
                    if enricher.applies_to(&event) {
                        enricher.enrich(&mut event).await?;
                    }
                }
                
                // 3. Apply actions
                for action in &self.action_plugins {
                    let (new_event, action) = action.process(event).await?;
                    match action {
                        EventAction::Drop => return,
                        EventAction::Pass | EventAction::Modified => {
                            event = new_event;
                        }
                        EventAction::Replace(events) => {
                            // Handle replacement
                        }
                    }
                }
                
                // 4. Build traces (if enabled)
                if let Some(trace_builder) = &self.trace_builder {
                    trace_builder.add_event(&event);
                }
                
                // 5. Broadcast to WebSocket clients
                self.event_sender.send(event.clone())?;
                
                // 6. Export
                for exporter in &self.export_plugins {
                    exporter.export(&event).await?;
                }
            }
            break; // First matching decoder wins
        }
    }
}
```

## HTTP Decoder Details

The HTTP decoder is the most complex, handling:

### Request/Response Correlation

```rust
struct HttpDecoder {
    pending_requests: HashMap<CorrelationKey, PendingRequest>,
    stream_reassemblers: HashMap<CorrelationKey, StreamReassembler>,
}

type CorrelationKey = (u32, u32); // (pid, tid)
```

Correlation logic:
1. HTTP request received → store in `pending_requests`
2. HTTP response received → look up matching request by (pid, tid)
3. Timeout cleanup: 10 seconds for non-streaming, 30 seconds for SSE

### AI Provider Detection

```rust
fn detect_provider(url: &str, headers: &Headers) -> Option<ProviderInfo> {
    if url.contains("api.openai.com") {
        return Some(ProviderInfo { name: "openai", ... });
    }
    if url.contains("api.anthropic.com") {
        return Some(ProviderInfo { name: "anthropic", ... });
    }
    // Check for self-hosted (vLLM, Ollama, etc.) via headers
    if headers.contains("x-vllm-") {
        return Some(ProviderInfo { name: "vllm", ... });
    }
    // ...
}
```

### Streaming Response Handling

OpenAI and Anthropic use different streaming formats:

```rust
// OpenAI: data: {"choices":[...]}
fn parse_openai_sse(line: &str) -> Option<StreamingChunk>;

// Anthropic: event: content_block_delta\ndata: {...}
fn parse_anthropic_sse(event: &str, data: &str) -> Option<StreamingChunk>;
```

## Trace Building

The trace builder groups related events:

```rust
pub struct TraceBuilder {
    active_traces: HashMap<TraceKey, AgentTrace>,
    max_trace_duration: Duration,
    cleanup_interval: Duration,
}

pub struct AgentTrace {
    pub trace_id: String,
    pub root_span: Span,
    pub spans: Vec<Span>,
    pub start_time: DateTime<Utc>,
    pub status: TraceStatus,
}
```

Events are grouped by:
1. Process hierarchy (ppid → pid)
2. Request/response pairs (request_id)
3. Time proximity (configurable window)

## Extending the Pipeline

To add a custom plugin:

```rust
pub struct MyEnricher;

impl PluginInfo for MyEnricher {
    fn name(&self) -> &str { "my-enricher" }
    fn version(&self) -> &str { "1.0.0" }
}

impl Plugin for MyEnricher {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

#[async_trait]
impl EnrichPlugin for MyEnricher {
    async fn enrich(&self, event: &mut OispEvent) -> PluginResult<()> {
        // Add custom metadata
        event.envelope_mut().attrs.insert(
            "custom.field".to_string(),
            json!("value"),
        );
        Ok(())
    }
}

// Register in pipeline
pipeline.add_enrich(Box::new(MyEnricher));
```

## Performance Considerations

- **Bounded channels**: Prevent memory exhaustion under load
- **Priority decoders**: HTTP checked before System (higher priority)
- **Batched exports**: OTLP and Kafka batch by count/time
- **Async everything**: Non-blocking I/O throughout

