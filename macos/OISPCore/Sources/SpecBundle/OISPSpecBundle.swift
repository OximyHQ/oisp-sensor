// OISPSpecBundle.swift
// OISPCore
//
// Dynamic runtime loader for oisp-spec-bundle.json
// Mirrors the Rust SpecLoader behavior - no code generation needed.
//
// Loading strategy:
// 1. Try cached bundle from disk (~/.cache/oisp/spec-bundle.json)
// 2. Fetch latest from remote (async, if stale)
// 3. Fall back to embedded bundle (shipped with app)

import Foundation

/// Default URL for fetching the spec bundle
public let defaultBundleURL = "https://oisp.dev/spec/v0.1/bundle.json"

/// How often to check for bundle updates (1 hour)
public let bundleRefreshInterval: TimeInterval = 3600

// MARK: - Spec Bundle Model

/// The complete OISP spec bundle - loaded from JSON at runtime
public struct OISPSpecBundle: Codable, Sendable {
    public let version: String
    public let bundleVersion: String
    public let generatedAt: String
    public let source: String
    public let domainIndex: [String: String]
    public let domainPatterns: [DomainPattern]
    public let providers: [String: ProviderSpec]
    public let extractionRules: [String: ExtractionRuleSet]
    public let models: [String: ModelSpec]?

    enum CodingKeys: String, CodingKey {
        case version
        case bundleVersion = "bundle_version"
        case generatedAt = "generated_at"
        case source
        case domainIndex = "domain_index"
        case domainPatterns = "domain_patterns"
        case providers
        case extractionRules = "extraction_rules"
        case models
    }
}

/// Domain pattern for wildcard matching
public struct DomainPattern: Codable, Sendable {
    public let pattern: String
    public let provider: String
    public let regex: String
}

/// Provider specification
public struct ProviderSpec: Codable, Sendable {
    public let id: String
    public let displayName: String
    public let type: String?
    public let domains: [String]?
    public let features: [String]?
    public let auth: AuthSpec?

    enum CodingKeys: String, CodingKey {
        case id
        case displayName = "display_name"
        case type
        case domains
        case features
        case auth
    }
}

/// Authentication specification
public struct AuthSpec: Codable, Sendable {
    public let type: String?
    public let header: String?
    public let prefix: String?
    public let keyPrefixes: [String]?

    enum CodingKeys: String, CodingKey {
        case type
        case header
        case prefix
        case keyPrefixes = "key_prefixes"
    }
}

/// Extraction rules for a provider
public struct ExtractionRuleSet: Codable, Sendable {
    public let auth: AnyCodable?
    public let endpoints: [String: EndpointRules]?
}

/// Endpoint extraction rules
public struct EndpointRules: Codable, Sendable {
    public let path: String
    public let method: String
    public let requestType: String?
    public let requestExtraction: [String: AnyCodable]?
    public let responseExtraction: [String: AnyCodable]?

    enum CodingKeys: String, CodingKey {
        case path
        case method
        case requestType = "request_type"
        case requestExtraction = "request_extraction"
        case responseExtraction = "response_extraction"
    }
}

/// Model specification
public struct ModelSpec: Codable, Sendable {
    public let id: String
    public let provider: String
    public let mode: String?
    public let maxInputTokens: Int?
    public let maxOutputTokens: Int?
    public let inputCostPer1k: Double?
    public let outputCostPer1k: Double?

    enum CodingKeys: String, CodingKey {
        case id
        case provider
        case mode
        case maxInputTokens = "max_input_tokens"
        case maxOutputTokens = "max_output_tokens"
        case inputCostPer1k = "input_cost_per_1k"
        case outputCostPer1k = "output_cost_per_1k"
    }
}

/// Helper for decoding arbitrary JSON values
public struct AnyCodable: Codable, Sendable {
    public let value: Any

    public init(_ value: Any) {
        self.value = value
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()

        if container.decodeNil() {
            self.value = NSNull()
        } else if let bool = try? container.decode(Bool.self) {
            self.value = bool
        } else if let int = try? container.decode(Int.self) {
            self.value = int
        } else if let double = try? container.decode(Double.self) {
            self.value = double
        } else if let string = try? container.decode(String.self) {
            self.value = string
        } else if let array = try? container.decode([AnyCodable].self) {
            self.value = array.map { $0.value }
        } else if let dictionary = try? container.decode([String: AnyCodable].self) {
            self.value = dictionary.mapValues { $0.value }
        } else {
            throw DecodingError.dataCorruptedError(in: container, debugDescription: "AnyCodable cannot decode value")
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()

        switch value {
        case is NSNull:
            try container.encodeNil()
        case let bool as Bool:
            try container.encode(bool)
        case let int as Int:
            try container.encode(int)
        case let double as Double:
            try container.encode(double)
        case let string as String:
            try container.encode(string)
        case let array as [Any]:
            try container.encode(array.map { AnyCodable($0) })
        case let dictionary as [String: Any]:
            try container.encode(dictionary.mapValues { AnyCodable($0) })
        default:
            throw EncodingError.invalidValue(value, EncodingError.Context(codingPath: container.codingPath, debugDescription: "AnyCodable cannot encode value"))
        }
    }
}

// MARK: - Spec Bundle Loader

/// Loads and caches the OISP spec bundle
/// Thread-safe singleton that handles embedded, cached, and remote bundles
public final class SpecBundleLoader: @unchecked Sendable {

    /// Shared instance
    public static let shared = SpecBundleLoader()

    /// Current loaded bundle
    private var _bundle: OISPSpecBundle?
    private let lock = NSLock()

    /// Cache file path
    public let cachePath: URL

    /// Bundle URL for remote fetch
    public let bundleURL: URL

    /// Whether network fetching is enabled
    public var networkEnabled: Bool = true

    /// Last fetch time
    private var lastFetchTime: Date?

    private init() {
        // Setup cache path: ~/Library/Caches/com.oisp/spec-bundle.json
        let cacheDir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask).first!
        let oisDir = cacheDir.appendingPathComponent("com.oisp", isDirectory: true)
        try? FileManager.default.createDirectory(at: oisDir, withIntermediateDirectories: true)
        self.cachePath = oisDir.appendingPathComponent("spec-bundle.json")
        self.bundleURL = URL(string: defaultBundleURL)!

        // Load initial bundle
        loadBundle()
    }

    /// Get the current bundle (thread-safe)
    public var bundle: OISPSpecBundle? {
        lock.lock()
        defer { lock.unlock() }
        return _bundle
    }

    /// Load bundle using fallback strategy
    private func loadBundle() {
        // 1. Try cached bundle
        if let cached = loadFromCache() {
            lock.lock()
            _bundle = cached
            lock.unlock()
            print("[OISP] Loaded spec bundle from cache: \(cachePath.path)")
            return
        }

        // 2. Fall back to embedded bundle
        if let embedded = loadEmbedded() {
            lock.lock()
            _bundle = embedded
            lock.unlock()
            print("[OISP] Loaded embedded spec bundle")
            return
        }

        print("[OISP] WARNING: No spec bundle available!")
    }

    /// Load from cache file
    private func loadFromCache() -> OISPSpecBundle? {
        guard FileManager.default.fileExists(atPath: cachePath.path) else {
            return nil
        }

        do {
            let data = try Data(contentsOf: cachePath)
            let bundle = try JSONDecoder().decode(OISPSpecBundle.self, from: data)
            return bundle
        } catch {
            print("[OISP] Failed to load cached bundle: \(error)")
            return nil
        }
    }

    /// Load embedded bundle from app resources
    private func loadEmbedded() -> OISPSpecBundle? {
        // Look for bundle in app resources
        guard let url = Bundle.main.url(forResource: "oisp-spec-bundle", withExtension: "json") else {
            print("[OISP] Embedded bundle not found in app resources")
            return nil
        }

        do {
            let data = try Data(contentsOf: url)
            let bundle = try JSONDecoder().decode(OISPSpecBundle.self, from: data)
            return bundle
        } catch {
            print("[OISP] Failed to load embedded bundle: \(error)")
            return nil
        }
    }

    /// Check if bundle needs refresh
    public func needsRefresh() -> Bool {
        guard networkEnabled else { return false }

        // Check cache file age
        guard let attrs = try? FileManager.default.attributesOfItem(atPath: cachePath.path),
              let modDate = attrs[.modificationDate] as? Date else {
            return true
        }

        let age = Date().timeIntervalSince(modDate)
        return age > bundleRefreshInterval
    }

    /// Refresh bundle from network (async)
    public func refreshAsync(completion: ((Result<Bool, Error>) -> Void)? = nil) {
        guard networkEnabled else {
            completion?(.success(false))
            return
        }

        let request = URLRequest(url: bundleURL, cachePolicy: .reloadIgnoringLocalCacheData, timeoutInterval: 30)

        URLSession.shared.dataTask(with: request) { [weak self] data, response, error in
            guard let self = self else { return }

            if let error = error {
                print("[OISP] Failed to fetch bundle: \(error)")
                completion?(.failure(error))
                return
            }

            guard let data = data,
                  let httpResponse = response as? HTTPURLResponse,
                  httpResponse.statusCode == 200 else {
                let error = NSError(domain: "OISPSpecBundle", code: -1, userInfo: [NSLocalizedDescriptionKey: "Invalid response"])
                completion?(.failure(error))
                return
            }

            do {
                // Validate JSON before caching
                let newBundle = try JSONDecoder().decode(OISPSpecBundle.self, from: data)

                // Save to cache
                try data.write(to: self.cachePath, options: .atomic)

                // Update in-memory bundle
                self.lock.lock()
                self._bundle = newBundle
                self.lastFetchTime = Date()
                self.lock.unlock()

                print("[OISP] Refreshed spec bundle: v\(newBundle.bundleVersion)")
                completion?(.success(true))
            } catch {
                print("[OISP] Failed to parse fetched bundle: \(error)")
                completion?(.failure(error))
            }
        }.resume()
    }

    /// Refresh if needed (call on app launch)
    public func refreshIfNeeded() {
        if needsRefresh() {
            refreshAsync()
        }
    }
}

// MARK: - Provider Detection (Runtime)

/// Dynamic provider registry using runtime-loaded spec bundle
public final class DynamicProviderRegistry: @unchecked Sendable {

    /// Shared instance
    public static let shared = DynamicProviderRegistry()

    /// Compiled regex patterns (lazy)
    private var compiledPatterns: [(NSRegularExpression, String)]?
    private let lock = NSLock()

    private init() {}

    /// Get compiled patterns (lazy initialization)
    private func getCompiledPatterns() -> [(NSRegularExpression, String)] {
        lock.lock()
        defer { lock.unlock() }

        if let patterns = compiledPatterns {
            return patterns
        }

        var patterns: [(NSRegularExpression, String)] = []

        if let bundle = SpecBundleLoader.shared.bundle {
            for pattern in bundle.domainPatterns {
                if let regex = try? NSRegularExpression(pattern: pattern.regex, options: .caseInsensitive) {
                    patterns.append((regex, pattern.provider))
                }
            }
        }

        compiledPatterns = patterns
        return patterns
    }

    /// Invalidate cached patterns (call after bundle refresh)
    public func invalidatePatterns() {
        lock.lock()
        compiledPatterns = nil
        lock.unlock()
    }

    /// Detect provider from domain
    public func detectProvider(from domain: String) -> String? {
        guard let bundle = SpecBundleLoader.shared.bundle else {
            return nil
        }

        let lowercased = domain.lowercased()

        // 1. Exact match (fast O(1) lookup)
        if let provider = bundle.domainIndex[lowercased] {
            return provider
        }

        // Also try without port
        if let colonIndex = lowercased.firstIndex(of: ":") {
            let hostOnly = String(lowercased[..<colonIndex])
            if let provider = bundle.domainIndex[hostOnly] {
                return provider
            }
        }

        // 2. Regex patterns
        let patterns = getCompiledPatterns()
        let range = NSRange(lowercased.startIndex..., in: lowercased)

        for (regex, provider) in patterns {
            if regex.firstMatch(in: lowercased, options: [], range: range) != nil {
                return provider
            }
        }

        return nil
    }

    /// Check if domain is a known AI endpoint
    public func isKnownEndpoint(_ domain: String) -> Bool {
        return detectProvider(from: domain) != nil
    }

    /// Get all known domains
    public var allKnownDomains: [String] {
        guard let bundle = SpecBundleLoader.shared.bundle else {
            return []
        }
        return Array(bundle.domainIndex.keys)
    }

    /// Get provider display name
    public func displayName(for providerId: String) -> String {
        guard let bundle = SpecBundleLoader.shared.bundle,
              let provider = bundle.providers[providerId] else {
            return providerId
        }
        return provider.displayName
    }
}

// MARK: - Backward Compatibility

/// Provider enum for backward compatibility
/// Now dynamically populated from spec bundle
public enum AIProvider: String, CaseIterable, Sendable {
    // Core providers (always available)
    case openai
    case anthropic
    case google
    case azureOpenai = "azure_openai"
    case awsBedrock = "aws_bedrock"
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
    case ollama
    case lmstudio
    case unknown

    /// Detect provider from hostname (uses runtime bundle)
    public static func detect(from hostname: String) -> AIProvider {
        guard let providerId = DynamicProviderRegistry.shared.detectProvider(from: hostname) else {
            return .unknown
        }
        return AIProvider(rawValue: providerId) ?? .unknown
    }

    /// Check if hostname is a known AI endpoint
    public static func isKnownEndpoint(_ hostname: String) -> Bool {
        return DynamicProviderRegistry.shared.isKnownEndpoint(hostname)
    }
}
