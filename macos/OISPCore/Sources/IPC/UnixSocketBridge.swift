// UnixSocketBridge.swift
// OISPCore
//
// Unix domain socket client for sending events to oisp-sensor (Rust)

import Foundation
import Network

/// Protocol for emitting raw capture events
public protocol EventEmitter: Sendable {
    func emit(_ event: RawCaptureEvent) async throws
    func connect() async throws
    func disconnect() async
    var isConnected: Bool { get async }
}

/// Unix socket client for connecting to oisp-sensor
public actor UnixSocketBridge: EventEmitter {
    // MARK: - Configuration

    /// Default socket path
    public static let defaultSocketPath = "/tmp/oisp.sock"

    private let socketPath: String
    private let reconnectDelay: TimeInterval
    private let maxReconnectAttempts: Int

    // MARK: - State

    private var connection: NWConnection?
    private var connectionState: NWConnection.State = .cancelled
    private var reconnectAttempts = 0
    private var shouldReconnect = true
    private var pendingEvents: [RawCaptureEvent] = []

    /// Maximum number of pending events to buffer during disconnection
    private let maxPendingEvents = 10000

    // MARK: - Statistics

    private var eventsSent: UInt64 = 0
    private var eventsDropped: UInt64 = 0
    private var bytesTransferred: UInt64 = 0

    // MARK: - Initialization

    public init(
        socketPath: String = defaultSocketPath,
        reconnectDelay: TimeInterval = 1.0,
        maxReconnectAttempts: Int = 10
    ) {
        self.socketPath = socketPath
        self.reconnectDelay = reconnectDelay
        self.maxReconnectAttempts = maxReconnectAttempts
    }

    // MARK: - Connection Management

    public var isConnected: Bool {
        connectionState == .ready
    }

    public func connect() async throws {
        shouldReconnect = true
        reconnectAttempts = 0
        try await establishConnection()
    }

    private func establishConnection() async throws {
        // Create Unix socket endpoint
        let endpoint = NWEndpoint.unix(path: socketPath)

        // Create connection parameters
        let parameters = NWParameters()
        parameters.allowLocalEndpointReuse = true

        // Create connection
        let conn = NWConnection(to: endpoint, using: parameters)
        connection = conn

        // Set up state handler
        conn.stateUpdateHandler = { [weak self] state in
            Task {
                await self?.handleStateChange(state)
            }
        }

        // Start connection
        conn.start(queue: .global(qos: .userInteractive))

        // Wait for connection to be ready
        for _ in 0..<100 { // 10 second timeout
            if connectionState == .ready {
                return
            }
            if case .failed = connectionState {
                throw UnixSocketError.connectionFailed("Connection failed")
            }
            try await Task.sleep(nanoseconds: 100_000_000) // 100ms
        }

        throw UnixSocketError.connectionTimeout
    }

    private func handleStateChange(_ state: NWConnection.State) async {
        connectionState = state

        switch state {
        case .ready:
            reconnectAttempts = 0
            // Send any pending events
            await flushPendingEvents()

        case .failed(let error):
            connection = nil
            if shouldReconnect {
                await attemptReconnect(error: error)
            }

        case .cancelled:
            connection = nil

        default:
            break
        }
    }

    private func attemptReconnect(error: NWError) async {
        guard reconnectAttempts < maxReconnectAttempts else {
            // Max attempts reached, give up
            return
        }

        reconnectAttempts += 1
        let delay = reconnectDelay * Double(reconnectAttempts)

        try? await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000))

        guard shouldReconnect else { return }

        try? await establishConnection()
    }

    public func disconnect() async {
        shouldReconnect = false
        connection?.cancel()
        connection = nil
    }

    // MARK: - Event Emission

    public func emit(_ event: RawCaptureEvent) async throws {
        // If not connected, buffer the event
        guard let conn = connection, connectionState == .ready else {
            await bufferEvent(event)
            return
        }

        // Serialize to JSON
        let jsonData = try event.toJSON()

        // Add newline delimiter
        var data = jsonData
        data.append(contentsOf: [0x0A]) // newline

        // Send data
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            conn.send(content: data, completion: .contentProcessed { error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            })
        }

        eventsSent += 1
        bytesTransferred += UInt64(data.count)
    }

    private func bufferEvent(_ event: RawCaptureEvent) async {
        if pendingEvents.count >= maxPendingEvents {
            // Drop oldest events
            pendingEvents.removeFirst(pendingEvents.count - maxPendingEvents + 1)
            eventsDropped += 1
        }
        pendingEvents.append(event)
    }

    private func flushPendingEvents() async {
        let events = pendingEvents
        pendingEvents.removeAll()

        for event in events {
            try? await emit(event)
        }
    }

    // MARK: - Statistics

    public struct Statistics: Sendable {
        public let eventsSent: UInt64
        public let eventsDropped: UInt64
        public let bytesTransferred: UInt64
        public let pendingEvents: Int
        public let isConnected: Bool
    }

    public func statistics() async -> Statistics {
        Statistics(
            eventsSent: eventsSent,
            eventsDropped: eventsDropped,
            bytesTransferred: bytesTransferred,
            pendingEvents: pendingEvents.count,
            isConnected: isConnected
        )
    }
}

// MARK: - Errors

public enum UnixSocketError: Error, LocalizedError {
    case connectionFailed(String)
    case connectionTimeout
    case sendFailed(String)
    case notConnected

    public var errorDescription: String? {
        switch self {
        case .connectionFailed(let reason):
            return "Failed to connect to Unix socket: \(reason)"
        case .connectionTimeout:
            return "Connection to Unix socket timed out"
        case .sendFailed(let reason):
            return "Failed to send data: \(reason)"
        case .notConnected:
            return "Not connected to Unix socket"
        }
    }
}

// MARK: - Mock Emitter for Testing

/// Mock event emitter that stores events in memory
public actor MockEventEmitter: EventEmitter {
    private var events: [RawCaptureEvent] = []
    private var connected = false

    public init() {}

    public var isConnected: Bool { connected }

    public func connect() async throws {
        connected = true
    }

    public func disconnect() async {
        connected = false
    }

    public func emit(_ event: RawCaptureEvent) async throws {
        events.append(event)
    }

    public func allEvents() -> [RawCaptureEvent] {
        events
    }

    public func clear() {
        events.removeAll()
    }
}
