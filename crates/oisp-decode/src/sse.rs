//! Server-Sent Events (SSE) parsing

use serde_json::Value;

/// A single SSE event
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

/// SSE parser for streaming responses
pub struct SseParser {
    buffer: String,
    events: Vec<SseEvent>,
}

impl SseParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            events: Vec::new(),
        }
    }

    /// Add data to the parser
    pub fn feed(&mut self, data: &[u8]) {
        if let Ok(s) = std::str::from_utf8(data) {
            self.buffer.push_str(s);
            self.parse_buffer();
        }
    }

    /// Get parsed events
    pub fn events(&self) -> &[SseEvent] {
        &self.events
    }

    /// Take all parsed events
    pub fn take_events(&mut self) -> Vec<SseEvent> {
        std::mem::take(&mut self.events)
    }

    /// Check if stream is done
    pub fn is_done(&self) -> bool {
        self.events.iter().any(|e| e.data == "[DONE]")
    }

    fn parse_buffer(&mut self) {
        // SSE events are separated by blank lines
        while let Some(pos) = self.buffer.find("\n\n") {
            let event_text = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 2..].to_string();

            if let Some(event) = self.parse_event(&event_text) {
                self.events.push(event);
            }
        }

        // Also handle \r\n\r\n
        while let Some(pos) = self.buffer.find("\r\n\r\n") {
            let event_text = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 4..].to_string();

            if let Some(event) = self.parse_event(&event_text) {
                self.events.push(event);
            }
        }
    }

    fn parse_event(&self, text: &str) -> Option<SseEvent> {
        let mut event = None;
        let mut data_lines = Vec::new();
        let mut id = None;
        let mut retry = None;

        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("event:") {
                event = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("id:") {
                id = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("retry:") {
                retry = rest.trim().parse().ok();
            }
        }

        if data_lines.is_empty() {
            return None;
        }

        Some(SseEvent {
            event,
            data: data_lines.join("\n"),
            id,
            retry,
        })
    }
}

impl Default for SseParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Reassemble streaming chunks into complete response
pub struct StreamReassembler {
    parser: SseParser,
    chunks: Vec<StreamChunk>,
    complete_content: String,
    #[allow(dead_code)]
    tool_calls: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub index: usize,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<Value>>,
    pub finish_reason: Option<String>,
}

impl StreamReassembler {
    pub fn new() -> Self {
        Self {
            parser: SseParser::new(),
            chunks: Vec::new(),
            complete_content: String::new(),
            tool_calls: Vec::new(),
        }
    }

    /// Feed data and parse chunks
    pub fn feed(&mut self, data: &[u8]) {
        self.parser.feed(data);

        for event in self.parser.take_events() {
            if event.data == "[DONE]" {
                continue;
            }

            // Try to parse as OpenAI-style streaming response
            if let Ok(json) = serde_json::from_str::<Value>(&event.data) {
                if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                    for choice in choices {
                        let index =
                            choice.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;

                        let content = choice
                            .get("delta")
                            .and_then(|d| d.get("content"))
                            .and_then(|c| c.as_str())
                            .map(String::from);

                        let tool_calls = choice
                            .get("delta")
                            .and_then(|d| d.get("tool_calls"))
                            .cloned();

                        let finish_reason = choice
                            .get("finish_reason")
                            .and_then(|f| f.as_str())
                            .map(String::from);

                        if let Some(c) = &content {
                            self.complete_content.push_str(c);
                        }

                        self.chunks.push(StreamChunk {
                            index,
                            content,
                            tool_calls: tool_calls.and_then(|t| t.as_array().cloned()),
                            finish_reason,
                        });
                    }
                }
            }
        }
    }

    /// Check if stream is complete
    pub fn is_complete(&self) -> bool {
        self.parser.is_done() || self.chunks.iter().any(|c| c.finish_reason.is_some())
    }

    /// Get complete content
    pub fn content(&self) -> &str {
        &self.complete_content
    }

    /// Get chunks
    pub fn chunks(&self) -> &[StreamChunk] {
        &self.chunks
    }

    /// Get finish reason
    pub fn finish_reason(&self) -> Option<&str> {
        self.chunks
            .iter()
            .filter_map(|c| c.finish_reason.as_deref())
            .next_back()
    }
}

impl Default for StreamReassembler {
    fn default() -> Self {
        Self::new()
    }
}
