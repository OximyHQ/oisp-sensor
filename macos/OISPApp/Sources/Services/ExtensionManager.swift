// ExtensionManager.swift
// OISPApp
//
// Manages the Network Extension lifecycle

import Foundation
import SystemExtensions
import NetworkExtension
import os.log

/// Manages the OISP Network Extension
public actor ExtensionManager {
    // MARK: - Singleton

    public static let shared = ExtensionManager()

    // MARK: - Properties

    private let logger = Logger(subsystem: "com.oisp.app", category: "extension-manager")

    private let extensionIdentifier = "com.oisp.networkextension"

    private var activationDelegate: ExtensionActivationDelegate?
    private var providerManager: NETunnelProviderManager?

    // MARK: - State

    public var isEnabled: Bool {
        get async {
            await refreshProviderManager()
            return providerManager?.isEnabled ?? false
        }
    }

    public var isConnected: Bool {
        get async {
            await refreshProviderManager()
            return providerManager?.connection.status == .connected
        }
    }

    // MARK: - Initialization

    private init() {}

    // MARK: - Activation

    /// Request activation of the system extension
    public func requestActivation() async throws {
        logger.info("Requesting system extension activation...")

        // Create activation request
        let request = OSSystemExtensionRequest.activationRequest(
            forExtensionWithIdentifier: extensionIdentifier,
            queue: .main
        )

        // Set up delegate
        let delegate = ExtensionActivationDelegate()
        activationDelegate = delegate
        request.delegate = delegate

        // Submit request
        OSSystemExtensionManager.shared.submitRequest(request)

        // Wait for result
        let result = try await delegate.waitForResult()

        if result {
            logger.info("System extension activation succeeded")
            try await configureProvider()
        } else {
            logger.error("System extension activation failed")
            throw ExtensionError.activationFailed
        }
    }

    /// Configure the network extension provider
    private func configureProvider() async throws {
        logger.info("Configuring network extension provider...")

        // Load existing managers
        let managers = try await NETunnelProviderManager.loadAllFromPreferences()

        // Find or create our manager
        var manager = managers.first { manager in
            (manager.protocolConfiguration as? NETunnelProviderProtocol)?.providerBundleIdentifier == extensionIdentifier
        }

        if manager == nil {
            manager = NETunnelProviderManager()
        }

        guard let manager = manager else {
            throw ExtensionError.configurationFailed
        }

        // Configure
        let proto = NETunnelProviderProtocol()
        proto.providerBundleIdentifier = extensionIdentifier
        proto.serverAddress = "OISP Local"

        manager.protocolConfiguration = proto
        manager.localizedDescription = "OISP AI Traffic Monitor"
        manager.isEnabled = true

        // Save
        try await manager.saveToPreferences()
        try await manager.loadFromPreferences()

        providerManager = manager

        logger.info("Network extension provider configured")
    }

    // MARK: - Control

    /// Start the extension
    public func start() async throws {
        await refreshProviderManager()

        guard let manager = providerManager else {
            throw ExtensionError.notConfigured
        }

        try manager.connection.startVPNTunnel()
        logger.info("Extension started")
    }

    /// Stop the extension
    public func stop() async {
        providerManager?.connection.stopVPNTunnel()
        logger.info("Extension stopped")
    }

    /// Pause capturing (extension stays running)
    public func pause() async {
        // Send message to extension to pause
        try? await sendMessage(["action": "pause"])
        logger.info("Capture paused")
    }

    /// Resume capturing
    public func resume() async {
        try? await sendMessage(["action": "resume"])
        logger.info("Capture resumed")
    }

    // MARK: - Communication

    /// Send message to the extension
    private func sendMessage(_ message: [String: Any]) async throws {
        guard let manager = providerManager else {
            throw ExtensionError.notConfigured
        }

        guard let session = manager.connection as? NETunnelProviderSession else {
            throw ExtensionError.communicationFailed
        }

        let data = try JSONSerialization.data(withJSONObject: message)
        try session.sendProviderMessage(data) { _ in }
    }

    // MARK: - Private

    private func refreshProviderManager() async {
        do {
            let managers = try await NETunnelProviderManager.loadAllFromPreferences()
            providerManager = managers.first { manager in
                (manager.protocolConfiguration as? NETunnelProviderProtocol)?.providerBundleIdentifier == extensionIdentifier
            }
        } catch {
            logger.error("Failed to load provider manager: \(error.localizedDescription)")
        }
    }
}

// MARK: - Activation Delegate

private class ExtensionActivationDelegate: NSObject, OSSystemExtensionRequestDelegate {
    private var continuation: CheckedContinuation<Bool, Error>?
    private var result: Result<Bool, Error>?

    func waitForResult() async throws -> Bool {
        if let result = result {
            return try result.get()
        }

        return try await withCheckedThrowingContinuation { continuation in
            self.continuation = continuation
        }
    }

    func request(_ request: OSSystemExtensionRequest, didFinishWithResult result: OSSystemExtensionRequest.Result) {
        let success = result == .completed || result == .willCompleteAfterReboot
        self.result = .success(success)
        continuation?.resume(returning: success)
    }

    func request(_ request: OSSystemExtensionRequest, didFailWithError error: Error) {
        self.result = .failure(error)
        continuation?.resume(throwing: error)
    }

    func requestNeedsUserApproval(_ request: OSSystemExtensionRequest) {
        // User needs to approve in System Settings
        // Open System Settings automatically
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy") {
            NSWorkspace.shared.open(url)
        }
    }

    func request(_ request: OSSystemExtensionRequest, actionForReplacingExtension existing: OSSystemExtensionProperties, withExtension ext: OSSystemExtensionProperties) -> OSSystemExtensionRequest.ReplacementAction {
        // Replace existing extension with new version
        return .replace
    }
}

// MARK: - Errors

public enum ExtensionError: Error, LocalizedError {
    case activationFailed
    case configurationFailed
    case notConfigured
    case communicationFailed

    public var errorDescription: String? {
        switch self {
        case .activationFailed:
            return "Failed to activate the network extension"
        case .configurationFailed:
            return "Failed to configure the network extension"
        case .notConfigured:
            return "Network extension is not configured"
        case .communicationFailed:
            return "Failed to communicate with the extension"
        }
    }
}
