// SettingsView.swift
// OISPApp
//
// Settings window with tabs for different configuration areas

import SwiftUI

struct SettingsView: View {
    var body: some View {
        TabView {
            GeneralSettingsView()
                .tabItem {
                    Label("General", systemImage: "gear")
                }

            CertificateSettingsView()
                .tabItem {
                    Label("Certificate", systemImage: "lock.shield")
                }

            ProviderSettingsView()
                .tabItem {
                    Label("Providers", systemImage: "server.rack")
                }

            AdvancedSettingsView()
                .tabItem {
                    Label("Advanced", systemImage: "gearshape.2")
                }
        }
        .frame(width: 500, height: 400)
    }
}

// MARK: - General Settings

struct GeneralSettingsView: View {
    @AppStorage("launchAtLogin") private var launchAtLogin = false
    @AppStorage("showInDock") private var showInDock = false
    @AppStorage("captureOnLaunch") private var captureOnLaunch = true

    var body: some View {
        Form {
            Section {
                Toggle("Launch OISP at login", isOn: $launchAtLogin)
                Toggle("Show in Dock", isOn: $showInDock)
                Toggle("Start capturing on launch", isOn: $captureOnLaunch)
            } header: {
                Text("Startup")
            }

            Section {
                ExtensionStatusView()
            } header: {
                Text("Network Extension")
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

struct ExtensionStatusView: View {
    @State private var isEnabled = false
    @State private var isChecking = true

    var body: some View {
        HStack {
            Circle()
                .fill(isEnabled ? Color.green : Color.red)
                .frame(width: 10, height: 10)

            Text(isEnabled ? "Extension Enabled" : "Extension Not Enabled")

            Spacer()

            if isChecking {
                ProgressView()
                    .scaleEffect(0.5)
            } else if !isEnabled {
                Button("Enable") {
                    enableExtension()
                }
            }
        }
        .task {
            await checkStatus()
        }
    }

    private func checkStatus() async {
        isChecking = true
        isEnabled = await ExtensionManager.shared.isEnabled
        isChecking = false
    }

    private func enableExtension() {
        Task {
            try? await ExtensionManager.shared.requestActivation()
            await checkStatus()
        }
    }
}

// MARK: - Certificate Settings

struct CertificateSettingsView: View {
    @State private var isTrusted = false
    @State private var isChecking = true
    @State private var showExportSheet = false

    var body: some View {
        Form {
            Section {
                HStack {
                    Circle()
                        .fill(isTrusted ? Color.green : Color.orange)
                        .frame(width: 10, height: 10)

                    Text(isTrusted ? "CA Certificate Trusted" : "CA Certificate Not Trusted")

                    Spacer()

                    if isChecking {
                        ProgressView()
                            .scaleEffect(0.5)
                    } else if !isTrusted {
                        Button("Trust Certificate") {
                            trustCertificate()
                        }
                    }
                }

                Text("OISP uses a local Certificate Authority to intercept HTTPS traffic to AI APIs. The CA certificate must be trusted for interception to work.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } header: {
                Text("Trust Status")
            }

            Section {
                Button("Export CA Certificate...") {
                    exportCertificate()
                }

                Button("Regenerate CA Certificate") {
                    regenerateCertificate()
                }
                .foregroundColor(.red)
            } header: {
                Text("Certificate Management")
            }
        }
        .formStyle(.grouped)
        .padding()
        .task {
            await checkTrustStatus()
        }
    }

    private func checkTrustStatus() async {
        isChecking = true
        isTrusted = await CertificateAuthority.shared.isTrusted
        isChecking = false
    }

    private func trustCertificate() {
        Task {
            try? await CertificateAuthority.shared.installTrust()
            await checkTrustStatus()
        }
    }

    private func exportCertificate() {
        Task {
            guard let pem = try? await CertificateAuthority.shared.exportCACertificatePEM() else {
                return
            }

            let panel = NSSavePanel()
            panel.nameFieldStringValue = "OISP-CA.pem"
            panel.allowedContentTypes = [.x509Certificate]

            if panel.runModal() == .OK, let url = panel.url {
                try? pem.write(to: url, atomically: true, encoding: .utf8)
            }
        }
    }

    private func regenerateCertificate() {
        // Show confirmation dialog first
        // Then regenerate
    }
}

// MARK: - Provider Settings

struct ProviderSettingsView: View {
    @State private var customEndpoints: [String] = []
    @State private var newEndpoint = ""

    var body: some View {
        Form {
            Section {
                Text("OISP intercepts traffic to these AI providers by default:")
                    .font(.caption)
                    .foregroundColor(.secondary)

                VStack(alignment: .leading, spacing: 4) {
                    ProviderRow(name: "OpenAI", endpoint: "api.openai.com")
                    ProviderRow(name: "Anthropic", endpoint: "api.anthropic.com")
                    ProviderRow(name: "Google AI", endpoint: "generativelanguage.googleapis.com")
                    ProviderRow(name: "Azure OpenAI", endpoint: "*.openai.azure.com")
                    ProviderRow(name: "AWS Bedrock", endpoint: "bedrock-runtime.*.amazonaws.com")
                    ProviderRow(name: "Mistral", endpoint: "api.mistral.ai")
                    ProviderRow(name: "Groq", endpoint: "api.groq.com")
                }
            } header: {
                Text("Default Providers")
            }

            Section {
                ForEach(customEndpoints, id: \.self) { endpoint in
                    HStack {
                        Text(endpoint)
                        Spacer()
                        Button(role: .destructive) {
                            customEndpoints.removeAll { $0 == endpoint }
                        } label: {
                            Image(systemName: "trash")
                        }
                        .buttonStyle(.borderless)
                    }
                }

                HStack {
                    TextField("Add endpoint...", text: $newEndpoint)
                        .textFieldStyle(.roundedBorder)

                    Button("Add") {
                        if !newEndpoint.isEmpty {
                            customEndpoints.append(newEndpoint)
                            newEndpoint = ""
                        }
                    }
                    .disabled(newEndpoint.isEmpty)
                }
            } header: {
                Text("Custom Endpoints")
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

struct ProviderRow: View {
    let name: String
    let endpoint: String

    var body: some View {
        HStack {
            Text(name)
                .fontWeight(.medium)
            Spacer()
            Text(endpoint)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.vertical, 2)
    }
}

// MARK: - Advanced Settings

struct AdvancedSettingsView: View {
    @AppStorage("socketPath") private var socketPath = "/tmp/oisp.sock"
    @AppStorage("logLevel") private var logLevel = "info"
    @AppStorage("maxPendingEvents") private var maxPendingEvents = 10000

    var body: some View {
        Form {
            Section {
                TextField("Socket Path", text: $socketPath)

                Picker("Log Level", selection: $logLevel) {
                    Text("Error").tag("error")
                    Text("Warning").tag("warn")
                    Text("Info").tag("info")
                    Text("Debug").tag("debug")
                    Text("Trace").tag("trace")
                }
            } header: {
                Text("Sensor Connection")
            }

            Section {
                TextField("Max Pending Events", value: $maxPendingEvents, format: .number)
            } header: {
                Text("Buffering")
            }

            Section {
                Button("Reset to Defaults") {
                    resetToDefaults()
                }
            }
        }
        .formStyle(.grouped)
        .padding()
    }

    private func resetToDefaults() {
        socketPath = "/tmp/oisp.sock"
        logLevel = "info"
        maxPendingEvents = 10000
    }
}

// MARK: - Preview

#Preview {
    SettingsView()
}
