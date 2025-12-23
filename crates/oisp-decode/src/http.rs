//! HTTP parsing utilities

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
            
            let body = if header_len < data.len() {
                Some(data[header_len..].to_vec())
            } else {
                None
            };
            
            Some(ParsedHttpRequest {
                method: req.method?.to_string(),
                path: req.path?.to_string(),
                version: format!("HTTP/1.{}", req.version?),
                host: header_map.get("host").cloned(),
                content_type: header_map.get("content-type").cloned(),
                content_length: header_map.get("content-length")
                    .and_then(|v| v.parse().ok()),
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
            
            let body = if header_len < data.len() {
                Some(data[header_len..].to_vec())
            } else {
                None
            };
            
            let content_type = header_map.get("content-type").cloned();
            let is_streaming = content_type.as_ref()
                .map(|ct| ct.contains("text/event-stream") || ct.contains("application/x-ndjson"))
                .unwrap_or(false);
            
            Some(ParsedHttpResponse {
                status_code: resp.code?,
                status_text: resp.reason?.to_string(),
                version: format!("HTTP/1.{}", resp.version?),
                content_type,
                content_length: header_map.get("content-length")
                    .and_then(|v| v.parse().ok()),
                headers: header_map,
                body,
                is_streaming,
            })
        }
        Ok(httparse::Status::Partial) => None,
        Err(_) => None,
    }
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
    ];
    
    prefixes.iter().any(|prefix| data.starts_with(prefix))
}

/// Check if data looks like an HTTP response
pub fn is_http_response(data: &[u8]) -> bool {
    data.starts_with(b"HTTP/")
}

