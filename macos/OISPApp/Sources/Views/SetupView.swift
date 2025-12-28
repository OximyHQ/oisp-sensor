// SetupView.swift
// OISPApp
//
// First-launch setup wizard

import SwiftUI

struct SetupView: View {
    @State private var currentStep = 0
    @State private var extensionEnabled = false
    @State private var certificateTrusted = false
    @State private var isProcessing = false
    @State private var errorMessage: String?

    private let steps = [
        "Welcome",
        "Enable Extension",
        "Trust Certificate",
        "Complete"
    ]

    var body: some View {
        VStack(spacing: 0) {
            // Progress indicator
            progressIndicator

            Divider()

            // Content
            stepContent
                .frame(maxWidth: .infinity, maxHeight: .infinity)

            Divider()

            // Navigation
            navigationButtons
        }
        .frame(width: 500, height: 400)
    }

    // MARK: - Progress Indicator

    private var progressIndicator: some View {
        HStack(spacing: 0) {
            ForEach(0..<steps.count, id: \.self) { index in
                HStack(spacing: 8) {
                    Circle()
                        .fill(index <= currentStep ? Color.accentColor : Color.gray.opacity(0.3))
                        .frame(width: 24, height: 24)
                        .overlay {
                            if index < currentStep {
                                Image(systemName: "checkmark")
                                    .font(.caption)
                                    .foregroundColor(.white)
                            } else {
                                Text("\(index + 1)")
                                    .font(.caption)
                                    .foregroundColor(index == currentStep ? .white : .gray)
                            }
                        }

                    Text(steps[index])
                        .font(.caption)
                        .foregroundColor(index == currentStep ? .primary : .secondary)
                }

                if index < steps.count - 1 {
                    Rectangle()
                        .fill(index < currentStep ? Color.accentColor : Color.gray.opacity(0.3))
                        .frame(height: 2)
                        .padding(.horizontal, 8)
                }
            }
        }
        .padding()
    }

    // MARK: - Step Content

    @ViewBuilder
    private var stepContent: some View {
        switch currentStep {
        case 0:
            welcomeStep
        case 1:
            extensionStep
        case 2:
            certificateStep
        case 3:
            completeStep
        default:
            EmptyView()
        }
    }

    private var welcomeStep: some View {
        VStack(spacing: 20) {
            Image(systemName: "brain")
                .font(.system(size: 60))
                .foregroundColor(.accentColor)

            Text("Welcome to OISP")
                .font(.largeTitle)
                .fontWeight(.bold)

            Text("Observability for Intelligent Systems Platform")
                .font(.headline)
                .foregroundColor(.secondary)

            Text("OISP captures AI API traffic from any application on your Mac, providing real-time visibility into LLM usage, costs, and performance.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
                .padding(.horizontal, 40)

            VStack(alignment: .leading, spacing: 8) {
                FeatureRow(icon: "eye", text: "Monitor all AI API calls")
                FeatureRow(icon: "dollarsign.circle", text: "Track token usage and costs")
                FeatureRow(icon: "clock", text: "Measure latency and performance")
                FeatureRow(icon: "lock.shield", text: "Local-only, privacy-first")
            }
            .padding(.top, 10)
        }
        .padding()
    }

    private var extensionStep: some View {
        VStack(spacing: 20) {
            Image(systemName: "network.badge.shield.half.filled")
                .font(.system(size: 50))
                .foregroundColor(.blue)

            Text("Enable Network Extension")
                .font(.title)
                .fontWeight(.bold)

            Text("OISP needs a network extension to capture AI API traffic. This extension runs locally and only intercepts connections to known AI providers.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
                .padding(.horizontal, 40)

            if extensionEnabled {
                Label("Extension Enabled", systemImage: "checkmark.circle.fill")
                    .foregroundColor(.green)
                    .font(.headline)
            } else {
                VStack(spacing: 12) {
                    Button(action: enableExtension) {
                        HStack {
                            if isProcessing {
                                ProgressView()
                                    .scaleEffect(0.8)
                            }
                            Text("Enable Extension")
                        }
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(isProcessing)

                    Text("You'll be prompted to approve the extension in System Settings")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }

            if let error = errorMessage {
                Text(error)
                    .foregroundColor(.red)
                    .font(.caption)
            }
        }
        .padding()
    }

    private var certificateStep: some View {
        VStack(spacing: 20) {
            Image(systemName: "lock.shield.fill")
                .font(.system(size: 50))
                .foregroundColor(.orange)

            Text("Trust CA Certificate")
                .font(.title)
                .fontWeight(.bold)

            Text("To decrypt HTTPS traffic to AI APIs, OISP uses a local Certificate Authority. This certificate never leaves your Mac and is used only for local interception.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
                .padding(.horizontal, 40)

            if certificateTrusted {
                Label("Certificate Trusted", systemImage: "checkmark.circle.fill")
                    .foregroundColor(.green)
                    .font(.headline)
            } else {
                VStack(spacing: 12) {
                    Button(action: trustCertificate) {
                        HStack {
                            if isProcessing {
                                ProgressView()
                                    .scaleEffect(0.8)
                            }
                            Text("Trust Certificate")
                        }
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(isProcessing)

                    Text("You'll be prompted to enter your password")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }

            if let error = errorMessage {
                Text(error)
                    .foregroundColor(.red)
                    .font(.caption)
            }
        }
        .padding()
    }

    private var completeStep: some View {
        VStack(spacing: 20) {
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 60))
                .foregroundColor(.green)

            Text("Setup Complete!")
                .font(.largeTitle)
                .fontWeight(.bold)

            Text("OISP is now ready to capture AI API traffic.")
                .font(.headline)
                .foregroundColor(.secondary)

            VStack(alignment: .leading, spacing: 12) {
                InfoRow(icon: "menubar.rectangle", text: "OISP runs in your menu bar")
                InfoRow(icon: "play.fill", text: "Capture starts automatically")
                InfoRow(icon: "gear", text: "Configure in Settings")
            }
            .padding(.top, 10)
        }
        .padding()
    }

    // MARK: - Navigation

    private var navigationButtons: some View {
        HStack {
            if currentStep > 0 && currentStep < steps.count - 1 {
                Button("Back") {
                    currentStep -= 1
                }
            }

            Spacer()

            if currentStep < steps.count - 1 {
                Button(currentStep == 0 ? "Get Started" : "Continue") {
                    nextStep()
                }
                .buttonStyle(.borderedProminent)
                .disabled(shouldDisableContinue)
            } else {
                Button("Finish") {
                    closeWindow()
                }
                .buttonStyle(.borderedProminent)
            }
        }
        .padding()
    }

    private var shouldDisableContinue: Bool {
        switch currentStep {
        case 1:
            return !extensionEnabled
        case 2:
            return !certificateTrusted
        default:
            return false
        }
    }

    // MARK: - Actions

    private func nextStep() {
        errorMessage = nil
        currentStep += 1
    }

    private func enableExtension() {
        isProcessing = true
        errorMessage = nil

        Task {
            do {
                try await ExtensionManager.shared.requestActivation()
                extensionEnabled = await ExtensionManager.shared.isEnabled
            } catch {
                errorMessage = error.localizedDescription
            }
            isProcessing = false
        }
    }

    private func trustCertificate() {
        isProcessing = true
        errorMessage = nil

        Task {
            do {
                try await CertificateAuthority.shared.initialize()
                try await CertificateAuthority.shared.installTrust()
                certificateTrusted = await CertificateAuthority.shared.isTrusted
            } catch {
                errorMessage = error.localizedDescription
            }
            isProcessing = false
        }
    }

    private func closeWindow() {
        NSApp.keyWindow?.close()
    }
}

// MARK: - Supporting Views

struct FeatureRow: View {
    let icon: String
    let text: String

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .foregroundColor(.accentColor)
                .frame(width: 24)
            Text(text)
        }
    }
}

struct InfoRow: View {
    let icon: String
    let text: String

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .foregroundColor(.secondary)
                .frame(width: 24)
            Text(text)
                .foregroundColor(.secondary)
        }
    }
}

// MARK: - Preview

#Preview {
    SetupView()
}
