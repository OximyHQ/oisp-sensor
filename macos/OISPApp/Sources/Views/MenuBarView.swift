// MenuBarView.swift
// OISPApp
//
// Main popover view for the menu bar dropdown

import SwiftUI

struct MenuBarView: View {
    @StateObject private var viewModel = MenuBarViewModel()

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Status Header
            statusHeader

            // Stats
            if viewModel.isCapturing {
                statsRow
            }

            Divider()

            // Recent Requests
            recentRequestsSection

            Divider()

            // Warnings
            warningsSection

            Divider()

            // Actions
            actionsRow
        }
        .padding()
        .frame(width: 360)
    }

    // MARK: - Status Header

    private var statusHeader: some View {
        HStack {
            Circle()
                .fill(viewModel.isCapturing ? Color.green : Color.gray)
                .frame(width: 10, height: 10)

            Text(viewModel.isCapturing ? "Capturing" : "Paused")
                .font(.headline)

            Spacer()

            Text("\(viewModel.requestCount) requests")
                .font(.subheadline)
                .foregroundColor(.secondary)
        }
    }

    // MARK: - Stats Row

    private var statsRow: some View {
        HStack(spacing: 20) {
            StatView(label: "Tokens", value: formatNumber(viewModel.totalTokens))
            StatView(label: "Cost", value: formatCost(viewModel.totalCost))
            StatView(label: "Avg Latency", value: "\(viewModel.avgLatencyMs)ms")
        }
        .font(.caption)
    }

    // MARK: - Recent Requests

    private var recentRequestsSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Recent Requests")
                .font(.subheadline)
                .foregroundColor(.secondary)

            if viewModel.recentRequests.isEmpty {
                Text("No requests captured yet")
                    .foregroundColor(.secondary)
                    .font(.caption)
                    .padding(.vertical, 8)
            } else {
                ForEach(viewModel.recentRequests.prefix(5)) { request in
                    RecentRequestRow(request: request)
                }
            }
        }
    }

    // MARK: - Warnings

    @ViewBuilder
    private var warningsSection: some View {
        if !viewModel.isExtensionEnabled {
            WarningRow(
                icon: "exclamationmark.triangle.fill",
                message: "Extension not enabled",
                action: "Enable",
                onAction: viewModel.enableExtension
            )
        }

        if !viewModel.isCATrusted {
            WarningRow(
                icon: "lock.open.fill",
                message: "CA certificate not trusted",
                action: "Trust",
                onAction: viewModel.trustCA
            )
        }
    }

    // MARK: - Actions

    private var actionsRow: some View {
        HStack {
            Button(action: viewModel.toggleCapture) {
                Label(
                    viewModel.isCapturing ? "Pause" : "Resume",
                    systemImage: viewModel.isCapturing ? "pause.fill" : "play.fill"
                )
            }
            .buttonStyle(.borderless)

            Spacer()

            Button(action: viewModel.openDashboard) {
                Label("Dashboard", systemImage: "chart.bar")
            }
            .buttonStyle(.borderless)

            Spacer()

            Button(action: { NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil) }) {
                Label("Settings", systemImage: "gear")
            }
            .buttonStyle(.borderless)

            Spacer()

            Button(action: { NSApp.terminate(nil) }) {
                Label("Quit", systemImage: "power")
            }
            .buttonStyle(.borderless)
        }
    }

    // MARK: - Helpers

    private func formatNumber(_ value: Int) -> String {
        if value >= 1_000_000 {
            return String(format: "%.1fM", Double(value) / 1_000_000)
        } else if value >= 1_000 {
            return String(format: "%.1fK", Double(value) / 1_000)
        }
        return "\(value)"
    }

    private func formatCost(_ cost: Double) -> String {
        String(format: "$%.2f", cost)
    }
}

// MARK: - Supporting Views

struct StatView: View {
    let label: String
    let value: String

    var body: some View {
        VStack(spacing: 2) {
            Text(value)
                .font(.system(.body, design: .monospaced))
                .fontWeight(.medium)
            Text(label)
                .font(.caption2)
                .foregroundColor(.secondary)
        }
    }
}

struct RecentRequestRow: View {
    let request: RecentRequest

    var body: some View {
        HStack {
            Image(systemName: request.provider.icon)
                .foregroundColor(request.provider.color)
                .frame(width: 20)

            VStack(alignment: .leading, spacing: 2) {
                Text(request.model)
                    .font(.caption)
                    .fontWeight(.medium)

                Text(request.preview)
                    .font(.caption2)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 2) {
                Text("\(request.latencyMs)ms")
                    .font(.caption2)

                Text("\(request.tokens) tok")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 2)
    }
}

struct WarningRow: View {
    let icon: String
    let message: String
    let action: String
    let onAction: () -> Void

    var body: some View {
        HStack {
            Image(systemName: icon)
                .foregroundColor(.orange)

            Text(message)
                .font(.caption)

            Spacer()

            Button(action) {
                onAction()
            }
            .font(.caption)
            .buttonStyle(.borderless)
        }
    }
}

// MARK: - Preview

#Preview {
    MenuBarView()
        .frame(width: 360, height: 400)
}
