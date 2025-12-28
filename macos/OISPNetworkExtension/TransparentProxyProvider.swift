// TransparentProxyProvider.swift
// OISPNetworkExtension
//
// NETransparentProxyProvider implementation for intercepting AI API traffic

import Foundation
import NetworkExtension
import Network
import os.log
import OISPCore

/// Main proxy provider for OISP network extension
class OISPTransparentProxyProvider: NETransparentProxyProvider {
    // MARK: - Components

    private let filter = AIEndpointFilter.shared
    private var tlsInterceptor: TLSInterceptor?
    private var eventEmitter: (any EventEmitter)?
    private let connectionManager = ConnectionManager()

    // MARK: - Logging

    private let logger = Logger(subsystem: "com.oisp.networkextension", category: "proxy")

    // MARK: - Configuration

    private var isCapturing = true

    // MARK: - Lifecycle

    override func startProxy(
        options: [String: Any]? = nil,
        completionHandler: @escaping (Error?) -> Void
    ) {
        logger.info("Starting OISP proxy...")

        Task {
            do {
                // Initialize Certificate Authority
                try await CertificateAuthority.shared.initialize()
                logger.info("Certificate Authority initialized")

                // Create TLS interceptor
                tlsInterceptor = TLSInterceptor(certificateAuthority: CertificateAuthority.shared)
                logger.info("TLS Interceptor created")

                // Connect to oisp-sensor
                let emitter = UnixSocketBridge()
                try await emitter.connect()
                eventEmitter = emitter
                logger.info("Connected to oisp-sensor")

                completionHandler(nil)
            } catch {
                logger.error("Failed to start proxy: \(error.localizedDescription)")
                completionHandler(error)
            }
        }
    }

    override func stopProxy(
        with reason: NEProviderStopReason,
        completionHandler: @escaping () -> Void
    ) {
        logger.info("Stopping OISP proxy (reason: \(String(describing: reason)))")

        Task {
            // Close all active connections
            await connectionManager.closeAll()

            // Disconnect from sensor
            await eventEmitter?.disconnect()

            completionHandler()
        }
    }

    // MARK: - Flow Handling

    override func handleNewFlow(_ flow: NEAppProxyFlow) -> Bool {
        guard let tcpFlow = flow as? NEAppProxyTCPFlow else {
            logger.debug("Ignoring non-TCP flow")
            return false
        }

        // Get destination endpoint
        guard let endpoint = tcpFlow.remoteEndpoint as? NWHostEndpoint else {
            logger.debug("Cannot get remote endpoint")
            return false
        }

        let host = endpoint.hostname
        let port = UInt16(endpoint.port) ?? 443

        // Check if we should intercept this connection
        guard filter.shouldIntercept(host: host, port: port) else {
            logger.debug("Not intercepting: \(host):\(port)")
            return false // Let it pass through unmodified
        }

        logger.info("Intercepting connection to \(host):\(port)")

        // Get process information
        let processInfo = getProcessInfo(from: tcpFlow)

        // Handle the flow asynchronously
        Task {
            await handleInterceptedFlow(
                flow: tcpFlow,
                host: host,
                port: port,
                processInfo: processInfo
            )
        }

        return true // We're handling this flow
    }

    // MARK: - Process Attribution

    private func getProcessInfo(from flow: NEAppProxyFlow) -> OISPProcessInfo? {
        // Get audit token from flow metadata
        guard let auditTokenData = flow.metaData.sourceAppAuditToken else {
            logger.warning("No audit token available for flow")
            return nil
        }

        return ProcessAttribution.shared.getProcessInfo(auditToken: auditTokenData)
    }

    // MARK: - Connection Interception

    private func handleInterceptedFlow(
        flow: NEAppProxyTCPFlow,
        host: String,
        port: UInt16,
        processInfo: OISPProcessInfo?
    ) async {
        guard let tlsInterceptor = tlsInterceptor else {
            logger.error("TLS Interceptor not initialized")
            closeFlow(flow)
            return
        }

        do {
            // Open the flow for reading/writing
            try await openFlow(flow)

            // Track this connection
            let connectionId = await connectionManager.track(
                flow: flow,
                host: host,
                port: port,
                processInfo: processInfo
            )

            logger.info("Connection \(connectionId): Starting MITM to \(host)")

            // Perform TLS MITM
            try await tlsInterceptor.intercept(
                clientFlow: flow,
                serverHost: host,
                serverPort: port,
                processInfo: processInfo,
                eventEmitter: eventEmitter
            )

            logger.info("Connection \(connectionId): MITM complete")

            // Cleanup
            await connectionManager.remove(connectionId)

        } catch {
            logger.error("Failed to intercept \(host): \(error.localizedDescription)")
            closeFlow(flow)
        }
    }

    private func openFlow(_ flow: NEAppProxyTCPFlow) async throws {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            flow.open(withLocalEndpoint: nil) { error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            }
        }
    }

    private func closeFlow(_ flow: NEAppProxyTCPFlow) {
        flow.closeReadWithError(nil)
        flow.closeWriteWithError(nil)
    }

    // MARK: - Control

    /// Pause capturing (connections still proxied but not recorded)
    func pause() {
        isCapturing = false
        logger.info("Capture paused")
    }

    /// Resume capturing
    func resume() {
        isCapturing = true
        logger.info("Capture resumed")
    }
}

// MARK: - Connection Manager

/// Manages active connections for the proxy
actor ConnectionManager {
    private var connections: [UUID: ConnectionInfo] = [:]

    struct ConnectionInfo {
        let id: UUID
        let flow: NEAppProxyTCPFlow
        let host: String
        let port: UInt16
        let processInfo: OISPProcessInfo?
        let startedAt: Date
    }

    func track(
        flow: NEAppProxyTCPFlow,
        host: String,
        port: UInt16,
        processInfo: OISPProcessInfo?
    ) -> UUID {
        let id = UUID()
        connections[id] = ConnectionInfo(
            id: id,
            flow: flow,
            host: host,
            port: port,
            processInfo: processInfo,
            startedAt: Date()
        )
        return id
    }

    func remove(_ id: UUID) {
        connections.removeValue(forKey: id)
    }

    func closeAll() {
        for (_, info) in connections {
            info.flow.closeReadWithError(nil)
            info.flow.closeWriteWithError(nil)
        }
        connections.removeAll()
    }

    var activeCount: Int {
        connections.count
    }
}
