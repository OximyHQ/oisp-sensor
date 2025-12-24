//! HTTP parsing utilities
//!
//! Provides parsing for HTTP/1.1 requests and responses captured from SSL/TLS data.

use std::collections::HashMap;

/// Parsed HTTP request
#[derive(Debug, Clone)]
pub struct ParsedHttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub host: Option<String>,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
    /// Whether the request body uses chunked transfer encoding
    pub is_chunked: bool,
}

/// Parsed HTTP response
#[derive(Debug, Clone)]
pub struct ParsedHttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
    pub is_streaming: bool,
    /// Whether the response body uses chunked transfer encoding
    pub is_chunked: bool,
    /// Whether the response body is gzipped
    pub is_gzipped: bool,
}

/// Parse HTTP request from bytes
pub fn parse_request(data: &[u8]) -> Option<ParsedHttpRequest> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);

    match req.parse(data) {
        Ok(httparse::Status::Complete(header_len)) => {
            let mut header_map = HashMap::new();
            for header in req.headers.iter() {
                let name = header.name.to_lowercase();
                let value = String::from_utf8_lossy(header.value).to_string();
                header_map.insert(name, value);
            }

            let is_chunked = header_map
                .get("transfer-encoding")
                .map(|v| v.to_lowercase().contains("chunked"))
                .unwrap_or(false);

            let content_length = header_map
                .get("content-length")
                .and_then(|v| v.parse().ok());

            // Extract body - handle chunked vs content-length
            let body = if header_len < data.len() {
                let raw_body = &data[header_len..];
                if is_chunked {
                    decode_chunked_body(raw_body)
                } else {
                    Some(raw_body.to_vec())
                }
            } else {
                None
            };

            Some(ParsedHttpRequest {
                method: req.method?.to_string(),
                path: req.path?.to_string(),
                version: format!("HTTP/1.{}", req.version?),
                host: header_map.get("host").cloned(),
                content_type: header_map.get("content-type").cloned(),
                content_length,
                is_chunked,
                headers: header_map,
                body,
            })
        }
        Ok(httparse::Status::Partial) => None,
        Err(_) => None,
    }
}

/// Parse HTTP response from bytes
pub fn parse_response(data: &[u8]) -> Option<ParsedHttpResponse> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut resp = httparse::Response::new(&mut headers);

    match resp.parse(data) {
        Ok(httparse::Status::Complete(header_len)) => {
            let mut header_map = HashMap::new();
            for header in resp.headers.iter() {
                let name = header.name.to_lowercase();
                let value = String::from_utf8_lossy(header.value).to_string();
                header_map.insert(name, value);
            }

            let content_type = header_map.get("content-type").cloned();

            let is_streaming = content_type
                .as_ref()
                .map(|ct| {
                    ct.contains("text/event-stream")
                        || ct.contains("application/x-ndjson")
                        || ct.contains("application/stream+json")
                })
                .unwrap_or(false);

            let is_chunked = header_map
                .get("transfer-encoding")
                .map(|v| v.to_lowercase().contains("chunked"))
                .unwrap_or(false);

            let is_gzipped = header_map
                .get("content-encoding")
                .map(|v| v.to_lowercase().contains("gzip"))
                .unwrap_or(false);

            let content_length = header_map
                .get("content-length")
                .and_then(|v| v.parse().ok());

            // Extract body - keep raw body for reassembly
            let body = if header_len < data.len() {
                let raw_body = &data[header_len..];
                Some(raw_body.to_vec())
            } else {
                None
            };

            Some(ParsedHttpResponse {
                status_code: resp.code?,
                status_text: resp.reason?.to_string(),
                version: format!("HTTP/1.{}", resp.version?),
                content_type,
                content_length,
                is_streaming,
                is_chunked,
                is_gzipped,
                headers: header_map,
                body,
            })
        }
        Ok(httparse::Status::Partial) => None,
        Err(_) => None,
    }
}

/// Decode chunked transfer encoding
///
/// Format: <size in hex>\r\n<data>\r\n...<0>\r\n\r\n
pub fn decode_chunked_body(data: &[u8]) -> Option<Vec<u8>> {
    let mut result = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // Find the chunk size line (ends with \r\n)
        let size_end = find_crlf(&data[pos..])?;
        let size_line = &data[pos..pos + size_end];

        // Parse chunk size (hex)
        let size_str = std::str::from_utf8(size_line).ok()?;
        // Handle chunk extensions (size;extension=value)
        let size_hex = size_str.split(';').next()?.trim();
        let chunk_size = usize::from_str_radix(size_hex, 16).ok()?;

        pos += size_end + 2; // Skip size line and CRLF

        if chunk_size == 0 {
            // Final chunk
            break;
        }

        // Read chunk data
        if pos + chunk_size > data.len() {
            // Incomplete chunk - just return what we have so far
            result.extend_from_slice(&data[pos..]);
            break;
        }

        result.extend_from_slice(&data[pos..pos + chunk_size]);
        pos += chunk_size;

        // Skip trailing CRLF
        if pos + 2 <= data.len() && &data[pos..pos + 2] == b"\r\n" {
            pos += 2;
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Find position of \r\n in data
fn find_crlf(data: &[u8]) -> Option<usize> {
    (0..data.len().saturating_sub(1)).find(|&i| data[i] == b'\r' && data[i + 1] == b'\n')
}

/// Check if data looks like an HTTP request
pub fn is_http_request(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }

    // Check for common HTTP methods
    let prefixes = [
        b"GET ".as_slice(),
        b"POST".as_slice(),
        b"PUT ".as_slice(),
        b"DELETE".as_slice(),
        b"HEAD".as_slice(),
        b"OPTIONS".as_slice(),
        b"PATCH".as_slice(),
        b"CONNECT".as_slice(),
        b"TRACE".as_slice(),
    ];

    prefixes.iter().any(|prefix| data.starts_with(prefix))
}

/// Check if data looks like an HTTP response
pub fn is_http_response(data: &[u8]) -> bool {
    data.starts_with(b"HTTP/")
}

/// Extract the body from potentially incomplete HTTP data
/// Useful for streaming responses where we may not have the full body
pub fn extract_partial_body(data: &[u8]) -> Option<&[u8]> {
    // Find end of headers
    let patterns = [b"\r\n\r\n".as_slice(), b"\n\n".as_slice()];

    for pattern in &patterns {
        if let Some(pos) = find_subsequence(data, pattern) {
            let body_start = pos + pattern.len();
            if body_start < data.len() {
                return Some(&data[body_start..]);
            }
        }
    }

    None
}

/// Find a subsequence in data
fn find_subsequence(data: &[u8], pattern: &[u8]) -> Option<usize> {
    if pattern.is_empty() || data.len() < pattern.len() {
        return None;
    }

    (0..=data.len() - pattern.len()).find(|&i| &data[i..i + pattern.len()] == pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_http_request() {
        assert!(is_http_request(b"GET / HTTP/1.1\r\n"));
        assert!(is_http_request(b"POST /api/v1/chat HTTP/1.1\r\n"));
        assert!(is_http_request(b"DELETE /resource HTTP/1.1\r\n"));
        assert!(!is_http_request(b"HTTP/1.1 200 OK\r\n"));
        assert!(!is_http_request(b"random data"));
        assert!(!is_http_request(b"GE")); // Too short
    }

    #[test]
    fn test_is_http_response() {
        assert!(is_http_response(b"HTTP/1.1 200 OK\r\n"));
        assert!(is_http_response(b"HTTP/1.0 404 Not Found\r\n"));
        assert!(!is_http_response(b"GET / HTTP/1.1\r\n"));
        assert!(!is_http_response(b"random data"));
    }

    #[test]
    fn test_parse_request() {
        let request = b"POST /v1/chat/completions HTTP/1.1\r\n\
                        Host: api.openai.com\r\n\
                        Content-Type: application/json\r\n\
                        Content-Length: 27\r\n\
                        \r\n\
                        {\"model\":\"gpt-4\",\"test\":1}";

        let parsed = parse_request(request).unwrap();
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.path, "/v1/chat/completions");
        assert_eq!(parsed.host, Some("api.openai.com".to_string()));
        assert_eq!(parsed.content_type, Some("application/json".to_string()));
        assert_eq!(parsed.content_length, Some(27));
        assert!(!parsed.is_chunked);
        assert!(parsed.body.is_some());

        let body = String::from_utf8_lossy(parsed.body.as_ref().unwrap());
        assert!(body.contains("gpt-4"));
    }

    #[test]
    fn test_parse_response() {
        let response = b"HTTP/1.1 200 OK\r\n\
                         Content-Type: application/json\r\n\
                         Content-Length: 42\r\n\
                         \r\n\
                         {\"id\":\"chatcmpl-123\",\"model\":\"gpt-4\"}";

        let parsed = parse_response(response).unwrap();
        assert_eq!(parsed.status_code, 200);
        assert_eq!(parsed.status_text, "OK");
        assert_eq!(parsed.content_type, Some("application/json".to_string()));
        assert!(!parsed.is_streaming);
        assert!(!parsed.is_chunked);
        assert!(parsed.body.is_some());
    }

    #[test]
    fn test_parse_streaming_response() {
        let response = b"HTTP/1.1 200 OK\r\n\
                         Content-Type: text/event-stream\r\n\
                         Transfer-Encoding: chunked\r\n\
                         \r\n\
                         data: {\"chunk\": 1}\n\n";

        let parsed = parse_response(response).unwrap();
        assert_eq!(parsed.status_code, 200);
        assert!(parsed.is_streaming);
        assert!(parsed.is_chunked);
    }

    #[test]
    fn test_decode_chunked_body() {
        // Simple chunked encoding: "Hello" (5 bytes) then "World" (5 bytes)
        let chunked = b"5\r\nHello\r\n5\r\nWorld\r\n0\r\n\r\n";
        let decoded = decode_chunked_body(chunked).unwrap();
        assert_eq!(decoded, b"HelloWorld");
    }

    #[test]
    fn test_decode_chunked_body_with_extension() {
        // Chunk with extension (should be ignored)
        let chunked = b"5;name=value\r\nHello\r\n0\r\n\r\n";
        let decoded = decode_chunked_body(chunked).unwrap();
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn test_extract_partial_body() {
        let data = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello, World!";
        let body = extract_partial_body(data).unwrap();
        assert_eq!(body, b"Hello, World!");
    }

    #[test]
    fn test_parse_request_no_body() {
        let request = b"GET /health HTTP/1.1\r\n\
                        Host: example.com\r\n\
                        \r\n";

        let parsed = parse_request(request).unwrap();
        assert_eq!(parsed.method, "GET");
        assert_eq!(parsed.path, "/health");
        assert!(parsed.body.is_none());
    }

    #[test]
    fn test_parse_response_with_headers() {
        let response = b"HTTP/1.1 200 OK\r\n\
                         X-Request-ID: abc123\r\n\
                         X-RateLimit-Remaining: 99\r\n\
                         Content-Type: application/json\r\n\
                         \r\n\
                         {}";

        let parsed = parse_response(response).unwrap();
        assert_eq!(
            parsed.headers.get("x-request-id"),
            Some(&"abc123".to_string())
        );
        assert_eq!(
            parsed.headers.get("x-ratelimit-remaining"),
            Some(&"99".to_string())
        );
    }
}
