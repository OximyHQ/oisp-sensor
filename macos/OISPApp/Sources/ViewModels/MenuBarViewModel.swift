// MenuBarViewModel.swift
// OISPApp
//
// ViewModel for the menu bar popover

import SwiftUI
import Combine

@MainActor
class MenuBarViewModel: ObservableObject {
    // MARK: - Published Properties

    @Published var isCapturing = false
    @Published var requestCount = 0
    @Published var totalTokens = 0
    @Published var totalCost: Double = 0.0
    @Published var avgLatencyMs = 0
    @Published var recentRequests: [RecentRequest] = []
    @Published var isExtensionEnabled = false
    @Published var isCATrusted = false

    // MARK: - Dependencies

    private let extensionManager = ExtensionManager.shared
    private var cancellables = Set<AnyCancellable>()

    // MARK: - Initialization

    init() {
        checkStatus()
        startPolling()
    }

    // MARK: - Status Check

    func checkStatus() {
        Task {
            isExtensionEnabled = await extensionManager.isEnabled
            isCATrusted = await CertificateAuthority.shared.isTrusted
        }
    }

    private func startPolling() {
        // Poll for status updates every 2 seconds
        Timer.publish(every: 2.0, on: .main, in: .common)
            .autoconnect()
            .sink { [weak self] _ in
                self?.checkStatus()
            }
            .store(in: &cancellables)
    }

    // MARK: - Actions

    func toggleCapture() {
        isCapturing.toggle()

        Task {
            if isCapturing {
                await extensionManager.resume()
            } else {
                await extensionManager.pause()
            }
        }
    }

    func enableExtension() {
        Task {
            do {
                try await extensionManager.requestActivation()
                isExtensionEnabled = await extensionManager.isEnabled
            } catch {
                // Show error to user
                print("Failed to enable extension: \(error)")
            }
        }
    }

    func trustCA() {
        Task {
            do {
                try await CertificateAuthority.shared.installTrust()
                isCATrusted = await CertificateAuthority.shared.isTrusted
            } catch {
                // Show error to user
                print("Failed to trust CA: \(error)")
            }
        }
    }

    func openDashboard() {
        if let url = URL(string: "http://localhost:3000") {
            NSWorkspace.shared.open(url)
        }
    }

    // MARK: - Event Handling

    func handleEvent(_ event: OISPDisplayEvent) {
        requestCount += 1

        if let tokens = event.totalTokens {
            totalTokens += tokens
        }

        if let cost = event.costUsd {
            totalCost += cost
        }

        let recent = RecentRequest(
            id: event.id,
            provider: event.provider,
            model: event.model,
            preview: event.preview,
            latencyMs: event.latencyMs ?? 0,
            tokens: event.totalTokens ?? 0
        )

        recentRequests.insert(recent, at: 0)
        if recentRequests.count > 20 {
            recentRequests.removeLast()
        }

        // Update average latency
        if let latency = event.latencyMs {
            avgLatencyMs = (avgLatencyMs * (requestCount - 1) + latency) / requestCount
        }
    }
}

// MARK: - Models

struct RecentRequest: Identifiable {
    let id: String
    let provider: AIProvider
    let model: String
    let preview: String
    let latencyMs: Int
    let tokens: Int
}

enum AIProvider: String {
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
    case local
    case unknown

    var icon: String {
        switch self {
        case .openai: return "circle.hexagongrid.fill"
        case .anthropic: return "a.circle.fill"
        case .google: return "g.circle.fill"
        case .azure: return "cloud.fill"
        case .aws: return "cloud.fill"
        case .cohere: return "c.circle.fill"
        case .mistral: return "m.circle.fill"
        case .groq: return "bolt.fill"
        case .together: return "person.2.fill"
        case .fireworks: return "flame.fill"
        case .perplexity: return "magnifyingglass"
        case .openrouter: return "arrow.triangle.branch"
        case .local: return "desktopcomputer"
        case .unknown: return "questionmark.circle"
        }
    }

    var color: Color {
        switch self {
        case .openai: return .green
        case .anthropic: return .orange
        case .google: return .blue
        case .azure: return .blue
        case .aws: return .orange
        case .cohere: return .purple
        case .mistral: return .cyan
        case .groq: return .yellow
        case .together: return .indigo
        case .fireworks: return .red
        case .perplexity: return .teal
        case .openrouter: return .mint
        case .local: return .gray
        case .unknown: return .secondary
        }
    }
}

/// Event for display in the UI
struct OISPDisplayEvent {
    let id: String
    let provider: AIProvider
    let model: String
    let preview: String
    let latencyMs: Int?
    let totalTokens: Int?
    let costUsd: Double?
}
