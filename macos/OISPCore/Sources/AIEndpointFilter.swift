// AIEndpointFilter.swift
// OISPCore
//
// Filters network traffic to only intercept AI API endpoints
// Uses runtime-loaded spec bundle (same as Linux) - no code generation needed.

import Foundation

/// Filter for AI/LLM API endpoints that should be intercepted
///
/// Uses OISPSpecBundle loaded at runtime for provider detection.
/// The bundle is automatically refreshed from the network every hour.
public struct AIEndpointFilter: Sendable {
    /// User-configured custom endpoints (in addition to spec bundle)
    private let customEndpoints: [String]

    /// Ports to intercept (default: 443 for HTTPS, plus local AI ports)
    private let interceptPorts: Set<UInt16>

    public init(
        additionalEndpoints: [String] = [],
        interceptPorts: Set<UInt16> = [443, 8443, 11434, 1234]
    ) {
        self.customEndpoints = additionalEndpoints
        self.interceptPorts = interceptPorts

        // Trigger bundle refresh check on first use
        SpecBundleLoader.shared.refreshIfNeeded()
    }

    /// Check if a connection should be intercepted
    /// - Parameters:
    ///   - host: The destination hostname
    ///   - port: The destination port
    /// - Returns: true if this connection should be MITM'd
    public func shouldIntercept(host: String, port: UInt16 = 443) -> Bool {
        // Only intercept allowed ports
        guard interceptPorts.contains(port) else {
            return false
        }

        // Check using runtime spec bundle
        let hostWithPort = "\(host):\(port)"
        if DynamicProviderRegistry.shared.isKnownEndpoint(hostWithPort) {
            return true
        }
        if DynamicProviderRegistry.shared.isKnownEndpoint(host) {
            return true
        }

        // Check custom endpoints
        let lowercased = host.lowercased()
        for endpoint in customEndpoints {
            if lowercased == endpoint.lowercased() {
                return true
            }
        }

        return false
    }

    /// Check if a hostname is a known AI endpoint (regardless of port)
    public func isAIEndpoint(_ host: String) -> Bool {
        if DynamicProviderRegistry.shared.isKnownEndpoint(host) {
            return true
        }

        // Check custom endpoints
        let lowercased = host.lowercased()
        for endpoint in customEndpoints {
            if lowercased == endpoint.lowercased() {
                return true
            }
        }

        return false
    }

    /// Detect the AI provider from hostname
    public func detectProvider(host: String, port: UInt16 = 443) -> AIProvider {
        // Try with port first (for local providers like Ollama)
        let hostWithPort = "\(host):\(port)"
        let providerWithPort = AIProvider.detect(from: hostWithPort)
        if providerWithPort != .unknown {
            return providerWithPort
        }

        // Try without port
        return AIProvider.detect(from: host)
    }
}

// MARK: - Convenience

extension AIEndpointFilter {
    /// Shared filter with default configuration
    public static let shared = AIEndpointFilter()

    /// Get list of all endpoint patterns (for UI display)
    public var allPatterns: [String] {
        var result = DynamicProviderRegistry.shared.allKnownDomains

        // Add custom endpoints
        result.append(contentsOf: customEndpoints)

        return result.sorted()
    }

    /// Get all known providers
    public var allProviders: [AIProvider] {
        return AIProvider.allCases.filter { $0 != .unknown }
    }
}

// MARK: - Backward Compatibility

extension AIEndpointFilter {
    /// Backward compatibility: Provider is now AIProvider (from OISPSpecBundle.swift)
    public typealias Provider = AIProvider
}
