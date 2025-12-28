//! Transparent TCP proxy for traffic interception
//!
//! This module implements a transparent proxy that:
//! 1. Accepts connections redirected by WinDivert
//! 2. Preserves the original destination using SO_ORIGINAL_DST
//! 3. Forwards traffic to the original destination
//! 4. Captures plaintext data before TLS (for Phase 4 TLS MITM)

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// Default proxy port
pub const DEFAULT_PROXY_PORT: u16 = 8443;

/// Proxy statistics
pub struct ProxyStats {
    pub connections_accepted: AtomicU64,
    pub connections_active: AtomicU64,
    pub bytes_forwarded: AtomicU64,
    pub errors: AtomicU64,
}

impl Default for ProxyStats {
    fn default() -> Self {
        Self {
            connections_accepted: AtomicU64::new(0),
            connections_active: AtomicU64::new(0),
            bytes_forwarded: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }
}

/// Original destination info for a redirected connection
#[derive(Debug, Clone)]
pub struct OriginalDestination {
    /// Original destination address
    pub dest_addr: SocketAddr,
    /// Process ID that initiated the connection
    pub pid: Option<u32>,
    /// Process name
    pub process_name: Option<String>,
}

/// Connection NAT table - maps local proxy connections to original destinations
pub struct NatTable {
    /// Maps (src_port) -> original destination
    /// When WinDivert redirects a connection to our proxy, we store the original dest
    entries: RwLock<HashMap<u16, OriginalDestination>>,
}

impl NatTable {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Add a NAT entry for a redirected connection
    pub async fn add_entry(&self, src_port: u16, original: OriginalDestination) {
        let mut entries = self.entries.write().await;
        debug!(
            "NAT: Adding entry for port {} -> {:?}",
            src_port, original.dest_addr
        );
        entries.insert(src_port, original);
    }

    /// Get and remove a NAT entry
    pub async fn get_entry(&self, src_port: u16) -> Option<OriginalDestination> {
        let mut entries = self.entries.write().await;
        entries.remove(&src_port)
    }

    /// Cleanup old entries (called periodically)
    pub async fn cleanup(&self) {
        // For now, just log the count
        let entries = self.entries.read().await;
        if !entries.is_empty() {
            debug!("NAT table has {} entries", entries.len());
        }
    }
}

impl Default for NatTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Callback for capturing data during proxying
pub type DataCallback = Arc<dyn Fn(&[u8], bool, &OriginalDestination) + Send + Sync>;

/// Transparent proxy server
pub struct TransparentProxy {
    /// Port to listen on
    port: u16,
    /// Running state
    running: Arc<AtomicBool>,
    /// Statistics
    stats: Arc<ProxyStats>,
    /// NAT table
    nat_table: Arc<NatTable>,
    /// Data capture callback (called with data, is_outbound, original_dest)
    data_callback: Option<DataCallback>,
}

impl TransparentProxy {
    /// Create a new transparent proxy
    pub fn new(port: u16) -> Self {
        Self {
            port,
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(ProxyStats::default()),
            nat_table: Arc::new(NatTable::new()),
            data_callback: None,
        }
    }

    /// Set the data capture callback
    pub fn set_data_callback(&mut self, callback: DataCallback) {
        self.data_callback = Some(callback);
    }

    /// Get the NAT table (for WinDivert to add entries)
    pub fn nat_table(&self) -> Arc<NatTable> {
        self.nat_table.clone()
    }

    /// Get statistics
    pub fn stats(&self) -> &ProxyStats {
        &self.stats
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Stop the proxy
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Start the proxy server
    pub async fn start(&self) -> Result<tokio::task::JoinHandle<()>> {
        if self.running.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Proxy already running"));
        }

        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .context(format!("Failed to bind proxy to {}", addr))?;

        self.running.store(true, Ordering::SeqCst);
        info!("Transparent proxy listening on {}", addr);

        let running = self.running.clone();
        let stats = self.stats.clone();
        let nat_table = self.nat_table.clone();
        let data_callback = self.data_callback.clone();

        let handle = tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, peer_addr)) => {
                                stats.connections_accepted.fetch_add(1, Ordering::Relaxed);
                                stats.connections_active.fetch_add(1, Ordering::Relaxed);

                                let stats = stats.clone();
                                let nat_table = nat_table.clone();
                                let data_callback = data_callback.clone();

                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(
                                        stream,
                                        peer_addr,
                                        nat_table,
                                        stats.clone(),
                                        data_callback,
                                    ).await {
                                        debug!("Connection error: {}", e);
                                        stats.errors.fetch_add(1, Ordering::Relaxed);
                                    }
                                    stats.connections_active.fetch_sub(1, Ordering::Relaxed);
                                });
                            }
                            Err(e) => {
                                error!("Accept error: {}", e);
                                stats.errors.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        // Check running flag
                    }
                }
            }
            info!("Transparent proxy stopped");
        });

        Ok(handle)
    }
}

/// Handle a single proxied connection
async fn handle_connection(
    mut client: TcpStream,
    peer_addr: SocketAddr,
    nat_table: Arc<NatTable>,
    stats: Arc<ProxyStats>,
    data_callback: Option<DataCallback>,
) -> Result<()> {
    // Get the original destination from NAT table
    let original = nat_table.get_entry(peer_addr.port()).await.ok_or_else(|| {
        anyhow::anyhow!(
            "No NAT entry for port {} - connection may not be from WinDivert",
            peer_addr.port()
        )
    })?;

    debug!(
        "Proxying connection from {} to {:?}",
        peer_addr, original.dest_addr
    );

    // Connect to the original destination
    let mut server = TcpStream::connect(original.dest_addr)
        .await
        .context(format!("Failed to connect to {:?}", original.dest_addr))?;

    debug!("Connected to original destination {:?}", original.dest_addr);

    // Bidirectional copy with optional data capture
    let (mut client_read, mut client_write) = client.split();
    let (mut server_read, mut server_write) = server.split();

    let stats_out = stats.clone();
    let stats_in = stats.clone();
    let original_out = original.clone();
    let original_in = original.clone();
    let callback_out = data_callback.clone();
    let callback_in = data_callback;

    // Client -> Server (outbound)
    let outbound = async move {
        let mut buf = [0u8; 65536];
        loop {
            match client_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    // Call data callback if set
                    if let Some(ref cb) = callback_out {
                        cb(&buf[..n], true, &original_out);
                    }
                    stats_out
                        .bytes_forwarded
                        .fetch_add(n as u64, Ordering::Relaxed);
                    if server_write.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    // Server -> Client (inbound)
    let inbound = async move {
        let mut buf = [0u8; 65536];
        loop {
            match server_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    // Call data callback if set
                    if let Some(ref cb) = callback_in {
                        cb(&buf[..n], false, &original_in);
                    }
                    stats_in
                        .bytes_forwarded
                        .fetch_add(n as u64, Ordering::Relaxed);
                    if client_write.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    // Wait for both directions to complete
    tokio::join!(outbound, inbound);

    debug!("Connection to {:?} closed", original.dest_addr);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nat_table() {
        let nat = NatTable::new();

        let original = OriginalDestination {
            dest_addr: "93.184.216.34:443".parse().unwrap(),
            pid: Some(1234),
            process_name: Some("python.exe".to_string()),
        };

        nat.add_entry(12345, original.clone()).await;

        let retrieved = nat.get_entry(12345).await.unwrap();
        assert_eq!(retrieved.dest_addr, original.dest_addr);
        assert_eq!(retrieved.pid, original.pid);

        // Entry should be removed after get
        assert!(nat.get_entry(12345).await.is_none());
    }
}
