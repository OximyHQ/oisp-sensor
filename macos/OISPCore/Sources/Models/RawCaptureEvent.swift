// RawCaptureEvent.swift
// OISPCore
//
// Event model matching oisp-core::plugins::RawCaptureEvent in Rust

import Foundation

/// The kind of SSL operation captured
public enum RawEventKind: String, Codable, Sendable {
    case sslRead = "SslRead"
    case sslWrite = "SslWrite"
}

/// Metadata about the process that made the request
public struct RawEventMetadata: Codable, Sendable {
    /// Process name (comm)
    public let comm: String

    /// Full executable path
    public let exe: String

    /// User ID
    public let uid: UInt32

    /// File descriptor (if available)
    public let fd: Int32?

    /// Parent process ID
    public let ppid: UInt32?

    public init(comm: String, exe: String, uid: UInt32, fd: Int32? = nil, ppid: UInt32? = nil) {
        self.comm = comm
        self.exe = exe
        self.uid = uid
        self.fd = fd
        self.ppid = ppid
    }
}

/// Raw capture event sent from Swift to Rust sensor
/// This matches the RawCaptureEvent struct in oisp-core
public struct RawCaptureEvent: Codable, Sendable {
    /// Unique event ID (ULID format)
    public let id: String

    /// Timestamp in nanoseconds since Unix epoch
    public let timestampNs: UInt64

    /// Type of event (SslRead or SslWrite)
    public let kind: RawEventKind

    /// Process ID that made the SSL call
    public let pid: UInt32

    /// Thread ID (if available)
    public let tid: UInt32?

    /// Captured data (base64 encoded plaintext HTTP)
    public let data: String

    /// Process metadata
    public let metadata: RawEventMetadata

    /// Remote host (for correlation)
    public let remoteHost: String?

    /// Remote port
    public let remotePort: UInt16?

    enum CodingKeys: String, CodingKey {
        case id
        case timestampNs = "timestamp_ns"
        case kind
        case pid
        case tid
        case data
        case metadata
        case remoteHost = "remote_host"
        case remotePort = "remote_port"
    }

    public init(
        kind: RawEventKind,
        pid: UInt32,
        tid: UInt32? = nil,
        data: Data,
        metadata: RawEventMetadata,
        remoteHost: String? = nil,
        remotePort: UInt16? = nil
    ) {
        self.id = Self.generateULID()
        self.timestampNs = Self.currentTimestampNs()
        self.kind = kind
        self.pid = pid
        self.tid = tid
        self.data = data.base64EncodedString()
        self.metadata = metadata
        self.remoteHost = remoteHost
        self.remotePort = remotePort
    }

    /// Generate a ULID-like identifier
    private static func generateULID() -> String {
        // Simple UUID-based ID for now
        // TODO: Use proper ULID library for time-sorted IDs
        UUID().uuidString.replacingOccurrences(of: "-", with: "").lowercased()
    }

    /// Get current timestamp in nanoseconds
    private static func currentTimestampNs() -> UInt64 {
        let now = Date()
        return UInt64(now.timeIntervalSince1970 * 1_000_000_000)
    }

    /// Serialize to JSON for sending over the bridge
    public func toJSON() throws -> Data {
        let encoder = JSONEncoder()
        return try encoder.encode(self)
    }

    /// Serialize to JSON string
    public func toJSONString() throws -> String {
        let data = try toJSON()
        return String(data: data, encoding: .utf8) ?? ""
    }
}

// MARK: - Convenience Extensions

extension RawCaptureEvent: CustomStringConvertible {
    public var description: String {
        "\(kind.rawValue) pid=\(pid) host=\(remoteHost ?? "unknown") bytes=\(data.count)"
    }
}
