// CertificateAuthority.swift
// OISPCore
//
// Local Certificate Authority for TLS MITM
// Generates CA certificate and per-host certificates for interception

import Foundation
import Security
import CryptoKit

/// Errors that can occur during certificate operations
public enum CertificateAuthorityError: Error, LocalizedError {
    case keyGenerationFailed(OSStatus)
    case certificateGenerationFailed(String)
    case keychainError(OSStatus)
    case identityCreationFailed
    case caNotInitialized
    case invalidCertificate
    case trustSettingsFailed(OSStatus)

    public var errorDescription: String? {
        switch self {
        case .keyGenerationFailed(let status):
            return "Failed to generate key pair: \(SecCopyErrorMessageString(status, nil) ?? "Unknown" as CFString)"
        case .certificateGenerationFailed(let reason):
            return "Failed to generate certificate: \(reason)"
        case .keychainError(let status):
            return "Keychain error: \(SecCopyErrorMessageString(status, nil) ?? "Unknown" as CFString)"
        case .identityCreationFailed:
            return "Failed to create identity from certificate and key"
        case .caNotInitialized:
            return "Certificate Authority not initialized"
        case .invalidCertificate:
            return "Invalid certificate data"
        case .trustSettingsFailed(let status):
            return "Failed to set trust settings: \(SecCopyErrorMessageString(status, nil) ?? "Unknown" as CFString)"
        }
    }
}

/// Local Certificate Authority for OISP
public actor CertificateAuthority {
    // MARK: - Singleton

    public static let shared = CertificateAuthority()

    // MARK: - Configuration

    private let keychainTag = "com.oisp.ca.privatekey"
    private let certLabel = "OISP Local CA"
    private let caCommonName = "OISP Local CA"
    private let caOrganization = "OISP"

    /// CA key size in bits
    private let caKeySize = 4096

    /// Per-host certificate key size
    private let hostKeySize = 2048

    /// CA certificate validity in days
    private let caValidityDays = 3650 // 10 years

    /// Per-host certificate validity in days
    private let hostValidityDays = 1 // 24 hours (short-lived for security)

    // MARK: - State

    private var caPrivateKey: SecKey?
    private var caCertificate: SecCertificate?
    private var isInitialized = false

    /// Cache of generated per-host certificates
    private var certificateCache: [String: CachedCertificate] = [:]

    private struct CachedCertificate {
        let identity: SecIdentity
        let expiresAt: Date
    }

    // MARK: - Initialization

    private init() {}

    /// Initialize the CA - creates or loads existing CA
    public func initialize() async throws {
        guard !isInitialized else { return }

        // Try to load existing CA from keychain
        if let existing = try? await loadCAFromKeychain() {
            caPrivateKey = existing.privateKey
            caCertificate = existing.certificate
            isInitialized = true
            return
        }

        // Generate new CA
        try await generateNewCA()
        isInitialized = true
    }

    // MARK: - CA Generation

    private func generateNewCA() async throws {
        // 1. Generate RSA key pair
        let keyParams: [String: Any] = [
            kSecAttrKeyType as String: kSecAttrKeyTypeRSA,
            kSecAttrKeySizeInBits as String: caKeySize,
            kSecAttrIsPermanent as String: true,
            kSecPrivateKeyAttrs as String: [
                kSecAttrApplicationTag as String: keychainTag.data(using: .utf8)!,
                kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlock,
                kSecAttrCanSign as String: true,
                kSecAttrCanDecrypt as String: true,
            ] as [String: Any]
        ]

        var error: Unmanaged<CFError>?
        guard let privateKey = SecKeyCreateRandomKey(keyParams as CFDictionary, &error) else {
            if let cfError = error?.takeRetainedValue() {
                let nsError = cfError as Error as NSError
                throw CertificateAuthorityError.keyGenerationFailed(OSStatus(nsError.code))
            }
            throw CertificateAuthorityError.keyGenerationFailed(errSecParam)
        }

        // 2. Generate self-signed CA certificate
        let certificate = try await generateSelfSignedCACertificate(privateKey: privateKey)

        // 3. Store certificate in keychain
        try await storeCertificate(certificate)

        caPrivateKey = privateKey
        caCertificate = certificate
    }

    private func generateSelfSignedCACertificate(privateKey: SecKey) async throws -> SecCertificate {
        // Get public key
        guard let publicKey = SecKeyCopyPublicKey(privateKey) else {
            throw CertificateAuthorityError.certificateGenerationFailed("Cannot get public key")
        }

        // Create certificate using Security framework
        // Note: For production, use swift-certificates library for proper X.509 generation
        // This is a simplified version using SecCertificateCreateWithData

        // For now, we'll use a shell command to generate the certificate
        // This is a workaround until swift-certificates is integrated
        let cert = try await generateCertificateViaOpenSSL(
            commonName: caCommonName,
            organization: caOrganization,
            privateKey: privateKey,
            isCA: true,
            validityDays: caValidityDays,
            signingKey: nil,
            issuerCert: nil
        )

        return cert
    }

    // MARK: - Per-Host Certificate Generation

    /// Generate a certificate for a specific hostname
    public func generateCertificate(for hostname: String) async throws -> SecIdentity {
        guard isInitialized, let caKey = caPrivateKey, let caCert = caCertificate else {
            throw CertificateAuthorityError.caNotInitialized
        }

        // Check cache first
        if let cached = certificateCache[hostname], cached.expiresAt > Date() {
            return cached.identity
        }

        // Generate new key pair for this host
        let keyParams: [String: Any] = [
            kSecAttrKeyType as String: kSecAttrKeyTypeRSA,
            kSecAttrKeySizeInBits as String: hostKeySize,
        ]

        var error: Unmanaged<CFError>?
        guard let privateKey = SecKeyCreateRandomKey(keyParams as CFDictionary, &error) else {
            throw CertificateAuthorityError.keyGenerationFailed(errSecParam)
        }

        // Generate certificate signed by CA
        let certificate = try await generateCertificateViaOpenSSL(
            commonName: hostname,
            organization: "OISP Generated",
            privateKey: privateKey,
            isCA: false,
            validityDays: hostValidityDays,
            signingKey: caKey,
            issuerCert: caCert,
            subjectAltNames: [hostname]
        )

        // Create identity (certificate + private key)
        let identity = try createIdentity(certificate: certificate, privateKey: privateKey)

        // Cache it
        let expires = Date().addingTimeInterval(Double(hostValidityDays) * 24 * 60 * 60)
        certificateCache[hostname] = CachedCertificate(identity: identity, expiresAt: expires)

        return identity
    }

    // MARK: - OpenSSL Certificate Generation

    /// Generate certificate using OpenSSL command line
    /// This is a temporary solution until swift-certificates is integrated
    private func generateCertificateViaOpenSSL(
        commonName: String,
        organization: String,
        privateKey: SecKey,
        isCA: Bool,
        validityDays: Int,
        signingKey: SecKey?,
        issuerCert: SecCertificate?,
        subjectAltNames: [String]? = nil
    ) async throws -> SecCertificate {
        // Export private key to PEM
        let keyPEM = try exportPrivateKeyToPEM(privateKey)

        // Create temp directory
        let tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("oisp-cert-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        defer {
            try? FileManager.default.removeItem(at: tempDir)
        }

        let keyPath = tempDir.appendingPathComponent("key.pem")
        let certPath = tempDir.appendingPathComponent("cert.pem")
        let csrPath = tempDir.appendingPathComponent("csr.pem")
        let configPath = tempDir.appendingPathComponent("openssl.cnf")

        // Write private key
        try keyPEM.write(to: keyPath, atomically: true, encoding: .utf8)

        // Create OpenSSL config
        var configContent = """
        [req]
        distinguished_name = req_distinguished_name
        x509_extensions = v3_ext
        prompt = no

        [req_distinguished_name]
        CN = \(commonName)
        O = \(organization)

        [v3_ext]
        """

        if isCA {
            configContent += """

            basicConstraints = critical, CA:TRUE
            keyUsage = critical, keyCertSign, cRLSign
            subjectKeyIdentifier = hash
            """
        } else {
            configContent += """

            basicConstraints = critical, CA:FALSE
            keyUsage = critical, digitalSignature, keyEncipherment
            extendedKeyUsage = serverAuth
            subjectKeyIdentifier = hash
            """

            if let sans = subjectAltNames, !sans.isEmpty {
                configContent += "\nsubjectAltName = @alt_names\n\n[alt_names]\n"
                for (index, san) in sans.enumerated() {
                    configContent += "DNS.\(index + 1) = \(san)\n"
                }
            }
        }

        try configContent.write(to: configPath, atomically: true, encoding: .utf8)

        // Generate certificate
        if isCA || signingKey == nil {
            // Self-signed certificate
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/openssl")
            process.arguments = [
                "req", "-new", "-x509",
                "-key", keyPath.path,
                "-out", certPath.path,
                "-days", String(validityDays),
                "-config", configPath.path
            ]
            try process.run()
            process.waitUntilExit()

            guard process.terminationStatus == 0 else {
                throw CertificateAuthorityError.certificateGenerationFailed("OpenSSL failed with status \(process.terminationStatus)")
            }
        } else {
            // CA-signed certificate
            guard let caKey = signingKey, let caCert = issuerCert else {
                throw CertificateAuthorityError.caNotInitialized
            }

            let caKeyPath = tempDir.appendingPathComponent("ca-key.pem")
            let caCertPath = tempDir.appendingPathComponent("ca-cert.pem")

            // Export CA key and cert
            let caKeyPEM = try exportPrivateKeyToPEM(caKey)
            try caKeyPEM.write(to: caKeyPath, atomically: true, encoding: .utf8)

            let caCertPEM = try exportCertificateToPEM(caCert)
            try caCertPEM.write(to: caCertPath, atomically: true, encoding: .utf8)

            // Generate CSR
            let csrProcess = Process()
            csrProcess.executableURL = URL(fileURLWithPath: "/usr/bin/openssl")
            csrProcess.arguments = [
                "req", "-new",
                "-key", keyPath.path,
                "-out", csrPath.path,
                "-config", configPath.path
            ]
            try csrProcess.run()
            csrProcess.waitUntilExit()

            // Sign with CA
            let signProcess = Process()
            signProcess.executableURL = URL(fileURLWithPath: "/usr/bin/openssl")
            signProcess.arguments = [
                "x509", "-req",
                "-in", csrPath.path,
                "-CA", caCertPath.path,
                "-CAkey", caKeyPath.path,
                "-CAcreateserial",
                "-out", certPath.path,
                "-days", String(validityDays),
                "-extfile", configPath.path,
                "-extensions", "v3_ext"
            ]
            try signProcess.run()
            signProcess.waitUntilExit()

            guard signProcess.terminationStatus == 0 else {
                throw CertificateAuthorityError.certificateGenerationFailed("OpenSSL signing failed")
            }
        }

        // Read generated certificate
        let certPEM = try String(contentsOf: certPath, encoding: .utf8)
        return try importCertificateFromPEM(certPEM)
    }

    // MARK: - PEM Export/Import

    private func exportPrivateKeyToPEM(_ key: SecKey) throws -> String {
        var error: Unmanaged<CFError>?
        guard let keyData = SecKeyCopyExternalRepresentation(key, &error) as Data? else {
            throw CertificateAuthorityError.certificateGenerationFailed("Cannot export key")
        }

        let base64 = keyData.base64EncodedString(options: [.lineLength64Characters, .endLineWithLineFeed])
        return "-----BEGIN RSA PRIVATE KEY-----\n\(base64)\n-----END RSA PRIVATE KEY-----\n"
    }

    private func exportCertificateToPEM(_ cert: SecCertificate) throws -> String {
        let certData = SecCertificateCopyData(cert) as Data
        let base64 = certData.base64EncodedString(options: [.lineLength64Characters, .endLineWithLineFeed])
        return "-----BEGIN CERTIFICATE-----\n\(base64)\n-----END CERTIFICATE-----\n"
    }

    private func importCertificateFromPEM(_ pem: String) throws -> SecCertificate {
        // Extract base64 content
        let lines = pem.components(separatedBy: .newlines)
        let base64 = lines.filter { !$0.hasPrefix("-----") }.joined()

        guard let certData = Data(base64Encoded: base64) else {
            throw CertificateAuthorityError.invalidCertificate
        }

        guard let certificate = SecCertificateCreateWithData(nil, certData as CFData) else {
            throw CertificateAuthorityError.invalidCertificate
        }

        return certificate
    }

    // MARK: - Keychain Operations

    private func loadCAFromKeychain() async throws -> (privateKey: SecKey, certificate: SecCertificate)? {
        // Query for private key
        let keyQuery: [String: Any] = [
            kSecClass as String: kSecClassKey,
            kSecAttrApplicationTag as String: keychainTag.data(using: .utf8)!,
            kSecReturnRef as String: true
        ]

        var keyItem: CFTypeRef?
        let keyStatus = SecItemCopyMatching(keyQuery as CFDictionary, &keyItem)

        guard keyStatus == errSecSuccess, let privateKey = keyItem as! SecKey? else {
            return nil
        }

        // Query for certificate
        let certQuery: [String: Any] = [
            kSecClass as String: kSecClassCertificate,
            kSecAttrLabel as String: certLabel,
            kSecReturnRef as String: true
        ]

        var certItem: CFTypeRef?
        let certStatus = SecItemCopyMatching(certQuery as CFDictionary, &certItem)

        guard certStatus == errSecSuccess, let certificate = certItem as! SecCertificate? else {
            return nil
        }

        return (privateKey, certificate)
    }

    private func storeCertificate(_ certificate: SecCertificate) async throws {
        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassCertificate,
            kSecValueRef as String: certificate,
            kSecAttrLabel as String: certLabel
        ]

        // Delete existing if any
        SecItemDelete(addQuery as CFDictionary)

        let status = SecItemAdd(addQuery as CFDictionary, nil)
        guard status == errSecSuccess || status == errSecDuplicateItem else {
            throw CertificateAuthorityError.keychainError(status)
        }
    }

    private func createIdentity(certificate: SecCertificate, privateKey: SecKey) throws -> SecIdentity {
        // Store private key temporarily to create identity
        let tempTag = "com.oisp.temp.\(UUID().uuidString)"

        let addKeyQuery: [String: Any] = [
            kSecClass as String: kSecClassKey,
            kSecValueRef as String: privateKey,
            kSecAttrApplicationTag as String: tempTag.data(using: .utf8)!,
        ]

        var status = SecItemAdd(addKeyQuery as CFDictionary, nil)
        guard status == errSecSuccess || status == errSecDuplicateItem else {
            throw CertificateAuthorityError.keychainError(status)
        }

        // Store certificate temporarily
        let tempCertLabel = "com.oisp.temp.cert.\(UUID().uuidString)"
        let addCertQuery: [String: Any] = [
            kSecClass as String: kSecClassCertificate,
            kSecValueRef as String: certificate,
            kSecAttrLabel as String: tempCertLabel,
        ]

        status = SecItemAdd(addCertQuery as CFDictionary, nil)
        guard status == errSecSuccess || status == errSecDuplicateItem else {
            throw CertificateAuthorityError.keychainError(status)
        }

        // Query for identity
        let identityQuery: [String: Any] = [
            kSecClass as String: kSecClassIdentity,
            kSecAttrLabel as String: tempCertLabel,
            kSecReturnRef as String: true,
        ]

        var identityItem: CFTypeRef?
        status = SecItemCopyMatching(identityQuery as CFDictionary, &identityItem)

        // Cleanup temp items
        SecItemDelete([kSecClass as String: kSecClassKey, kSecAttrApplicationTag as String: tempTag.data(using: .utf8)!] as CFDictionary)
        SecItemDelete([kSecClass as String: kSecClassCertificate, kSecAttrLabel as String: tempCertLabel] as CFDictionary)

        guard status == errSecSuccess, let identity = identityItem as! SecIdentity? else {
            throw CertificateAuthorityError.identityCreationFailed
        }

        return identity
    }

    // MARK: - Trust Management

    /// Check if the CA certificate is trusted by the system
    public var isTrusted: Bool {
        get async {
            guard let cert = caCertificate else { return false }

            var trustSettings: CFArray?
            let status = SecTrustSettingsCopyTrustSettings(cert, .user, &trustSettings)

            return status == errSecSuccess && trustSettings != nil
        }
    }

    /// Install CA certificate as trusted (requires user approval)
    public func installTrust() async throws {
        guard let cert = caCertificate else {
            throw CertificateAuthorityError.caNotInitialized
        }

        // Trust settings: kSecTrustSettingsResultTrustRoot = 1
        // This value means "trust this certificate as a root CA"
        let trustSettings: [String: Any] = [
            kSecTrustSettingsResult as String: NSNumber(value: 1) // kSecTrustSettingsResultTrustRoot
        ]

        let status = SecTrustSettingsSetTrustSettings(
            cert,
            .user,
            [trustSettings] as CFArray
        )

        guard status == errSecSuccess else {
            throw CertificateAuthorityError.trustSettingsFailed(status)
        }
    }

    /// Export CA certificate for manual trust installation
    public func exportCACertificate() async throws -> Data {
        guard let cert = caCertificate else {
            throw CertificateAuthorityError.caNotInitialized
        }
        return SecCertificateCopyData(cert) as Data
    }

    /// Export CA certificate as PEM string
    public func exportCACertificatePEM() async throws -> String {
        guard let cert = caCertificate else {
            throw CertificateAuthorityError.caNotInitialized
        }
        return try exportCertificateToPEM(cert)
    }
}
