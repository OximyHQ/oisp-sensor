//! TLS MITM (Man-in-the-Middle) proxy for decrypting SSL/TLS traffic
//!
//! This module implements:
//! 1. Certificate Authority (CA) generation and management
//! 2. Dynamic certificate generation per hostname (SNI)
//! 3. TLS server (client-facing) using rustls
//! 4. TLS client (server-facing) using rustls
//! 5. Bidirectional data forwarding with capture

use anyhow::{Context, Result};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType,
    ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose, SanType,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use std::collections::HashMap;
use std::fs;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tracing::{debug, info};

use super::proxy::OriginalDestination;

/// Default CA certificate validity in days
const CA_VALIDITY_DAYS: i64 = 3650; // 10 years

/// Default leaf certificate validity in days
const CERT_VALIDITY_DAYS: i64 = 365; // 1 year

/// Certificate Authority for MITM proxy
pub struct CertificateAuthority {
    /// CA certificate
    ca_cert: Certificate,
    /// CA key pair
    ca_keypair: KeyPair,
    /// PEM-encoded CA certificate (for distribution)
    ca_cert_pem: String,
    /// Cache of generated certificates per hostname
    cert_cache: RwLock<HashMap<String, Arc<CachedCertificate>>>,
}

/// Cached certificate for a hostname
pub struct CachedCertificate {
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>,
}

impl CertificateAuthority {
    /// Create a new CA or load existing one from disk
    pub fn new_or_load(ca_dir: &PathBuf) -> Result<Self> {
        let ca_cert_path = ca_dir.join("oisp-ca.crt");
        let ca_key_path = ca_dir.join("oisp-ca.key");

        if ca_cert_path.exists() && ca_key_path.exists() {
            info!("Loading existing CA from {:?}", ca_dir);
            Self::load_from_files(&ca_cert_path, &ca_key_path)
        } else {
            info!("Creating new CA in {:?}", ca_dir);
            Self::create_new(ca_dir)
        }
    }

    /// Create a new CA and save to disk
    fn create_new(ca_dir: &PathBuf) -> Result<Self> {
        // Ensure directory exists
        fs::create_dir_all(ca_dir)?;

        // Generate CA key pair
        let ca_keypair = KeyPair::generate()?;

        // Create CA certificate parameters
        let mut params = CertificateParams::default();

        // Distinguished Name
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "OISP Sensor CA");
        dn.push(DnType::OrganizationName, "OISP");
        dn.push(DnType::CountryName, "US");
        params.distinguished_name = dn;

        // Validity period
        let now = OffsetDateTime::now_utc();
        params.not_before = now;
        params.not_after = now + Duration::days(CA_VALIDITY_DAYS);

        // CA extensions
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];

        // Generate CA certificate
        let ca_cert = params.self_signed(&ca_keypair)?;

        // Get PEM representations
        let ca_cert_pem = ca_cert.pem();
        let ca_key_pem = ca_keypair.serialize_pem();

        // Save to files
        let ca_cert_path = ca_dir.join("oisp-ca.crt");
        let ca_key_path = ca_dir.join("oisp-ca.key");

        let mut cert_file = fs::File::create(&ca_cert_path)?;
        cert_file.write_all(ca_cert_pem.as_bytes())?;

        let mut key_file = fs::File::create(&ca_key_path)?;
        key_file.write_all(ca_key_pem.as_bytes())?;

        info!("Created new CA certificate: {:?}", ca_cert_path);
        info!(
            "To trust this CA, install {:?} as a trusted root",
            ca_cert_path
        );

        Ok(Self {
            ca_cert,
            ca_keypair,
            ca_cert_pem,
            cert_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Load existing CA from files
    fn load_from_files(cert_path: &PathBuf, key_path: &PathBuf) -> Result<Self> {
        let ca_cert_pem = fs::read_to_string(cert_path)?;
        let ca_key_pem = fs::read_to_string(key_path)?;

        let ca_keypair = KeyPair::from_pem(&ca_key_pem)?;

        // Parse the certificate to get params
        let ca_cert_params = CertificateParams::from_ca_cert_pem(&ca_cert_pem)?;
        let ca_cert = ca_cert_params.self_signed(&ca_keypair)?;

        info!("Loaded existing CA certificate");

        Ok(Self {
            ca_cert,
            ca_keypair,
            ca_cert_pem,
            cert_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Get or generate a certificate for a hostname
    pub async fn get_cert_for_host(&self, hostname: &str) -> Result<Arc<CachedCertificate>> {
        // Check cache first
        {
            let cache = self.cert_cache.read().await;
            if let Some(cached) = cache.get(hostname) {
                debug!("Using cached certificate for {}", hostname);
                return Ok(cached.clone());
            }
        }

        // Generate new certificate
        debug!("Generating certificate for {}", hostname);
        let cert = self.generate_cert_for_host(hostname)?;
        let cached = Arc::new(cert);

        // Store in cache
        {
            let mut cache = self.cert_cache.write().await;
            cache.insert(hostname.to_string(), cached.clone());
        }

        Ok(cached)
    }

    /// Generate a certificate for a specific hostname
    fn generate_cert_for_host(&self, hostname: &str) -> Result<CachedCertificate> {
        // Generate leaf key pair
        let leaf_keypair = KeyPair::generate()?;

        // Create certificate parameters
        let mut params = CertificateParams::default();

        // Distinguished Name
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, hostname);
        params.distinguished_name = dn;

        // Subject Alternative Names
        params.subject_alt_names = vec![SanType::DnsName(hostname.try_into()?)];

        // Validity period
        let now = OffsetDateTime::now_utc();
        params.not_before = now;
        params.not_after = now + Duration::days(CERT_VALIDITY_DAYS);

        // Leaf certificate extensions
        params.is_ca = IsCa::NoCa;
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
            ExtendedKeyUsagePurpose::ClientAuth,
        ];

        // Sign with CA
        let leaf_cert = params.signed_by(&leaf_keypair, &self.ca_cert, &self.ca_keypair)?;

        Ok(CachedCertificate {
            cert_der: leaf_cert.der().to_vec(),
            key_der: leaf_keypair.serialize_der(),
        })
    }

    /// Get the CA certificate in PEM format (for installation)
    pub fn ca_cert_pem(&self) -> &str {
        &self.ca_cert_pem
    }
}

/// TLS MITM handler for a single connection
pub struct TlsMitmHandler {
    /// Certificate Authority
    ca: Arc<CertificateAuthority>,
    /// Root certificate store for server connections
    root_store: Arc<RootCertStore>,
}

impl TlsMitmHandler {
    /// Create a new TLS MITM handler
    pub fn new(ca: Arc<CertificateAuthority>) -> Self {
        // Load system root certificates
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        Self {
            ca,
            root_store: Arc::new(root_store),
        }
    }

    /// Handle a TLS connection with MITM
    ///
    /// Returns the decrypted request and response data
    pub async fn handle_connection(
        &self,
        client_stream: TcpStream,
        original: &OriginalDestination,
        data_callback: Option<&super::proxy::DataCallback>,
    ) -> Result<()> {
        let hostname = match &original.dest_addr {
            std::net::SocketAddr::V4(addr) => addr.ip().to_string(),
            std::net::SocketAddr::V6(addr) => addr.ip().to_string(),
        };

        debug!("Starting TLS MITM for {}", hostname);

        // Get certificate for this host
        let cert = self.ca.get_cert_for_host(&hostname).await?;

        // Create server config for client connection
        let server_config = self.create_server_config(&cert)?;
        let acceptor = TlsAcceptor::from(Arc::new(server_config));

        // Accept TLS from client
        let tls_client = acceptor.accept(client_stream).await?;
        debug!("TLS handshake with client complete for {}", hostname);

        // Connect to real server
        let server_stream = TcpStream::connect(original.dest_addr).await?;

        // Create client config for server connection
        let client_config = self.create_client_config()?;
        let connector = TlsConnector::from(Arc::new(client_config));

        // Parse ServerName
        let server_name: ServerName<'_> = hostname
            .clone()
            .try_into()
            .unwrap_or_else(|_| ServerName::try_from("localhost").unwrap());

        // Connect TLS to server
        let tls_server = connector
            .connect(server_name.to_owned(), server_stream)
            .await?;
        debug!("TLS handshake with server complete for {}", hostname);

        // Bidirectional forwarding with capture
        let (mut client_read, mut client_write) = tokio::io::split(tls_client);
        let (mut server_read, mut server_write) = tokio::io::split(tls_server);

        let original_clone = original.clone();
        let callback_out = data_callback.cloned();
        let callback_in = data_callback.cloned();

        // Client -> Server (request)
        let outbound = async move {
            let mut buf = [0u8; 65536];
            loop {
                match client_read.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        // Capture request data
                        if let Some(ref cb) = callback_out {
                            cb(&buf[..n], true, &original_clone);
                        }
                        if server_write.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        };

        let original_clone2 = original.clone();

        // Server -> Client (response)
        let inbound = async move {
            let mut buf = [0u8; 65536];
            loop {
                match server_read.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        // Capture response data
                        if let Some(ref cb) = callback_in {
                            cb(&buf[..n], false, &original_clone2);
                        }
                        if client_write.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        };

        // Wait for both directions
        tokio::join!(outbound, inbound);

        debug!("TLS MITM connection closed for {}", hostname);
        Ok(())
    }

    /// Create server config for accepting client connections
    fn create_server_config(&self, cert: &CachedCertificate) -> Result<ServerConfig> {
        let cert_chain = vec![CertificateDer::from(cert.cert_der.clone())];
        let private_key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(cert.key_der.clone()));

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .context("Failed to create server config")?;

        Ok(config)
    }

    /// Create client config for connecting to real servers
    fn create_client_config(&self) -> Result<ClientConfig> {
        let config = ClientConfig::builder()
            .with_root_certificates((*self.root_store).clone())
            .with_no_client_auth();

        Ok(config)
    }
}

/// Determine the CA directory based on platform
pub fn get_ca_dir() -> PathBuf {
    #[cfg(windows)]
    {
        // Use %LOCALAPPDATA%\OISP\
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local_app_data).join("OISP");
        }
    }

    // Fallback to current directory
    PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_ca_creation() {
        let ca_dir = temp_dir().join("oisp-test-ca");
        let _ = fs::remove_dir_all(&ca_dir); // Clean up any previous test

        let ca = CertificateAuthority::new_or_load(&ca_dir).unwrap();
        assert!(ca.ca_cert_pem.contains("BEGIN CERTIFICATE"));

        // Cleanup
        let _ = fs::remove_dir_all(&ca_dir);
    }

    #[tokio::test]
    async fn test_cert_generation() {
        let ca_dir = temp_dir().join("oisp-test-ca-2");
        let _ = fs::remove_dir_all(&ca_dir);

        let ca = CertificateAuthority::new_or_load(&ca_dir).unwrap();
        let cert = ca.get_cert_for_host("api.openai.com").await.unwrap();

        assert!(!cert.cert_der.is_empty());
        assert!(!cert.key_der.is_empty());

        // Second call should use cache
        let cert2 = ca.get_cert_for_host("api.openai.com").await.unwrap();
        assert_eq!(cert.cert_der, cert2.cert_der);

        // Cleanup
        let _ = fs::remove_dir_all(&ca_dir);
    }
}
