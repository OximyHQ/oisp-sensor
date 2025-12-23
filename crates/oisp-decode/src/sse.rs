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

/// Parse Anthropic streaming response
pub struct AnthropicStreamReassembler {
    parser: SseParser,
    chunks: Vec<AnthropicStreamChunk>,
    complete_content: String,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    stop_reason: Option<String>,
    model: Option<String>,
    message_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AnthropicStreamChunk {
    pub event_type: String,
    pub index: Option<usize>,
    pub content: Option<String>,
    pub stop_reason: Option<String>,
}

impl AnthropicStreamReassembler {
    pub fn new() -> Self {
        Self {
            parser: SseParser::new(),
            chunks: Vec::new(),
            complete_content: String::new(),
            input_tokens: None,
            output_tokens: None,
            stop_reason: None,
            model: None,
            message_id: None,
        }
    }

    pub fn feed(&mut self, data: &[u8]) {
        self.parser.feed(data);

        for event in self.parser.take_events() {
            let event_type = event.event.clone().unwrap_or_default();

            if let Ok(json) = serde_json::from_str::<Value>(&event.data) {
                match event_type.as_str() {
                    "message_start" => {
                        if let Some(message) = json.get("message") {
                            self.message_id =
                                message.get("id").and_then(|i| i.as_str()).map(String::from);
                            self.model = message
                                .get("model")
                                .and_then(|m| m.as_str())
                                .map(String::from);
                            if let Some(usage) = message.get("usage") {
                                self.input_tokens =
                                    usage.get("input_tokens").and_then(|t| t.as_u64());
                            }
                        }
                    }
                    "content_block_delta" => {
                        if let Some(delta) = json.get("delta") {
                            if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                self.complete_content.push_str(text);
                                self.chunks.push(AnthropicStreamChunk {
                                    event_type: event_type.clone(),
                                    index: json
                                        .get("index")
                                        .and_then(|i| i.as_u64())
                                        .map(|i| i as usize),
                                    content: Some(text.to_string()),
                                    stop_reason: None,
                                });
                            }
                        }
                    }
                    "message_delta" => {
                        self.stop_reason = json
                            .get("delta")
                            .and_then(|d| d.get("stop_reason"))
                            .and_then(|r| r.as_str())
                            .map(String::from);
                        if let Some(usage) = json.get("usage") {
                            self.output_tokens =
                                usage.get("output_tokens").and_then(|t| t.as_u64());
                        }
                    }
                    "message_stop" => {
                        // Stream complete
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn is_complete(&self) -> bool {
        self.stop_reason.is_some()
    }

    pub fn content(&self) -> &str {
        &self.complete_content
    }

    pub fn stop_reason(&self) -> Option<&str> {
        self.stop_reason.as_deref()
    }

    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub fn usage(&self) -> (Option<u64>, Option<u64>) {
        (self.input_tokens, self.output_tokens)
    }
}

impl Default for AnthropicStreamReassembler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_parser_basic() {
        let mut parser = SseParser::new();
        parser.feed(b"data: hello world\n\n");

        let events = parser.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello world");
    }

    #[test]
    fn test_sse_parser_with_event_type() {
        let mut parser = SseParser::new();
        parser.feed(b"event: message\ndata: test\n\n");

        let events = parser.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, Some("message".to_string()));
        assert_eq!(events[0].data, "test");
    }

    #[test]
    fn test_sse_parser_multiple_events() {
        let mut parser = SseParser::new();
        parser.feed(b"data: first\n\ndata: second\n\n");

        let events = parser.events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "first");
        assert_eq!(events[1].data, "second");
    }

    #[test]
    fn test_sse_parser_multiline_data() {
        let mut parser = SseParser::new();
        parser.feed(b"data: line1\ndata: line2\n\n");

        let events = parser.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2");
    }

    #[test]
    fn test_sse_parser_done() {
        let mut parser = SseParser::new();
        parser.feed(b"data: [DONE]\n\n");

        assert!(parser.is_done());
    }

    #[test]
    fn test_sse_parser_with_id() {
        let mut parser = SseParser::new();
        parser.feed(b"id: 123\ndata: test\n\n");

        let events = parser.events();
        assert_eq!(events[0].id, Some("123".to_string()));
    }

    #[test]
    fn test_sse_parser_incremental() {
        let mut parser = SseParser::new();

        // Feed partial data
        parser.feed(b"data: hel");
        assert_eq!(parser.events().len(), 0);

        // Complete the event
        parser.feed(b"lo\n\n");
        assert_eq!(parser.events().len(), 1);
        assert_eq!(parser.events()[0].data, "hello");
    }

    #[test]
    fn test_stream_reassembler_openai() {
        let mut reassembler = StreamReassembler::new();

        // Simulate OpenAI streaming response
        let chunk1 = br#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

"#;
        let chunk2 = br#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

"#;
        let chunk3 = br#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":null}]}

"#;
        let chunk4 = br#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

"#;
        let done = b"data: [DONE]\n\n";

        reassembler.feed(chunk1);
        reassembler.feed(chunk2);
        reassembler.feed(chunk3);
        reassembler.feed(chunk4);
        reassembler.feed(done);

        assert!(reassembler.is_complete());
        assert_eq!(reassembler.content(), "Hello!");
        assert_eq!(reassembler.finish_reason(), Some("stop"));
    }

    #[test]
    fn test_anthropic_stream_reassembler() {
        let mut reassembler = AnthropicStreamReassembler::new();

        let start = br#"event: message_start
data: {"type":"message_start","message":{"id":"msg_123","model":"claude-3-opus","usage":{"input_tokens":10}}}

"#;
        let delta1 = br#"event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

"#;
        let delta2 = br#"event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"!"}}

"#;
        let msg_delta = br#"event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":5}}

"#;
        let stop = br#"event: message_stop
data: {"type":"message_stop"}

"#;

        reassembler.feed(start);
        reassembler.feed(delta1);
        reassembler.feed(delta2);
        reassembler.feed(msg_delta);
        reassembler.feed(stop);

        assert!(reassembler.is_complete());
        assert_eq!(reassembler.content(), "Hello!");
        assert_eq!(reassembler.stop_reason(), Some("end_turn"));
        assert_eq!(reassembler.model(), Some("claude-3-opus"));
        assert_eq!(reassembler.usage(), (Some(10), Some(5)));
    }
}
