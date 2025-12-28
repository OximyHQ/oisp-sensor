// AIEndpointFilter.swift
// OISPNetworkExtension
//
// Filters network traffic to only intercept AI API endpoints

import Foundation

/// Filter for AI/LLM API endpoints that should be intercepted
public struct AIEndpointFilter: Sendable {
    /// Static list of known AI API endpoints
    public static let defaultEndpoints: [EndpointPattern] = [
        // OpenAI
        .exact("api.openai.com"),

        // Anthropic
        .exact("api.anthropic.com"),

        // Google AI
        .exact("generativelanguage.googleapis.com"),
        .exact("aiplatform.googleapis.com"),

        // Azure OpenAI
        .suffix(".openai.azure.com"),

        // AWS Bedrock
        .regex(#"bedrock-runtime\..*\.amazonaws\.com"#),

        // Cohere
        .exact("api.cohere.ai"),
        .exact("api.cohere.com"),

        // Mistral AI
        .exact("api.mistral.ai"),

        // Groq
        .exact("api.groq.com"),

        // Together AI
        .exact("api.together.xyz"),
        .exact("api.together.ai"),

        // Fireworks AI
        .exact("api.fireworks.ai"),

        // Perplexity
        .exact("api.perplexity.ai"),

        // OpenRouter
        .exact("openrouter.ai"),
        .exact("api.openrouter.ai"),

        // Replicate
        .exact("api.replicate.com"),

        // Hugging Face
        .exact("api-inference.huggingface.co"),

        // DeepSeek
        .exact("api.deepseek.com"),

        // xAI (Grok)
        .exact("api.x.ai"),

        // Ollama (local)
        .exact("localhost"),
        .exact("127.0.0.1"),
    ]

    /// Endpoint matching pattern
    public enum EndpointPattern: Sendable {
        /// Exact hostname match
        case exact(String)

        /// Suffix match (e.g., ".openai.azure.com")
        case suffix(String)

        /// Regex pattern match
        case regex(String)

        /// Check if a hostname matches this pattern
        public func matches(_ hostname: String) -> Bool {
            switch self {
            case .exact(let pattern):
                return hostname.lowercased() == pattern.lowercased()

            case .suffix(let suffix):
                return hostname.lowercased().hasSuffix(suffix.lowercased())

            case .regex(let pattern):
                do {
                    let regex = try NSRegularExpression(pattern: pattern, options: .caseInsensitive)
                    let range = NSRange(hostname.startIndex..., in: hostname)
                    return regex.firstMatch(in: hostname, options: [], range: range) != nil
                } catch {
                    return false
                }
            }
        }
    }

    /// Current endpoint patterns (static + user-configured)
    private let patterns: [EndpointPattern]

    /// User-configured custom endpoints
    private let customEndpoints: [String]

    /// Ports to intercept (default: 443 for HTTPS)
    private let interceptPorts: Set<UInt16>

    public init(
        additionalEndpoints: [String] = [],
        additionalPatterns: [EndpointPattern] = [],
        interceptPorts: Set<UInt16> = [443, 8443]
    ) {
        self.customEndpoints = additionalEndpoints
        self.interceptPorts = interceptPorts

        // Combine default + custom patterns
        var allPatterns = Self.defaultEndpoints
        allPatterns.append(contentsOf: additionalPatterns)

        // Add custom endpoints as exact matches
        for endpoint in additionalEndpoints {
            allPatterns.append(.exact(endpoint))
        }

        self.patterns = allPatterns
    }

    /// Check if a connection should be intercepted
    /// - Parameters:
    ///   - host: The destination hostname
    ///   - port: The destination port
    /// - Returns: true if this connection should be MITM'd
    public func shouldIntercept(host: String, port: UInt16 = 443) -> Bool {
        // Only intercept HTTPS ports
        guard interceptPorts.contains(port) else {
            return false
        }

        // Check against all patterns
        for pattern in patterns {
            if pattern.matches(host) {
                return true
            }
        }

        return false
    }

    /// Check if a hostname is a known AI endpoint (regardless of port)
    public func isAIEndpoint(_ host: String) -> Bool {
        for pattern in patterns {
            if pattern.matches(host) {
                return true
            }
        }
        return false
    }
}

// MARK: - Convenience

extension AIEndpointFilter {
    /// Shared filter with default configuration
    public static let shared = AIEndpointFilter()

    /// Get list of all endpoint patterns (for UI display)
    public var allPatterns: [String] {
        var result: [String] = []
        for pattern in patterns {
            switch pattern {
            case .exact(let host):
                result.append(host)
            case .suffix(let suffix):
                result.append("*\(suffix)")
            case .regex(let regex):
                result.append("/\(regex)/")
            }
        }
        return result
    }
}

// MARK: - Provider Detection

extension AIEndpointFilter {
    /// Detected AI provider
    public enum Provider: String, Sendable {
        case openai
        case anthropic
        case google
        case azure
        case aws
        case cohere
        case mistral
        case groq
        case together
        case fireworks
        case perplexity
        case openrouter
        case replicate
        case huggingface
        case deepseek
        case xai
        case local
        case unknown
    }

    /// Detect the AI provider from hostname
    public func detectProvider(host: String) -> Provider {
        let lowercased = host.lowercased()

        // Azure OpenAI must be checked first (ends with .openai.azure.com)
        if lowercased.contains(".openai.azure.com") { return .azure }
        if lowercased.contains("openai.com") { return .openai }
        if lowercased.contains("anthropic.com") { return .anthropic }
        if lowercased.contains("googleapis.com") { return .google }
        if lowercased.contains("amazonaws.com") { return .aws }
        if lowercased.contains("cohere") { return .cohere }
        if lowercased.contains("mistral") { return .mistral }
        if lowercased.contains("groq.com") { return .groq }
        if lowercased.contains("together") { return .together }
        if lowercased.contains("fireworks") { return .fireworks }
        if lowercased.contains("perplexity") { return .perplexity }
        if lowercased.contains("openrouter") { return .openrouter }
        if lowercased.contains("replicate") { return .replicate }
        if lowercased.contains("huggingface") { return .huggingface }
        if lowercased.contains("deepseek") { return .deepseek }
        if lowercased.contains("x.ai") { return .xai }
        if lowercased == "localhost" || lowercased == "127.0.0.1" { return .local }

        return .unknown
    }
}
