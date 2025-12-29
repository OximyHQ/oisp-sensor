//! Main decoder plugin
//!
//! Decodes raw SSL/TLS capture events into structured OISP events.
//! Handles HTTP request/response correlation and AI provider detection.

use crate::ai::{
    detect_provider_from_body, is_ai_request, parse_ai_request, parse_ai_response,
    parse_anthropic_request, parse_anthropic_response,
};
use crate::http::{is_http_request, is_http_response, parse_request, parse_response};
use crate::sse::{AnthropicStreamReassembler, StreamReassembler};

use oisp_core::events::*;
use oisp_core::plugins::{
    DecodePlugin, Plugin, PluginConfig, PluginInfo, PluginResult, RawCaptureEvent, RawEventKind,
};
use oisp_core::providers::{Provider, ProviderRegistry};
use oisp_core::spec::{DynamicProviderRegistry, SpecLoader};

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, info, trace, warn};

/// Maximum time to keep a pending request before discarding
const PENDING_REQUEST_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

/// Maximum number of pending requests to keep (prevents memory leaks)
const MAX_PENDING_REQUESTS: usize = 10000;

/// HTTP decoder plugin
pub struct HttpDecoder {
    /// Spec-driven provider registry (95+ providers from spec bundle)
    spec_registry: Arc<DynamicProviderRegistry>,
    /// Legacy provider registry (for Provider enum conversion, backward compatibility)
    legacy_registry: ProviderRegistry,
    // Track partial requests being reassembled
    partial_requests: RwLock<HashMap<CorrelationKey, RequestReassembler>>,
    // Track partial responses being reassembled
    partial_responses: RwLock<HashMap<CorrelationKey, ResponseReassembler>>,
    // Track pending requests for correlation
    pending_requests: RwLock<HashMap<CorrelationKey, PendingRequest>>,
    // Track streaming responses (OpenAI style)
    stream_reassemblers: RwLock<HashMap<CorrelationKey, StreamReassembler>>,
    // Track Anthropic streaming responses
    anthropic_reassemblers: RwLock<HashMap<CorrelationKey, AnthropicStreamReassembler>>,
    // Last cleanup time
    last_cleanup: RwLock<Instant>,
}

#[derive(Clone)]
struct ResponseReassembler {
    headers: crate::http::ParsedHttpResponse,
    body_buffer: Vec<u8>,
    created_at: Instant,
}

impl ResponseReassembler {
    fn new(headers: crate::http::ParsedHttpResponse) -> Self {
        let body_initial = headers.body.clone().unwrap_or_default();
        Self {
            headers,
            body_buffer: body_initial,
            created_at: Instant::now(),
        }
    }

    fn feed(&mut self, data: &[u8]) {
        self.body_buffer.extend_from_slice(data);
    }

    /// Try standard gzip decompression using flate2
    fn try_gzip_decompress(data: &[u8]) -> Option<Vec<u8>> {
        use flate2::bufread::GzDecoder;
        use std::io::{BufReader, Read};

        let reader = BufReader::new(data);
        let mut decoder = GzDecoder::new(reader);
        let mut decompressed = Vec::new();

        match decoder.read_to_end(&mut decompressed) {
            Ok(_) if !decompressed.is_empty() => Some(decompressed),
            Ok(_) => {
                info!("Gzip decompress returned empty");
                None
            }
            Err(e) => {
                info!("Gzip decompress failed: {}", e);
                None
            }
        }
    }

    /// Try decompression by skipping the gzip wrapper and using raw deflate
    /// Handles streaming gzip with sync-flush markers (00 00 00 ff ff or 00 00 ff ff)
    fn try_miniz_decompress(data: &[u8]) -> Option<Vec<u8>> {
        use flate2::Decompress;
        use flate2::FlushDecompress;

        // Skip gzip header (minimum 10 bytes)
        if data.len() <= 10 || data[0] != 0x1f || data[1] != 0x8b {
            return None;
        }

        let mut header_end = 10;
        let flags = data[3];

        // Check for extra field (FEXTRA)
        if flags & 0x04 != 0 && header_end + 2 <= data.len() {
            let xlen = u16::from_le_bytes([data[header_end], data[header_end + 1]]) as usize;
            header_end += 2 + xlen;
        }
        // Check for filename (FNAME)
        if flags & 0x08 != 0 {
            while header_end < data.len() && data[header_end] != 0 {
                header_end += 1;
            }
            header_end += 1;
        }
        // Check for comment (FCOMMENT)
        if flags & 0x10 != 0 {
            while header_end < data.len() && data[header_end] != 0 {
                header_end += 1;
            }
            header_end += 1;
        }
        // Check for header CRC (FHCRC)
        if flags & 0x02 != 0 {
            header_end += 2;
        }

        if header_end >= data.len() {
            return None;
        }

        // Check for sync-flush marker at the start of deflate data
        // Streaming gzip often starts with an empty sync-flush: 00 00 00 ff ff or 00 00 ff ff
        let mut deflate_start = header_end;

        // Pattern 1: 00 00 00 ff ff (5 bytes - empty stored block with sync flush)
        if data.len() >= deflate_start + 5
            && data[deflate_start] == 0x00
            && data[deflate_start + 1] == 0x00
            && data[deflate_start + 2] == 0x00
            && data[deflate_start + 3] == 0xff
            && data[deflate_start + 4] == 0xff
        {
            info!("Found 5-byte sync-flush marker at start, skipping");
            deflate_start += 5;
        }
        // Pattern 2: 00 00 ff ff (4 bytes)
        else if data.len() >= deflate_start + 4
            && data[deflate_start] == 0x00
            && data[deflate_start + 1] == 0x00
            && data[deflate_start + 2] == 0xff
            && data[deflate_start + 3] == 0xff
        {
            info!("Found 4-byte sync-flush marker at start, skipping");
            deflate_start += 4;
        }

        // The deflate data, excluding the 8-byte gzip trailer (if present)
        let deflate_end = if data.len() >= deflate_start + 8 {
            data.len() - 8
        } else {
            data.len()
        };

        if deflate_start >= deflate_end {
            info!("No deflate data after skipping sync-flush markers");
            return None;
        }

        let deflate_data = &data[deflate_start..deflate_end];

        info!(
            "Trying streaming deflate on {} bytes (deflate_start={}, deflate_end={}), first 10 bytes: {:?}",
            deflate_data.len(),
            deflate_start,
            deflate_end,
            &deflate_data[..std::cmp::min(10, deflate_data.len())]
        );

        // Use low-level Decompress API which can return partial results
        let mut decompress = Decompress::new(false); // false = raw deflate (no zlib header)
        let mut output = vec![0u8; 10 * 1024 * 1024]; // 10MB max
        let mut total_out = 0;
        let mut total_in = 0;

        loop {
            let before_in = decompress.total_in() as usize;
            let before_out = decompress.total_out() as usize;

            let input = &deflate_data[total_in..];
            let out_slice = &mut output[total_out..];

            if input.is_empty() || out_slice.is_empty() {
                break;
            }

            match decompress.decompress(input, out_slice, FlushDecompress::Sync) {
                Ok(status) => {
                    let bytes_in = decompress.total_in() as usize - before_in;
                    let bytes_out = decompress.total_out() as usize - before_out;
                    total_in += bytes_in;
                    total_out += bytes_out;

                    match status {
                        flate2::Status::Ok => {
                            // Continue decompressing
                            if bytes_in == 0 && bytes_out == 0 {
                                break; // No progress
                            }
                        }
                        flate2::Status::BufError => {
                            // Need more output space or hit end of input
                            break;
                        }
                        flate2::Status::StreamEnd => {
                            // Done!
                            break;
                        }
                    }
                }
                Err(e) => {
                    info!(
                        "Streaming deflate error after {} bytes out: {}",
                        total_out, e
                    );
                    break;
                }
            }
        }

        if total_out > 0 {
            output.truncate(total_out);
            info!(
                "Streaming deflate succeeded: {} in -> {} out",
                total_in, total_out
            );
            Some(output)
        } else {
            info!("Streaming deflate produced no output");
            None
        }
    }

    fn is_complete(&self) -> bool {
        if self.headers.is_chunked {
            // For chunked encoding, look for the final chunk marker "0\r\n\r\n"
            // This is more lenient than full validation since SSL reads may fragment chunks
            Self::has_final_chunk_marker(&self.body_buffer)
        } else if let Some(content_len) = self.headers.content_length {
            self.body_buffer.len() >= content_len
        } else {
            // No length info and not chunked - usually means end of stream
            true
        }
    }

    /// Check if buffer contains the final chunk marker (0\r\n\r\n)
    /// This indicates the chunked response is complete even if intermediate chunks are fragmented
    fn has_final_chunk_marker(data: &[u8]) -> bool {
        // Look for "0\r\n\r\n" anywhere in the last 20 bytes
        if data.len() < 5 {
            return false;
        }

        let search_start = data.len().saturating_sub(20);
        let search_region = &data[search_start..];

        // Pattern: 0\r\n\r\n
        for i in 0..search_region.len().saturating_sub(4) {
            if search_region[i] == b'0'
                && search_region[i + 1] == b'\r'
                && search_region[i + 2] == b'\n'
                && search_region[i + 3] == b'\r'
                && search_region[i + 4] == b'\n'
            {
                info!(
                    "Found final chunk marker at offset {} from end",
                    data.len() - (search_start + i)
                );
                return true;
            }
        }

        false
    }

    fn decompress_if_needed(&mut self) {
        info!(
            "decompress_if_needed: is_gzipped={}, is_chunked={}, body_buffer_len={}",
            self.headers.is_gzipped,
            self.headers.is_chunked,
            self.body_buffer.len()
        );

        if self.headers.is_gzipped {
            // For chunked encoding, we need to extract the actual data from the chunks first.
            // Our self.body_buffer contains the RAW chunked stream.
            let raw_data = if self.headers.is_chunked {
                if let Some(decoded) = crate::http::decode_chunked_body(&self.body_buffer) {
                    info!(
                        "Chunked decode succeeded: {} -> {} bytes",
                        self.body_buffer.len(),
                        decoded.len()
                    );
                    decoded
                } else {
                    info!("Chunked decode FAILED, using raw buffer");
                    self.body_buffer.clone()
                }
            } else {
                self.body_buffer.clone()
            };

            info!(
                "Attempting gzip decompress of {} bytes, first 20: {:?}, last 20: {:?}",
                raw_data.len(),
                &raw_data[..std::cmp::min(20, raw_data.len())],
                &raw_data[raw_data.len().saturating_sub(20)..]
            );

            // Try standard gzip decompression first
            if let Some(decompressed) = Self::try_gzip_decompress(&raw_data) {
                info!(
                    "Gzip decompress succeeded: {} -> {} bytes",
                    raw_data.len(),
                    decompressed.len()
                );
                self.body_buffer = decompressed;
                return;
            }

            // Try using miniz_oxide directly with lenient parsing for truncated/incomplete streams
            if raw_data.len() > 10 && raw_data[0] == 0x1f && raw_data[1] == 0x8b {
                if let Some(decompressed) = Self::try_miniz_decompress(&raw_data) {
                    info!(
                        "Miniz decompress succeeded: {} -> {} bytes",
                        raw_data.len(),
                        decompressed.len()
                    );
                    self.body_buffer = decompressed;
                    return;
                }
            }

            info!("All decompression methods failed, using raw data");
            self.body_buffer = raw_data;
        } else if self.headers.is_chunked {
            // Not gzipped, but still chunked - need to decode chunks
            if let Some(decoded) = crate::http::decode_chunked_body(&self.body_buffer) {
                self.body_buffer = decoded;
            }
        }

        info!(
            "After decompress: body_buffer_len={}, preview: {:?}",
            self.body_buffer.len(),
            String::from_utf8_lossy(
                &self.body_buffer[..std::cmp::min(100, self.body_buffer.len())]
            )
        );
    }
}

#[derive(Clone)]
struct RequestReassembler {
    buffer: Vec<u8>,
    expected_body_len: Option<usize>,
    header_len: Option<usize>,
    created_at: Instant,
}

impl RequestReassembler {
    fn new(data: &[u8]) -> Self {
        let mut reassembler = Self {
            buffer: data.to_vec(),
            expected_body_len: None,
            header_len: None,
            created_at: Instant::now(),
        };
        reassembler.try_parse_headers();
        reassembler
    }

    fn try_parse_headers(&mut self) {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        if let Ok(httparse::Status::Complete(header_len)) = req.parse(&self.buffer) {
            self.header_len = Some(header_len);
            for header in req.headers.iter() {
                if header.name.to_lowercase() == "content-length" {
                    if let Ok(val_str) = std::str::from_utf8(header.value) {
                        if let Ok(len) = val_str.trim().parse::<usize>() {
                            self.expected_body_len = Some(len);
                        }
                    }
                }
            }
        }
    }

    fn feed(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        if self.header_len.is_none() {
            self.try_parse_headers();
        }
    }

    fn is_complete(&self) -> bool {
        match (self.header_len, self.expected_body_len) {
            (Some(h_len), Some(b_len)) => self.buffer.len() >= h_len + b_len,
            (Some(h_len), None) => {
                // If no content-length, assume complete if headers end with \r\n\r\n
                // (Though for POST this usually means no body)
                self.buffer.len() >= h_len
            }
            _ => false,
        }
    }
}

/// Key for correlating requests and responses
/// Uses PID + optional TID for more accurate correlation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct CorrelationKey {
    pid: u32,
    tid: Option<u32>,
    /// File descriptor if available (for multiple connections per process)
    fd: Option<i32>,
}

impl CorrelationKey {
    fn from_event(raw: &RawCaptureEvent) -> Self {
        Self {
            pid: raw.pid,
            tid: raw.tid,
            fd: raw.metadata.fd,
        }
    }

    /// Alternative key without TID for fallback matching
    fn without_tid(&self) -> Self {
        Self {
            pid: self.pid,
            tid: None,
            fd: self.fd,
        }
    }
}

#[derive(Clone)]
struct PendingRequest {
    request_id: String,
    request_data: AiRequestData,
    timestamp: chrono::DateTime<chrono::Utc>,
    #[allow(dead_code)]
    created_at: Instant,
    provider: Provider,
    is_streaming: bool,
    #[allow(dead_code)]
    host: Option<String>,
}

impl HttpDecoder {
    /// Create a new decoder with default spec bundle
    pub fn new() -> Self {
        let spec_loader = SpecLoader::new();
        let spec_registry = Arc::new(DynamicProviderRegistry::new(spec_loader.bundle()));
        let provider_count = spec_registry.provider_ids().len();
        info!(
            "Initialized HttpDecoder with {} providers from spec bundle",
            provider_count
        );

        Self {
            spec_registry,
            legacy_registry: ProviderRegistry::new(),
            partial_requests: RwLock::new(HashMap::new()),
            partial_responses: RwLock::new(HashMap::new()),
            pending_requests: RwLock::new(HashMap::new()),
            stream_reassemblers: RwLock::new(HashMap::new()),
            anthropic_reassemblers: RwLock::new(HashMap::new()),
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    /// Create a decoder with a specific spec loader (for testing or custom bundles)
    pub fn with_spec_loader(spec_loader: &SpecLoader) -> Self {
        let spec_registry = Arc::new(DynamicProviderRegistry::new(spec_loader.bundle()));
        let provider_count = spec_registry.provider_ids().len();
        info!(
            "Initialized HttpDecoder with {} providers from custom spec bundle",
            provider_count
        );

        Self {
            spec_registry,
            legacy_registry: ProviderRegistry::new(),
            partial_requests: RwLock::new(HashMap::new()),
            partial_responses: RwLock::new(HashMap::new()),
            pending_requests: RwLock::new(HashMap::new()),
            stream_reassemblers: RwLock::new(HashMap::new()),
            anthropic_reassemblers: RwLock::new(HashMap::new()),
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    /// Cleanup stale pending requests periodically
    fn maybe_cleanup(&self) {
        let should_cleanup = {
            let last = self.last_cleanup.read().unwrap();
            last.elapsed() > Duration::from_secs(60) // Cleanup every minute
        };

        if should_cleanup {
            self.cleanup_stale_requests();
            *self.last_cleanup.write().unwrap() = Instant::now();
        }
    }

    fn cleanup_stale_requests(&self) {
        let now = Instant::now();

        // Cleanup partial requests
        {
            let mut partial = self.partial_requests.write().unwrap();
            partial.retain(|_, req| now.duration_since(req.created_at) < PENDING_REQUEST_TIMEOUT);
        }

        // Cleanup partial responses
        {
            let mut partial = self.partial_responses.write().unwrap();
            partial.retain(|_, resp| now.duration_since(resp.created_at) < PENDING_REQUEST_TIMEOUT);
        }

        // Cleanup pending requests
        {
            let mut pending = self.pending_requests.write().unwrap();
            let before = pending.len();
            pending.retain(|_, req| now.duration_since(req.created_at) < PENDING_REQUEST_TIMEOUT);
            let removed = before - pending.len();
            if removed > 0 {
                debug!("Cleaned up {} stale pending requests", removed);
            }
        }

        // Cleanup stream reassemblers (keep for 5 minutes)
        {
            let mut reassemblers = self.stream_reassemblers.write().unwrap();
            if reassemblers.len() > MAX_PENDING_REQUESTS {
                warn!(
                    "Too many stream reassemblers ({}), clearing oldest",
                    reassemblers.len()
                );
                reassemblers.clear();
            }
        }

        {
            let mut reassemblers = self.anthropic_reassemblers.write().unwrap();
            if reassemblers.len() > MAX_PENDING_REQUESTS {
                warn!(
                    "Too many Anthropic reassemblers ({}), clearing oldest",
                    reassemblers.len()
                );
                reassemblers.clear();
            }
        }
    }

    fn decode_ssl_write(&self, raw: &RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        self.maybe_cleanup();
        let mut events = Vec::new();

        let key = CorrelationKey::from_event(raw);

        // Check if we have an existing partial request for this connection
        let is_new_request = is_http_request(&raw.data);
        let reassembler_opt = {
            let mut partial = self.partial_requests.write().unwrap();
            if is_new_request {
                // New request starting - replace any old one for this key
                let reassembler = RequestReassembler::new(&raw.data);
                partial.insert(key.clone(), reassembler);
                partial.get(&key).cloned()
            } else {
                // Not a new request, see if it's a continuation of a partial one
                if let Some(reassembler) = partial.get_mut(&key) {
                    reassembler.feed(&raw.data);
                    Some(reassembler.clone())
                } else {
                    None
                }
            }
        };

        let reassembler = match reassembler_opt {
            Some(r) => r,
            None => {
                info!("SSL write is not HTTP request and no partial request found, skipping (data starts with: {:?})", 
                    String::from_utf8_lossy(&raw.data[..std::cmp::min(raw.data.len(), 20)]));
                return Ok(events);
            }
        };

        if !reassembler.is_complete() {
            debug!(
                "HTTP request not yet complete, buffering (current size: {} bytes)",
                reassembler.buffer.len()
            );
            return Ok(events);
        }

        // Request is complete! Remove from partials and proceed to decode
        self.partial_requests.write().unwrap().remove(&key);

        let http_req = match parse_request(&reassembler.buffer) {
            Some(req) => req,
            None => {
                info!("Failed to parse reassembled HTTP request");
                return Ok(events);
            }
        };

        // Check if this is an AI provider using spec-driven detection first
        let domain = http_req.host.as_deref().unwrap_or("");

        // Use spec-driven detection (95+ providers from spec bundle)
        let provider_id = match self.spec_registry.detect_from_domain(domain) {
            Some(id) => {
                debug!(
                    "Spec registry detected provider '{}' for domain {}",
                    id, domain
                );
                id.to_string()
            }
            None => {
                // Fall back to legacy registry for backward compatibility
                match self.legacy_registry.detect_from_domain(domain) {
                    Some(p) => {
                        debug!(
                            "Legacy registry detected provider {:?} for domain {}",
                            p, domain
                        );
                        format!("{:?}", p).to_lowercase()
                    }
                    None => {
                        debug!("Domain {} is not a known AI provider", domain);
                        return Ok(events);
                    }
                }
            }
        };

        // Convert to Provider enum for existing code paths (backward compatibility)
        let provider = self
            .legacy_registry
            .detect_from_domain(domain)
            .unwrap_or(Provider::Unknown);

        debug!(
            "Detected AI provider: id='{}', enum={:?} for domain {}",
            provider_id, provider, domain
        );

        // Try to parse body as JSON
        let body = match &http_req.body {
            Some(b) => b,
            None => {
                trace!("No body in HTTP request");
                return Ok(events);
            }
        };

        let json: serde_json::Value = match serde_json::from_slice(body) {
            Ok(j) => j,
            Err(e) => {
                trace!("Failed to parse request body as JSON: {}", e);
                return Ok(events);
            }
        };

        if !is_ai_request(&json) {
            trace!("Request does not look like an AI request");
            return Ok(events);
        }

        let endpoint = format!("https://{}{}", domain, http_req.path);

        // Parse request based on provider
        let request_data = match provider {
            Provider::Anthropic => parse_anthropic_request(&json, &endpoint),
            _ => parse_ai_request(&json, provider, &endpoint),
        };

        let request_data = match request_data {
            Some(data) => data,
            None => {
                trace!("Failed to parse AI request data");
                return Ok(events);
            }
        };

        let envelope = self.create_envelope(raw, "ai.request");
        let is_streaming = request_data.streaming.unwrap_or(false);

        // Store for response correlation
        {
            let mut pending = self.pending_requests.write().unwrap();

            // Enforce max pending requests
            if pending.len() >= MAX_PENDING_REQUESTS {
                warn!("Max pending requests reached, removing oldest");
                // Find and remove the oldest
                if let Some(oldest_key) = pending
                    .iter()
                    .min_by_key(|(_, r)| r.created_at)
                    .map(|(k, _)| k.clone())
                {
                    pending.remove(&oldest_key);
                }
            }

            pending.insert(
                key.clone(),
                PendingRequest {
                    request_id: request_data.request_id.clone(),
                    request_data: request_data.clone(),
                    timestamp: envelope.ts,
                    created_at: Instant::now(),
                    provider,
                    is_streaming,
                    host: http_req.host.clone(),
                },
            );
        }

        debug!(
            "Parsed AI request: model={:?}, provider={:?}, streaming={}",
            request_data.model.as_ref().map(|m| &m.id),
            provider,
            is_streaming
        );

        events.push(OispEvent::AiRequest(AiRequestEvent {
            envelope,
            data: request_data,
        }));

        Ok(events)
    }

    fn decode_ssl_read(&self, raw: &RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        self.maybe_cleanup();
        let mut events = Vec::new();

        let key = CorrelationKey::from_event(raw);

        // 1. Check for existing partial response
        let is_new_response = is_http_response(&raw.data);

        info!(
            "decode_ssl_read: pid={}, tid={:?}, fd={:?}, is_new_response={}, data_len={}, data_start={:?}",
            key.pid,
            key.tid,
            key.fd,
            is_new_response,
            raw.data.len(),
            String::from_utf8_lossy(&raw.data[..std::cmp::min(50, raw.data.len())])
        );

        let reassembler_opt: Option<ResponseReassembler> = {
            let mut partials = self.partial_responses.write().unwrap();

            // Log all current partial responses
            info!(
                "Current partial_responses count: {}, keys: {:?}",
                partials.len(),
                partials.keys().collect::<Vec<_>>()
            );

            if is_new_response {
                if let Some(http_resp) = parse_response(&raw.data) {
                    info!("New HTTP response: status={}, is_chunked={}, is_gzipped={}, content_length={:?}",
                        http_resp.status_code, http_resp.is_chunked, http_resp.is_gzipped, http_resp.content_length);
                    let reassembler = ResponseReassembler::new(http_resp);
                    partials.insert(key.clone(), reassembler);
                    partials.get(&key).cloned()
                } else {
                    info!("Failed to parse HTTP response");
                    None
                }
            } else if let Some(reassembler) = partials.get_mut(&key) {
                info!(
                    "Feeding {} bytes to existing reassembler for key {:?}",
                    raw.data.len(),
                    key
                );
                reassembler.feed(&raw.data);
                Some(reassembler.clone())
            } else {
                // Try without fd as fallback
                let key_no_fd = CorrelationKey {
                    pid: key.pid,
                    tid: key.tid,
                    fd: None,
                };
                if let Some(reassembler) = partials.get_mut(&key_no_fd) {
                    info!(
                        "Feeding {} bytes to reassembler via key_no_fd {:?}",
                        raw.data.len(),
                        key_no_fd
                    );
                    reassembler.feed(&raw.data);
                    Some(reassembler.clone())
                } else {
                    info!(
                        "No matching reassembler found for key {:?} or key_no_fd {:?}",
                        key, key_no_fd
                    );
                    None
                }
            }
        };

        // 2. If we have a reassembler, check if it's complete
        if let Some(mut reassembler) = reassembler_opt {
            info!(
                "Response reassembler: body_buffer_len={}, is_complete={}",
                reassembler.body_buffer.len(),
                reassembler.is_complete()
            );

            if reassembler.is_complete() {
                info!(
                    "Response COMPLETE for pid={}, buffer ends with: {:?}",
                    key.pid,
                    &reassembler.body_buffer[reassembler.body_buffer.len().saturating_sub(10)..]
                );

                // Remove from partials
                self.partial_responses.write().unwrap().remove(&key);

                // Find the matching pending request
                let pending_opt = {
                    let pending = self.pending_requests.read().unwrap();
                    pending
                        .get(&key)
                        .cloned()
                        .or_else(|| pending.get(&key.without_tid()).cloned())
                };

                if let Some(pending_req) = pending_opt {
                    info!(
                        "Found pending request for response: request_id={}",
                        pending_req.request_id
                    );
                    // Decompress body if needed
                    reassembler.decompress_if_needed();

                    // Update headers with full body
                    let mut full_resp = reassembler.headers;
                    full_resp.body = Some(reassembler.body_buffer);

                    if full_resp.is_streaming || pending_req.is_streaming {
                        self.handle_streaming_response(
                            &key,
                            &pending_req,
                            &full_resp.body,
                            raw,
                            &mut events,
                        );
                    } else {
                        self.handle_complete_response(
                            &key,
                            &pending_req,
                            &full_resp,
                            raw,
                            &mut events,
                        );
                    }
                }
            }
            return Ok(events);
        }

        // 3. Fallback for unexpected data or AI-specific streaming
        let pending_opt = {
            let pending = self.pending_requests.read().unwrap();
            pending
                .get(&key)
                .cloned()
                .or_else(|| pending.get(&key.without_tid()).cloned())
        };

        if let Some(pending_req) = pending_opt {
            if pending_req.is_streaming {
                self.handle_streaming_chunk(&key, &pending_req, &raw.data, raw, &mut events);
            }
        }

        Ok(events)
    }

    fn handle_streaming_response(
        &self,
        key: &CorrelationKey,
        pending_req: &PendingRequest,
        body: &Option<Vec<u8>>,
        raw: &RawCaptureEvent,
        events: &mut Vec<OispEvent>,
    ) {
        let body = match body {
            Some(b) => b,
            None => return,
        };

        match pending_req.provider {
            Provider::Anthropic => {
                let mut reassemblers = self.anthropic_reassemblers.write().unwrap();
                let reassembler = reassemblers.entry(key.clone()).or_default();
                reassembler.feed(body);

                if reassembler.is_complete() {
                    // Build complete response
                    let envelope = self.create_envelope(raw, "ai.response");
                    let latency = envelope.ts - pending_req.timestamp;

                    let (input_tokens, output_tokens) = reassembler.usage();

                    let response_data = AiResponseData {
                        request_id: pending_req.request_id.clone(),
                        provider_request_id: None,
                        provider: pending_req.request_data.provider.clone(),
                        model: pending_req.request_data.model.clone(),
                        status_code: Some(200),
                        success: Some(true),
                        error: None,
                        choices: vec![Choice {
                            index: 0,
                            message: Some(Message {
                                role: MessageRole::Assistant,
                                content: Some(MessageContent::Text(
                                    reassembler.content().to_string(),
                                )),
                                content_hash: None,
                                content_length: Some(reassembler.content().len()),
                                has_images: None,
                                image_count: None,
                                tool_call_id: None,
                                name: None,
                            }),
                            finish_reason: reassembler.stop_reason().map(|r| match r {
                                "end_turn" => FinishReason::Stop,
                                "max_tokens" => FinishReason::Length,
                                "tool_use" => FinishReason::ToolCalls,
                                _ => FinishReason::Other,
                            }),
                        }],
                        tool_calls: Vec::new(),
                        tool_calls_count: Some(0),
                        usage: Some(Usage {
                            prompt_tokens: input_tokens,
                            completion_tokens: output_tokens,
                            total_tokens: match (input_tokens, output_tokens) {
                                (Some(i), Some(o)) => Some(i + o),
                                _ => None,
                            },
                            cached_tokens: None,
                            reasoning_tokens: None,
                            input_cost_usd: None,
                            output_cost_usd: None,
                            total_cost_usd: None,
                        }),
                        latency_ms: Some(latency.num_milliseconds() as u64),
                        time_to_first_token_ms: None,
                        was_cached: None,
                        finish_reason: reassembler.stop_reason().map(|r| match r {
                            "end_turn" => FinishReason::Stop,
                            _ => FinishReason::Other,
                        }),
                        thinking: None, // Streaming doesn't capture thinking blocks yet
                    };

                    events.push(OispEvent::AiResponse(AiResponseEvent {
                        envelope,
                        data: response_data,
                    }));

                    // Cleanup
                    reassemblers.remove(key);
                    self.pending_requests.write().unwrap().remove(key);
                }
            }
            _ => {
                // OpenAI-style streaming
                let mut reassemblers = self.stream_reassemblers.write().unwrap();
                let reassembler = reassemblers.entry(key.clone()).or_default();
                reassembler.feed(body);

                if reassembler.is_complete() {
                    let envelope = self.create_envelope(raw, "ai.response");
                    let latency = envelope.ts - pending_req.timestamp;

                    let response_data = AiResponseData {
                        request_id: pending_req.request_id.clone(),
                        provider_request_id: None,
                        provider: pending_req.request_data.provider.clone(),
                        model: pending_req.request_data.model.clone(),
                        status_code: Some(200),
                        success: Some(true),
                        error: None,
                        choices: vec![Choice {
                            index: 0,
                            message: Some(Message {
                                role: MessageRole::Assistant,
                                content: Some(MessageContent::Text(
                                    reassembler.content().to_string(),
                                )),
                                content_hash: None,
                                content_length: Some(reassembler.content().len()),
                                has_images: None,
                                image_count: None,
                                tool_call_id: None,
                                name: None,
                            }),
                            finish_reason: reassembler.finish_reason().map(|r| match r {
                                "stop" => FinishReason::Stop,
                                "length" => FinishReason::Length,
                                "tool_calls" => FinishReason::ToolCalls,
                                _ => FinishReason::Other,
                            }),
                        }],
                        tool_calls: Vec::new(),
                        tool_calls_count: Some(0),
                        usage: None, // Streaming responses often don't include usage
                        latency_ms: Some(latency.num_milliseconds() as u64),
                        time_to_first_token_ms: None,
                        was_cached: None,
                        finish_reason: reassembler.finish_reason().map(|r| match r {
                            "stop" => FinishReason::Stop,
                            _ => FinishReason::Other,
                        }),
                        thinking: None, // Streaming doesn't capture thinking blocks yet
                    };

                    events.push(OispEvent::AiResponse(AiResponseEvent {
                        envelope,
                        data: response_data,
                    }));

                    // Cleanup
                    reassemblers.remove(key);
                    self.pending_requests.write().unwrap().remove(key);
                }
            }
        }
    }

    fn handle_streaming_chunk(
        &self,
        key: &CorrelationKey,
        pending_req: &PendingRequest,
        data: &[u8],
        raw: &RawCaptureEvent,
        events: &mut Vec<OispEvent>,
    ) {
        // Feed to appropriate reassembler based on provider
        match pending_req.provider {
            Provider::Anthropic => {
                let mut reassemblers = self.anthropic_reassemblers.write().unwrap();
                let reassembler = reassemblers.entry(key.clone()).or_default();
                reassembler.feed(data);
                // Check completion similar to above
                if reassembler.is_complete() {
                    // Build and emit response (same logic as above)
                    let envelope = self.create_envelope(raw, "ai.response");
                    let latency = envelope.ts - pending_req.timestamp;

                    let (input_tokens, output_tokens) = reassembler.usage();

                    let response_data = AiResponseData {
                        request_id: pending_req.request_id.clone(),
                        provider_request_id: None,
                        provider: pending_req.request_data.provider.clone(),
                        model: pending_req.request_data.model.clone(),
                        status_code: Some(200),
                        success: Some(true),
                        error: None,
                        choices: vec![Choice {
                            index: 0,
                            message: Some(Message {
                                role: MessageRole::Assistant,
                                content: Some(MessageContent::Text(
                                    reassembler.content().to_string(),
                                )),
                                content_hash: None,
                                content_length: Some(reassembler.content().len()),
                                has_images: None,
                                image_count: None,
                                tool_call_id: None,
                                name: None,
                            }),
                            finish_reason: Some(FinishReason::Stop),
                        }],
                        tool_calls: Vec::new(),
                        tool_calls_count: Some(0),
                        usage: Some(Usage {
                            prompt_tokens: input_tokens,
                            completion_tokens: output_tokens,
                            total_tokens: match (input_tokens, output_tokens) {
                                (Some(i), Some(o)) => Some(i + o),
                                _ => None,
                            },
                            cached_tokens: None,
                            reasoning_tokens: None,
                            input_cost_usd: None,
                            output_cost_usd: None,
                            total_cost_usd: None,
                        }),
                        latency_ms: Some(latency.num_milliseconds() as u64),
                        time_to_first_token_ms: None,
                        was_cached: None,
                        finish_reason: Some(FinishReason::Stop),
                        thinking: None, // Streaming doesn't capture thinking blocks yet
                    };

                    events.push(OispEvent::AiResponse(AiResponseEvent {
                        envelope,
                        data: response_data,
                    }));

                    reassemblers.remove(key);
                    self.pending_requests.write().unwrap().remove(key);
                }
            }
            _ => {
                let mut reassemblers = self.stream_reassemblers.write().unwrap();
                let reassembler = reassemblers.entry(key.clone()).or_default();
                reassembler.feed(data);
            }
        }
    }

    fn handle_complete_response(
        &self,
        key: &CorrelationKey,
        pending_req: &PendingRequest,
        http_resp: &crate::http::ParsedHttpResponse,
        raw: &RawCaptureEvent,
        events: &mut Vec<OispEvent>,
    ) {
        info!(
            "handle_complete_response: status={}, body_len={:?}",
            http_resp.status_code,
            http_resp.body.as_ref().map(|b| b.len())
        );

        let body = match &http_resp.body {
            Some(b) => b,
            None => {
                info!("handle_complete_response: No body, returning");
                return;
            }
        };

        let json: serde_json::Value = match serde_json::from_slice(body) {
            Ok(j) => j,
            Err(e) => {
                info!("handle_complete_response: JSON parse FAILED: {}", e);
                info!(
                    "Body preview: {:?}",
                    String::from_utf8_lossy(&body[..std::cmp::min(body.len(), 200)])
                );
                return;
            }
        };

        info!("handle_complete_response: JSON parsed successfully");

        // Detect provider from body or use the one from request
        let provider = detect_provider_from_body(&json).unwrap_or(pending_req.provider);

        let response_data = match provider {
            Provider::Anthropic => parse_anthropic_response(&json, &pending_req.request_id),
            _ => parse_ai_response(&json, &pending_req.request_id, provider),
        };

        let response_data = match response_data {
            Some(data) => data,
            None => {
                trace!("Failed to parse AI response data");
                return;
            }
        };

        let envelope = self.create_envelope(raw, "ai.response");
        let latency = envelope.ts - pending_req.timestamp;

        let mut response_data = response_data;
        response_data.latency_ms = Some(latency.num_milliseconds() as u64);
        response_data.status_code = Some(http_resp.status_code);

        debug!(
            "Parsed AI response: status={}, latency={}ms",
            http_resp.status_code,
            latency.num_milliseconds()
        );

        events.push(OispEvent::AiResponse(AiResponseEvent {
            envelope,
            data: response_data,
        }));

        // Cleanup
        self.pending_requests.write().unwrap().remove(key);
    }

    fn decode_process_exec(&self, raw: &RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        let envelope = self.create_envelope(raw, "process.exec");

        let data = ProcessExecData {
            exe: raw.metadata.exe.clone().unwrap_or_default(),
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            interpreter: None,
            script_path: None,
            is_shell: None,
            is_script: None,
            is_interactive: None,
            binary_hash: None,
            code_signature: None,
        };

        Ok(vec![OispEvent::ProcessExec(ProcessExecEvent {
            envelope,
            data,
        })])
    }

    fn decode_network_connect(&self, raw: &RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        let envelope = self.create_envelope(raw, "network.connect");

        let data = NetworkConnectData {
            dest: Endpoint {
                ip: raw.metadata.remote_addr.clone(),
                port: raw.metadata.remote_port,
                domain: None,
                is_private: None,
                geo: None,
            },
            src: Some(Endpoint {
                ip: raw.metadata.local_addr.clone(),
                port: raw.metadata.local_port,
                domain: None,
                is_private: None,
                geo: None,
            }),
            protocol: Some(Protocol::Tcp),
            success: Some(true),
            error: None,
            latency_ms: None,
            tls: None,
        };

        Ok(vec![OispEvent::NetworkConnect(NetworkConnectEvent {
            envelope,
            data,
        })])
    }

    fn create_envelope(&self, raw: &RawCaptureEvent, event_type: &str) -> EventEnvelope {
        let mut envelope = EventEnvelope::new(event_type);
        envelope.ts = chrono::Utc::now();
        envelope.ts_mono = Some(raw.timestamp_ns);

        envelope.process = Some(ProcessInfo {
            pid: raw.pid,
            ppid: raw.metadata.ppid,
            exe: raw.metadata.exe.clone(),
            name: raw.metadata.comm.clone(),
            cmdline: None,
            cwd: None,
            tid: raw.tid,
            container_id: None,
            hash: None,
            code_signature: None,
        });

        envelope.actor = raw.metadata.uid.map(|uid| Actor {
            uid: Some(uid),
            user: None,
            gid: None,
            session_id: None,
            identity: None,
        });

        envelope.source = Source {
            collector: "oisp-sensor".to_string(),
            collector_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            capture_method: Some(CaptureMethod::TlsBoundary),
            capture_point: match &raw.kind {
                RawEventKind::SslWrite => Some("ssl_write".to_string()),
                RawEventKind::SslRead => Some("ssl_read".to_string()),
                _ => None,
            },
            sensor_host: None,
        };

        envelope.confidence = Confidence {
            level: ConfidenceLevel::High,
            completeness: Completeness::Full,
            reasons: vec!["tls_boundary_capture".to_string()],
            content_source: Some("tls_boundary".to_string()),
            ai_detection_method: Some("known_endpoint".to_string()),
        };

        envelope
    }

    /// Get statistics about decoder state
    pub fn stats(&self) -> DecoderStats {
        DecoderStats {
            pending_requests: self.pending_requests.read().unwrap().len(),
            stream_reassemblers: self.stream_reassemblers.read().unwrap().len(),
            anthropic_reassemblers: self.anthropic_reassemblers.read().unwrap().len(),
        }
    }
}

/// Statistics about the decoder's internal state
#[derive(Debug, Clone)]
pub struct DecoderStats {
    pub pending_requests: usize,
    pub stream_reassemblers: usize,
    pub anthropic_reassemblers: usize,
}

impl Default for HttpDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginInfo for HttpDecoder {
    fn name(&self) -> &str {
        "http-decoder"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "HTTP/SSE decoder with AI provider fingerprinting"
    }
}

impl Plugin for HttpDecoder {
    fn init(&mut self, _config: &PluginConfig) -> PluginResult<()> {
        // Could add configuration options here
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
impl DecodePlugin for HttpDecoder {
    fn can_decode(&self, raw: &RawCaptureEvent) -> bool {
        matches!(
            raw.kind,
            RawEventKind::SslWrite
                | RawEventKind::SslRead
                | RawEventKind::ProcessExec
                | RawEventKind::NetworkConnect
        )
    }

    async fn decode(&self, raw: RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        match raw.kind {
            RawEventKind::SslWrite => self.decode_ssl_write(&raw),
            RawEventKind::SslRead => self.decode_ssl_read(&raw),
            RawEventKind::ProcessExec => self.decode_process_exec(&raw),
            RawEventKind::NetworkConnect => self.decode_network_connect(&raw),
            _ => Ok(Vec::new()),
        }
    }

    fn priority(&self) -> i32 {
        100 // High priority for HTTP decoder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oisp_core::plugins::RawEventMetadata;

    fn create_raw_event(kind: RawEventKind, data: &[u8], pid: u32) -> RawCaptureEvent {
        RawCaptureEvent {
            id: ulid::Ulid::new().to_string(),
            timestamp_ns: 1000000000,
            kind,
            pid,
            tid: Some(1),
            data: data.to_vec(),
            metadata: RawEventMetadata {
                comm: Some("test".to_string()),
                uid: Some(1000),
                fd: Some(5),
                ..Default::default()
            },
        }
    }

    #[tokio::test]
    async fn test_decode_openai_request() {
        let decoder = HttpDecoder::new();

        let request = b"POST /v1/chat/completions HTTP/1.1\r\n\
                        Host: api.openai.com\r\n\
                        Content-Type: application/json\r\n\
                        \r\n\
                        {\"model\":\"gpt-4\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}]}";

        let raw = create_raw_event(RawEventKind::SslWrite, request, 1234);
        let events = decoder.decode(raw).await.unwrap();

        assert_eq!(events.len(), 1);
        if let OispEvent::AiRequest(req) = &events[0] {
            assert_eq!(req.data.model.as_ref().unwrap().id, "gpt-4");
            assert_eq!(req.data.messages.len(), 1);
        } else {
            panic!("Expected AiRequest event");
        }

        // Check that request is tracked
        let stats = decoder.stats();
        assert_eq!(stats.pending_requests, 1);
    }

    #[tokio::test]
    async fn test_decode_openai_response() {
        let decoder = HttpDecoder::new();

        // First send request
        let request = b"POST /v1/chat/completions HTTP/1.1\r\n\
                        Host: api.openai.com\r\n\
                        Content-Type: application/json\r\n\
                        \r\n\
                        {\"model\":\"gpt-4\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}]}";

        let raw_req = create_raw_event(RawEventKind::SslWrite, request, 1234);
        decoder.decode(raw_req).await.unwrap();

        // Then send response
        let response = b"HTTP/1.1 200 OK\r\n\
                         Content-Type: application/json\r\n\
                         \r\n\
                         {\"id\":\"chatcmpl-123\",\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"message\":{\"role\":\"assistant\",\"content\":\"Hi!\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}";

        let raw_resp = create_raw_event(RawEventKind::SslRead, response, 1234);
        let events = decoder.decode(raw_resp).await.unwrap();

        assert_eq!(events.len(), 1);
        if let OispEvent::AiResponse(resp) = &events[0] {
            assert!(resp.data.latency_ms.is_some());
            assert_eq!(resp.data.status_code, Some(200));
            assert_eq!(resp.data.choices.len(), 1);
            assert_eq!(resp.data.finish_reason, Some(FinishReason::Stop));
        } else {
            panic!("Expected AiResponse event");
        }

        // Request should be cleaned up
        let stats = decoder.stats();
        assert_eq!(stats.pending_requests, 0);
    }

    #[tokio::test]
    async fn test_decode_anthropic_request() {
        let decoder = HttpDecoder::new();

        let request = b"POST /v1/messages HTTP/1.1\r\n\
                        Host: api.anthropic.com\r\n\
                        Content-Type: application/json\r\n\
                        \r\n\
                        {\"model\":\"claude-3-opus-20240229\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}],\"max_tokens\":1024}";

        let raw = create_raw_event(RawEventKind::SslWrite, request, 1234);
        let events = decoder.decode(raw).await.unwrap();

        assert_eq!(events.len(), 1);
        if let OispEvent::AiRequest(req) = &events[0] {
            assert!(req.data.model.as_ref().unwrap().id.starts_with("claude"));
        } else {
            panic!("Expected AiRequest event");
        }
    }

    #[tokio::test]
    async fn test_correlation_by_pid() {
        let decoder = HttpDecoder::new();

        // Request from process 1234
        let request = b"POST /v1/chat/completions HTTP/1.1\r\n\
                        Host: api.openai.com\r\n\
                        Content-Type: application/json\r\n\
                        \r\n\
                        {\"model\":\"gpt-4\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}]}";

        let raw_req = create_raw_event(RawEventKind::SslWrite, request, 1234);
        decoder.decode(raw_req).await.unwrap();

        // Response from different process should not correlate
        let response = b"HTTP/1.1 200 OK\r\n\
                         Content-Type: application/json\r\n\
                         \r\n\
                         {\"id\":\"chatcmpl-123\",\"model\":\"gpt-4\",\"choices\":[]}";

        let raw_resp = create_raw_event(RawEventKind::SslRead, response, 5678);
        let events = decoder.decode(raw_resp).await.unwrap();

        // Should not produce any events (no matching request)
        assert_eq!(events.len(), 0);

        // Original request should still be pending
        let stats = decoder.stats();
        assert_eq!(stats.pending_requests, 1);
    }

    #[tokio::test]
    async fn test_non_ai_request_ignored() {
        let decoder = HttpDecoder::new();

        // Regular HTTP request to non-AI domain
        let request = b"GET /index.html HTTP/1.1\r\n\
                        Host: example.com\r\n\
                        \r\n";

        let raw = create_raw_event(RawEventKind::SslWrite, request, 1234);
        let events = decoder.decode(raw).await.unwrap();

        assert_eq!(events.len(), 0);
    }
}
