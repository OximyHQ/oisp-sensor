//! Network events

use super::envelope::EventEnvelope;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Network connect event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConnectEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    #[serde(flatten)]
    pub data: NetworkConnectData,
}

/// Network connect data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConnectData {
    /// Destination endpoint
    pub dest: Endpoint,
    
    /// Source endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<Endpoint>,
    
    /// Protocol
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<Protocol>,
    
    /// Whether connection succeeded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    
    /// Error if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Connection latency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
    
    /// TLS information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsInfo>,
}

/// Network accept event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAcceptEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    #[serde(flatten)]
    pub data: NetworkAcceptData,
}

/// Network accept data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAcceptData {
    /// Source endpoint (connecting client)
    pub src: Endpoint,
    
    /// Destination endpoint (local)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest: Option<Endpoint>,
    
    /// Protocol
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<Protocol>,
}

/// Network flow summary event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFlowEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    #[serde(flatten)]
    pub data: NetworkFlowData,
}

/// Network flow data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFlowData {
    /// Destination endpoint
    pub dest: Endpoint,
    
    /// Source endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<Endpoint>,
    
    /// Protocol
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<Protocol>,
    
    /// Direction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<FlowDirection>,
    
    /// Bytes sent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_sent: Option<u64>,
    
    /// Bytes received
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_received: Option<u64>,
    
    /// Packets sent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packets_sent: Option<u64>,
    
    /// Packets received
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packets_received: Option<u64>,
    
    /// Flow duration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    
    /// Start time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<DateTime<Utc>>,
    
    /// End time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,
    
    /// TLS information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsInfo>,
    
    /// HTTP information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpInfo>,
}

/// DNS event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDnsEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,
    
    #[serde(flatten)]
    pub data: NetworkDnsData,
}

/// DNS data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDnsData {
    /// Query name
    pub query_name: String,
    
    /// Query type
    pub query_type: DnsQueryType,
    
    /// Response code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_code: Option<DnsResponseCode>,
    
    /// Answers
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<DnsAnswer>,
    
    /// Resolver used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolver: Option<String>,
    
    /// Latency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
}

/// Network endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    /// IP address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    
    /// Port
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    
    /// Domain name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    
    /// Whether private/internal IP
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_private: Option<bool>,
    
    /// Geolocation data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geo: Option<GeoInfo>,
}

/// Geolocation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asn: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,
}

/// Transport protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Tcp,
    Udp,
    Unix,
    Other,
}

/// Flow direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowDirection {
    Outbound,
    Inbound,
    Bidirectional,
}

/// TLS information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsInfo {
    /// TLS version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    
    /// Cipher suite
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cipher_suite: Option<String>,
    
    /// Server Name Indication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    
    /// ALPN result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<String>,
    
    /// Certificate information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<CertificateInfo>,
    
    /// JA3 fingerprint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ja3_fingerprint: Option<String>,
    
    /// JA3S fingerprint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ja3s_fingerprint: Option<String>,
}

/// Certificate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_before: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_after: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub san: Vec<String>,
}

/// HTTP information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpInfo {
    /// HTTP method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    
    /// Request path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    
    /// Status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    
    /// Host header
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    
    /// User-Agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    
    /// Content-Type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    
    /// Content-Length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_length: Option<u64>,
    
    /// Request headers
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub request_headers: HashMap<String, String>,
    
    /// Response headers
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub response_headers: HashMap<String, String>,
}

/// DNS query type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DnsQueryType {
    A,
    Aaaa,
    Cname,
    Mx,
    Txt,
    Ns,
    Ptr,
    Srv,
    Other,
}

/// DNS response code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DnsResponseCode {
    Noerror,
    Nxdomain,
    Servfail,
    Refused,
    Other,
}

/// DNS answer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsAnswer {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub answer_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u32>,
}

