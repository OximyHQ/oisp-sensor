// TLSInterceptor.swift
// OISPNetworkExtension
//
// TLS Man-in-the-Middle implementation for decrypting AI API traffic

import Foundation
import Network
import NetworkExtension
import Security
import os.log
import OISPCore

// MARK: - Flow-based TLS Session

/// TLS session that wraps NEAppProxyTCPFlow for client-side TLS
/// This handles TLS encryption/decryption directly on the flow
public actor FlowTLSSession {
    private let flow: NEAppProxyTCPFlow
    private var isClosed = false
    private let logger = Logger(subsystem: "com.oisp.networkextension", category: "flow-tls-session")

    init(flow: NEAppProxyTCPFlow) {
        self.flow = flow
    }

    /// Read plaintext data from the flow
    /// Note: When TLS is established, this reads decrypted data
    func read() async throws -> Data {
        guard !isClosed else {
            throw TLSError.connectionClosed
        }

        return try await withCheckedThrowingContinuation { continuation in
            flow.readData { data, error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else if let data = data, !data.isEmpty {
                    continuation.resume(returning: data)
                } else {
                    continuation.resume(throwing: TLSError.connectionClosed)
                }
            }
        }
    }

    /// Write plaintext data to the flow
    func write(_ data: Data) async throws {
        guard !isClosed else {
            throw TLSError.connectionClosed
        }

        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            flow.write(data) { error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            }
        }
    }

    /// Close the flow
    func close() {
        guard !isClosed else { return }
        isClosed = true
        flow.closeReadWithError(nil)
        flow.closeWriteWithError(nil)
    }
}

// MARK: - NWConnection TLS Session

/// TLS session wrapper for NWConnection (server-side connection)
public actor NWConnectionTLSSession {
    private let connection: NWConnection
    private var isClosed = false
    private let logger = Logger(subsystem: "com.oisp.networkextension", category: "nw-tls-session")

    init(connection: NWConnection) {
        self.connection = connection
    }

    /// Read data from the TLS connection
    func read() async throws -> Data {
        guard !isClosed else {
            throw TLSError.connectionClosed
        }

        return try await withCheckedThrowingContinuation { continuation in
            connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { data, _, isComplete, error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else if let data = data, !data.isEmpty {
                    continuation.resume(returning: data)
                } else if isComplete {
                    continuation.resume(throwing: TLSError.connectionClosed)
                } else {
                    continuation.resume(throwing: TLSError.readFailed("No data received"))
                }
            }
        }
    }

    /// Write data to the TLS connection
    func write(_ data: Data) async throws {
        guard !isClosed else {
            throw TLSError.connectionClosed
        }

        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            connection.send(content: data, completion: .contentProcessed { error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            })
        }
    }

    /// Close the connection
    func close() {
        guard !isClosed else { return }
        isClosed = true
        connection.cancel()
    }

    var state: NWConnection.State {
        connection.state
    }
}

// MARK: - TLS Interceptor

/// TLS Man-in-the-Middle interceptor
///
/// This implementation uses a simplified approach:
/// 1. For client-side: Read raw TCP data from the flow, we handle TLS ourselves
/// 2. For server-side: Connect to the real server using NWConnection with TLS
/// 3. Bridge the two, capturing plaintext HTTP as it passes through
///
/// Note: The complex part is that NEAppProxyTCPFlow gives us raw TCP,
/// so we need to handle the TLS protocol ourselves for the client side.
/// For simplicity, we use a "tunnel mode" where we just relay encrypted
/// data between client and server, but sniff the TLS handshake to get
/// the server name and then connect.
public actor TLSInterceptor {
    private let certificateAuthority: CertificateAuthority
    private let logger = Logger(subsystem: "com.oisp.networkextension", category: "tls-interceptor")

    public init(certificateAuthority: CertificateAuthority) {
        self.certificateAuthority = certificateAuthority
    }

    /// Perform TLS MITM on a connection
    ///
    /// Implementation approach:
    /// 1. Create a local NWListener that will accept our internal connection
    /// 2. Bridge the flow to this local listener
    /// 3. Accept the connection on the listener and do TLS termination
    /// 4. Connect to the real server
    /// 5. Relay and capture plaintext
    public func intercept(
        clientFlow: NEAppProxyTCPFlow,
        serverHost: String,
        serverPort: UInt16,
        processInfo: OISPProcessInfo?,
        eventEmitter: (any EventEmitter)?
    ) async throws {
        logger.info("Starting MITM for \(serverHost):\(serverPort)")

        // For this implementation, we use a direct relay approach:
        // Since the flow is already open at the TCP level, and we can't easily
        // do TLS termination on a flow, we'll relay encrypted traffic
        // and decode it on the other side.

        // Actually, for proper MITM we need to:
        // 1. Generate a cert for serverHost
        // 2. Perform TLS handshake with the client using our cert
        // 3. Connect to the real server
        // 4. Relay decrypted traffic

        // Connect to the real server first
        logger.info("Connecting to server \(serverHost):\(serverPort)")
        let serverSession = try await connectToServer(host: serverHost, port: serverPort)

        // Create the client session wrapper
        let clientSession = FlowTLSSession(flow: clientFlow)

        // For now, implement a pass-through relay that captures raw data
        // In a full implementation, we'd do TLS termination here
        logger.info("Starting relay (pass-through mode)")
        await relayPassThrough(
            client: clientSession,
            server: serverSession,
            host: serverHost,
            port: serverPort,
            processInfo: processInfo,
            eventEmitter: eventEmitter
        )

        logger.info("MITM session ended for \(serverHost)")
    }

    /// Connect to the real server as a TLS client
    private func connectToServer(host: String, port: UInt16) async throws -> NWConnectionTLSSession {
        let endpoint = NWEndpoint.hostPort(
            host: NWEndpoint.Host(host),
            port: NWEndpoint.Port(rawValue: port)!
        )

        // Create TLS options using system trust store
        let tlsOptions = NWProtocolTLS.Options()

        // Set SNI
        sec_protocol_options_set_tls_server_name(
            tlsOptions.securityProtocolOptions,
            host
        )

        let parameters = NWParameters(tls: tlsOptions)
        let connection = NWConnection(to: endpoint, using: parameters)

        return try await withCheckedThrowingContinuation { continuation in
            var hasResumed = false

            connection.stateUpdateHandler = { [weak connection] state in
                guard !hasResumed else { return }

                switch state {
                case .ready:
                    hasResumed = true
                    continuation.resume(returning: NWConnectionTLSSession(connection: connection!))
                case .failed(let error):
                    hasResumed = true
                    continuation.resume(throwing: TLSError.connectionFailed(error.localizedDescription))
                case .cancelled:
                    hasResumed = true
                    continuation.resume(throwing: TLSError.connectionClosed)
                default:
                    break
                }
            }
            connection.start(queue: .global(qos: .userInteractive))
        }
    }

    // MARK: - Full MITM with TLS Termination

    /// Perform full MITM with TLS termination
    /// This creates a local socket pair to bridge the flow to NWConnection
    public func interceptWithTLSTermination(
        clientFlow: NEAppProxyTCPFlow,
        serverHost: String,
        serverPort: UInt16,
        processInfo: OISPProcessInfo?,
        eventEmitter: (any EventEmitter)?
    ) async throws {
        // Generate certificate for this hostname
        let identity = try await certificateAuthority.generateCertificate(for: serverHost)
        logger.info("Generated certificate for \(serverHost)")

        // Create a local listener on a random port
        let listener = try createLocalListener(withIdentity: identity)

        defer {
            listener.cancel()
        }

        // Get the port the listener is on
        guard let port = listener.port else {
            throw TLSError.connectionFailed("Could not determine listener port")
        }

        logger.info("Local TLS listener on port \(port.rawValue)")

        // Start accepting on the listener
        let clientConnectionTask = Task<NWConnection, Error> {
            try await withCheckedThrowingContinuation { continuation in
                var hasResumed = false
                listener.newConnectionHandler = { connection in
                    guard !hasResumed else { return }
                    hasResumed = true
                    continuation.resume(returning: connection)
                }
            }
        }

        // Connect flow to local listener
        let bridgeTask = Task {
            try await bridgeFlowToLocalPort(flow: clientFlow, port: port.rawValue)
        }

        // Wait for client connection
        let clientConnection: NWConnection
        do {
            clientConnection = try await clientConnectionTask.value
        } catch {
            bridgeTask.cancel()
            throw error
        }

        // Wait for client connection to be ready
        let clientSession = try await waitForConnection(clientConnection)

        // Connect to real server
        let serverSession = try await connectToServer(host: serverHost, port: serverPort)

        // Now relay with capture
        await relayWithCapture(
            client: clientSession,
            server: serverSession,
            host: serverHost,
            port: serverPort,
            processInfo: processInfo,
            eventEmitter: eventEmitter
        )
    }

    private func createLocalListener(withIdentity identity: SecIdentity) throws -> NWListener {
        let tlsOptions = NWProtocolTLS.Options()

        // Set our identity as the server certificate
        if let secIdentity = sec_identity_create(identity) {
            sec_protocol_options_set_local_identity(tlsOptions.securityProtocolOptions, secIdentity)
        }

        let parameters = NWParameters(tls: tlsOptions)
        parameters.allowLocalEndpointReuse = true

        // Listen on localhost, random port
        let listener = try NWListener(using: parameters, on: .any)
        listener.stateUpdateHandler = { state in
            // Log state changes
        }
        listener.start(queue: .global())

        return listener
    }

    private func bridgeFlowToLocalPort(flow: NEAppProxyTCPFlow, port: UInt16) async throws {
        // Create a TCP connection to our local listener
        let endpoint = NWEndpoint.hostPort(host: .ipv4(.loopback), port: NWEndpoint.Port(rawValue: port)!)
        let parameters = NWParameters.tcp
        let connection = NWConnection(to: endpoint, using: parameters)

        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            connection.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    continuation.resume()
                case .failed(let error):
                    continuation.resume(throwing: error)
                case .cancelled:
                    continuation.resume(throwing: TLSError.connectionClosed)
                default:
                    break
                }
            }
            connection.start(queue: .global())
        }

        // Bridge data between flow and connection
        async let flowToConnection: Void = {
            while true {
                do {
                    let data = try await withCheckedThrowingContinuation { (cont: CheckedContinuation<Data, Error>) in
                        flow.readData { data, error in
                            if let error = error {
                                cont.resume(throwing: error)
                            } else if let data = data {
                                cont.resume(returning: data)
                            } else {
                                cont.resume(throwing: TLSError.connectionClosed)
                            }
                        }
                    }

                    try await withCheckedThrowingContinuation { (cont: CheckedContinuation<Void, Error>) in
                        connection.send(content: data, completion: .contentProcessed { error in
                            if let error = error {
                                cont.resume(throwing: error)
                            } else {
                                cont.resume()
                            }
                        })
                    }
                } catch {
                    break
                }
            }
        }()

        async let connectionToFlow: Void = {
            while true {
                do {
                    let data = try await withCheckedThrowingContinuation { (cont: CheckedContinuation<Data, Error>) in
                        connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { data, _, isComplete, error in
                            if let error = error {
                                cont.resume(throwing: error)
                            } else if let data = data, !data.isEmpty {
                                cont.resume(returning: data)
                            } else if isComplete {
                                cont.resume(throwing: TLSError.connectionClosed)
                            } else {
                                cont.resume(throwing: TLSError.readFailed("No data"))
                            }
                        }
                    }

                    try await withCheckedThrowingContinuation { (cont: CheckedContinuation<Void, Error>) in
                        flow.write(data) { error in
                            if let error = error {
                                cont.resume(throwing: error)
                            } else {
                                cont.resume()
                            }
                        }
                    }
                } catch {
                    break
                }
            }
        }()

        _ = await (flowToConnection, connectionToFlow)
        connection.cancel()
    }

    private func waitForConnection(_ connection: NWConnection) async throws -> NWConnectionTLSSession {
        try await withCheckedThrowingContinuation { continuation in
            var hasResumed = false
            connection.stateUpdateHandler = { state in
                guard !hasResumed else { return }
                switch state {
                case .ready:
                    hasResumed = true
                    continuation.resume(returning: NWConnectionTLSSession(connection: connection))
                case .failed(let error):
                    hasResumed = true
                    continuation.resume(throwing: TLSError.connectionFailed(error.localizedDescription))
                case .cancelled:
                    hasResumed = true
                    continuation.resume(throwing: TLSError.connectionClosed)
                default:
                    break
                }
            }
            connection.start(queue: .global())
        }
    }

    // MARK: - Relay Functions

    /// Pass-through relay that captures raw (potentially encrypted) data
    private func relayPassThrough(
        client: FlowTLSSession,
        server: NWConnectionTLSSession,
        host: String,
        port: UInt16,
        processInfo: OISPProcessInfo?,
        eventEmitter: (any EventEmitter)?
    ) async {
        let metadata = processInfo?.toMetadata() ?? RawEventMetadata(
            comm: "unknown",
            exe: "unknown",
            uid: 0
        )
        let pid = UInt32(processInfo?.pid ?? 0)

        await withTaskGroup(of: Void.self) { group in
            // Client → Server
            group.addTask {
                while true {
                    do {
                        let data = try await client.read()

                        // Emit capture event
                        if let emitter = eventEmitter {
                            let event = RawCaptureEvent(
                                kind: .sslWrite,
                                pid: pid,
                                data: data,
                                metadata: metadata,
                                remoteHost: host,
                                remotePort: port
                            )
                            try? await emitter.emit(event)
                        }

                        try await server.write(data)
                    } catch {
                        break
                    }
                }
            }

            // Server → Client
            group.addTask {
                while true {
                    do {
                        let data = try await server.read()

                        // Emit capture event
                        if let emitter = eventEmitter {
                            let event = RawCaptureEvent(
                                kind: .sslRead,
                                pid: pid,
                                data: data,
                                metadata: metadata,
                                remoteHost: host,
                                remotePort: port
                            )
                            try? await emitter.emit(event)
                        }

                        try await client.write(data)
                    } catch {
                        break
                    }
                }
            }
        }

        await client.close()
        await server.close()
    }

    /// Relay with capture for decrypted (plaintext) data
    private func relayWithCapture(
        client: NWConnectionTLSSession,
        server: NWConnectionTLSSession,
        host: String,
        port: UInt16,
        processInfo: OISPProcessInfo?,
        eventEmitter: (any EventEmitter)?
    ) async {
        let metadata = processInfo?.toMetadata() ?? RawEventMetadata(
            comm: "unknown",
            exe: "unknown",
            uid: 0
        )
        let pid = UInt32(processInfo?.pid ?? 0)

        await withTaskGroup(of: Void.self) { group in
            // Client → Server (requests)
            group.addTask {
                while true {
                    do {
                        let data = try await client.read()

                        // Emit capture event for plaintext request
                        if let emitter = eventEmitter {
                            let event = RawCaptureEvent(
                                kind: .sslWrite,
                                pid: pid,
                                data: data,
                                metadata: metadata,
                                remoteHost: host,
                                remotePort: port
                            )
                            try? await emitter.emit(event)
                        }

                        try await server.write(data)
                    } catch {
                        self.logger.debug("Client→Server relay ended")
                        break
                    }
                }
            }

            // Server → Client (responses)
            group.addTask {
                while true {
                    do {
                        let data = try await server.read()

                        // Emit capture event for plaintext response
                        if let emitter = eventEmitter {
                            let event = RawCaptureEvent(
                                kind: .sslRead,
                                pid: pid,
                                data: data,
                                metadata: metadata,
                                remoteHost: host,
                                remotePort: port
                            )
                            try? await emitter.emit(event)
                        }

                        try await client.write(data)
                    } catch {
                        self.logger.debug("Server→Client relay ended")
                        break
                    }
                }
            }
        }

        await client.close()
        await server.close()
    }
}

// MARK: - Errors

public enum TLSError: Error, LocalizedError {
    case connectionFailed(String)
    case connectionClosed
    case handshakeFailed(String)
    case certificateError(String)
    case readFailed(String)
    case writeFailed(String)
    case notImplemented(String)
    case listenerFailed(String)

    public var errorDescription: String? {
        switch self {
        case .connectionFailed(let reason):
            return "TLS connection failed: \(reason)"
        case .connectionClosed:
            return "TLS connection closed"
        case .handshakeFailed(let reason):
            return "TLS handshake failed: \(reason)"
        case .certificateError(let reason):
            return "Certificate error: \(reason)"
        case .readFailed(let reason):
            return "TLS read failed: \(reason)"
        case .writeFailed(let reason):
            return "TLS write failed: \(reason)"
        case .notImplemented(let feature):
            return "Not implemented: \(feature)"
        case .listenerFailed(let reason):
            return "Listener failed: \(reason)"
        }
    }
}
