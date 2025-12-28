//! Windows main implementation for OISP Redirector
//!
//! This module implements packet capture and redirection using WinDivert.
//! It runs with elevated privileges and communicates with oisp-sensor via Named Pipes.
//!
//! This module only fully functions on Windows.

use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

mod ai_filter;
mod connection;
mod ipc;
mod packet_rewrite;
mod proxy;
mod tls_mitm;
mod windivert_capture;

use ai_filter::AiEndpointFilter;
use connection::ConnectionTracker;
use ipc::IpcClient;
use packet_rewrite::rewrite_ipv4_dst;
use proxy::TransparentProxy;
use std::net::Ipv4Addr;
use tls_mitm::{get_ca_dir, CertificateAuthority, TlsMitmHandler};
use windivert_capture::WinDivertCapture;

/// Configuration for the redirector
#[derive(Debug, Clone)]
pub struct RedirectorConfig {
    /// Named pipe path for IPC with oisp-sensor
    pub pipe_path: String,

    /// Local proxy port for redirected traffic
    pub proxy_port: u16,

    /// Filter expression for WinDivert (which ports to intercept)
    pub filter_ports: Vec<u16>,

    /// Whether to only capture (no redirection)
    pub capture_only: bool,

    /// Whether to enable TLS MITM (decryption)
    pub tls_mitm: bool,

    /// Whether to filter for AI endpoints only
    pub ai_filter: bool,

    /// Whether to log packet details
    pub verbose: bool,
}

impl Default for RedirectorConfig {
    fn default() -> Self {
        Self {
            pipe_path: r"\\.\pipe\oisp-capture".to_string(),
            proxy_port: 8443,
            // Default: HTTPS ports commonly used by AI APIs
            filter_ports: vec![443],
            capture_only: true, // Start with capture-only mode
            tls_mitm: false,    // TLS MITM disabled by default
            ai_filter: true,    // AI filtering enabled by default
            verbose: false,
        }
    }
}

/// Main entry point for Windows redirector
pub fn run() -> Result<()> {
    // Initialize logging
    init_logging();

    info!("OISP Redirector starting...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Check if running as Administrator (Windows only)
    #[cfg(windows)]
    {
        if !is_elevated()? {
            error!("This application requires Administrator privileges");
            error!("Please run as Administrator (right-click -> Run as administrator)");
            return Err(anyhow::anyhow!("Administrator privileges required"));
        }
        info!("Running with Administrator privileges");
    }

    #[cfg(not(windows))]
    {
        error!("This application only works on Windows");
        return Err(anyhow::anyhow!("Windows required"));
    }

    // Parse command line arguments
    let config = parse_args()?;

    // Create shutdown signal
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    // Set up Ctrl+C handler
    ctrlc::set_handler(move || {
        info!("Received shutdown signal");
        running_clone.store(false, Ordering::SeqCst);
    })
    .context("Failed to set Ctrl+C handler")?;

    // Run the capture
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to create Tokio runtime")?;

    runtime.block_on(async { run_capture(config, running).await })
}

/// Initialize logging
fn init_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,oisp_redirector=debug"));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}

/// Check if running with elevated privileges (Windows only)
#[cfg(windows)]
fn is_elevated() -> Result<bool> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token_handle = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle)
            .context("Failed to open process token")?;

        let mut elevation = TOKEN_ELEVATION::default();
        let mut return_length = 0u32;

        GetTokenInformation(
            token_handle,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        )
        .context("Failed to get token information")?;

        Ok(elevation.TokenIsElevated != 0)
    }
}

/// Parse command line arguments
fn parse_args() -> Result<RedirectorConfig> {
    let args: Vec<String> = std::env::args().collect();
    let mut config = RedirectorConfig::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--capture-only" | "-c" => {
                config.capture_only = true;
            }
            "--redirect" | "-r" => {
                config.capture_only = false;
            }
            "--tls-mitm" | "-t" => {
                config.tls_mitm = true;
                config.capture_only = false; // MITM requires redirection
            }
            "--no-ai-filter" => {
                config.ai_filter = false;
            }
            "--all-traffic" | "-a" => {
                config.ai_filter = false; // Intercept all HTTPS, not just AI
            }
            "--verbose" | "-v" => {
                config.verbose = true;
            }
            "--proxy-port" | "-p" => {
                i += 1;
                if i < args.len() {
                    config.proxy_port = args[i].parse().context("Invalid proxy port")?;
                }
            }
            "--pipe" => {
                i += 1;
                if i < args.len() {
                    config.pipe_path = args[i].clone();
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                warn!("Unknown argument: {}", other);
            }
        }
        i += 1;
    }

    Ok(config)
}

/// Print help message
fn print_help() {
    println!("OISP Redirector - Windows packet capture and redirection");
    println!();
    println!("Usage: oisp-redirector.exe [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -c, --capture-only    Only capture packets, don't redirect (default)");
    println!("  -r, --redirect        Enable traffic redirection to proxy (passthrough)");
    println!("  -t, --tls-mitm        Enable TLS MITM decryption (requires CA trust)");
    println!("  -a, --all-traffic     Intercept all HTTPS traffic (not just AI endpoints)");
    println!("  --no-ai-filter        Disable AI endpoint filtering");
    println!("  -v, --verbose         Enable verbose packet logging");
    println!("  -p, --proxy-port      Local proxy port (default: 8443)");
    println!("  --pipe                Named pipe path (default: \\\\.\\pipe\\oisp-capture)");
    println!("  -h, --help            Show this help message");
    println!();
    println!("AI Filtering:");
    println!("  By default, only traffic to known AI API endpoints is intercepted.");
    println!("  Supported providers: OpenAI, Anthropic, Google, Azure OpenAI, AWS Bedrock,");
    println!("  Groq, Mistral, Cohere, Together, Fireworks, Replicate, Perplexity, and more.");
    println!();
    println!("TLS MITM Mode:");
    println!("  When using --tls-mitm, OISP will create a CA certificate that must be");
    println!("  trusted by your system. The CA will be stored in %LOCALAPPDATA%\\OISP\\");
    println!();
    println!("This application requires Administrator privileges.");
}

/// Run the main capture loop
async fn run_capture(config: RedirectorConfig, running: Arc<AtomicBool>) -> Result<()> {
    info!("Configuration:");
    info!("  Capture only: {}", config.capture_only);
    info!("  TLS MITM: {}", config.tls_mitm);
    info!("  AI filter: {}", config.ai_filter);
    info!("  Proxy port: {}", config.proxy_port);
    info!("  Filter ports: {:?}", config.filter_ports);
    info!("  Pipe path: {}", config.pipe_path);

    // Initialize AI endpoint filter (currently unused, will be used in future for filtering)
    let _ai_filter = if config.ai_filter {
        match AiEndpointFilter::new() {
            Ok(filter) => {
                let (domains, patterns) = filter.stats();
                info!(
                    "AI filter loaded: {} domains, {} patterns",
                    domains, patterns
                );
                info!("Supported providers: {:?}", filter.providers());
                Some(filter)
            }
            Err(e) => {
                error!("Failed to load AI filter: {}", e);
                warn!("Continuing without AI filtering");
                None
            }
        }
    } else {
        info!("AI filtering disabled - intercepting all traffic to filter ports");
        None
    };

    // Initialize Certificate Authority if TLS MITM is enabled
    let ca = if config.tls_mitm {
        let ca_dir = get_ca_dir();
        info!("CA directory: {:?}", ca_dir);
        match CertificateAuthority::new_or_load(&ca_dir) {
            Ok(ca) => {
                info!("CA initialized successfully");
                info!("CA certificate location: {:?}/oisp-ca.crt", ca_dir);
                info!("IMPORTANT: You must trust this CA for TLS MITM to work!");
                Some(Arc::new(ca))
            }
            Err(e) => {
                error!("Failed to initialize CA: {}", e);
                return Err(e);
            }
        }
    } else {
        None
    };

    // Create TLS MITM handler if enabled
    let _tls_handler = ca
        .as_ref()
        .map(|ca| Arc::new(TlsMitmHandler::new(ca.clone())));

    // Initialize connection tracker
    let mut tracker = ConnectionTracker::new();

    // Try to connect to IPC
    let mut ipc_client = match IpcClient::connect(&config.pipe_path).await {
        Ok(client) => {
            if client.is_connected() {
                info!("Connected to oisp-sensor via Named Pipe");
            } else {
                info!("Named Pipe not available yet, will retry on send");
            }
            Some(client)
        }
        Err(e) => {
            warn!("Could not connect to oisp-sensor: {}", e);
            warn!("Running in standalone mode (logging to console only)");
            None
        }
    };

    // Start transparent proxy if not in capture-only mode
    let proxy = if !config.capture_only {
        let proxy = TransparentProxy::new(config.proxy_port);

        // Set up data callback to send captured data to IPC
        // This will be used in Phase 4 for TLS-terminated traffic

        match proxy.start().await {
            Ok(handle) => {
                info!("Transparent proxy started on port {}", config.proxy_port);
                Some((proxy, handle))
            }
            Err(e) => {
                error!("Failed to start proxy: {}", e);
                return Err(e);
            }
        }
    } else {
        info!("Running in capture-only mode (no proxy)");
        None
    };

    // Build WinDivert filter
    let filter = build_filter(&config.filter_ports, config.capture_only, config.proxy_port);
    info!("WinDivert filter: {}", filter);

    // Initialize WinDivert capture
    let mut capture = WinDivertCapture::new(&filter, config.capture_only)
        .context("Failed to initialize WinDivert capture")?;

    info!("WinDivert capture initialized successfully");
    info!(
        "Listening for TCP connections on ports: {:?}",
        config.filter_ports
    );
    info!("Press Ctrl+C to stop");

    // Main capture loop
    let mut packet_count = 0u64;
    let mut last_stats_time = std::time::Instant::now();

    while running.load(Ordering::SeqCst) {
        match capture.recv_packet() {
            Ok(Some(mut packet_info)) => {
                packet_count += 1;

                // Track connection
                if let Some(conn_info) = tracker.process_packet(&packet_info) {
                    if config.verbose {
                        debug!(
                            "Connection: {} -> {} (PID: {:?})",
                            conn_info.local_addr, conn_info.remote_addr, conn_info.pid
                        );
                    }

                    // Send connection event to IPC if connected
                    if let Some(ref mut client) = ipc_client {
                        if let Err(e) = client.send_connection_event(&conn_info).await {
                            debug!("Failed to send connection event: {}", e);
                        }
                    }

                    // If proxy mode is enabled and this is an outbound SYN to our target ports,
                    // add NAT entry and rewrite destination to proxy
                    if let Some((ref proxy_inst, _)) = proxy {
                        if let Some(ref tcp_info) = packet_info.tcp_info {
                            // Check if this is a new outbound connection to target port
                            if packet_info.outbound
                                && tcp_info.flags.syn
                                && !tcp_info.flags.ack
                                && config.filter_ports.contains(&tcp_info.dst_port)
                            {
                                // Add NAT entry
                                let original_dest = proxy::OriginalDestination {
                                    dest_addr: conn_info.remote_addr,
                                    pid: conn_info.pid,
                                    process_name: conn_info.process_name.clone(),
                                };
                                proxy_inst
                                    .nat_table()
                                    .add_entry(tcp_info.src_port, original_dest)
                                    .await;

                                // Rewrite packet destination to proxy
                                if rewrite_ipv4_dst(
                                    &mut packet_info.data,
                                    Ipv4Addr::new(127, 0, 0, 1),
                                    config.proxy_port,
                                ) {
                                    debug!(
                                        "Redirected {}:{} -> localhost:{}",
                                        tcp_info.dst_addr, tcp_info.dst_port, config.proxy_port
                                    );
                                } else {
                                    warn!("Failed to rewrite packet for redirection");
                                }
                            }
                        }
                    }
                }

                // Re-inject packet (possibly modified)
                capture.send_packet(&packet_info)?;

                // Print stats every 5 seconds
                if last_stats_time.elapsed() > std::time::Duration::from_secs(5) {
                    let proxy_stats = proxy
                        .as_ref()
                        .map(|(p, _)| {
                            format!(
                                ", proxy: {} conns ({} bytes)",
                                p.stats().connections_accepted.load(Ordering::Relaxed),
                                p.stats().bytes_forwarded.load(Ordering::Relaxed)
                            )
                        })
                        .unwrap_or_default();

                    let ipc_stats = ipc_client
                        .as_ref()
                        .map(|c| {
                            format!(
                                ", IPC: {} events sent",
                                c.stats().events_sent.load(Ordering::Relaxed)
                            )
                        })
                        .unwrap_or_default();

                    info!(
                        "Stats: {} packets, {} active conns{}{}",
                        packet_count,
                        tracker.active_connections(),
                        proxy_stats,
                        ipc_stats
                    );
                    last_stats_time = std::time::Instant::now();
                }
            }
            Ok(None) => {
                // Timeout, check if we should continue
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
            Err(e) => {
                error!("Packet receive error: {}", e);
                if !running.load(Ordering::SeqCst) {
                    break;
                }
            }
        }
    }

    info!("Shutting down...");
    info!("Total packets captured: {}", packet_count);

    // Stop proxy if running
    if let Some((proxy_inst, handle)) = proxy {
        proxy_inst.stop();
        handle.abort();
        info!("Proxy stopped");
    }

    // Cleanup
    drop(capture);

    info!("Redirector stopped");
    Ok(())
}

/// Build WinDivert filter expression for specific ports
fn build_filter(ports: &[u16], capture_only: bool, proxy_port: u16) -> String {
    if ports.is_empty() {
        // Default: all TCP traffic
        return "tcp".to_string();
    }

    let port_filters: Vec<String> = ports
        .iter()
        .map(|p| format!("tcp.DstPort == {} or tcp.SrcPort == {}", p, p))
        .collect();

    let base_filter = format!("({})", port_filters.join(" or "));

    if capture_only {
        // In capture-only mode, just filter the target ports
        base_filter
    } else {
        // In redirect mode, also capture traffic to/from our proxy port
        // but exclude loopback to avoid infinite loop
        format!(
            "({}) and not (ip.DstAddr == 127.0.0.1 and tcp.DstPort == {})",
            base_filter, proxy_port
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_filter_capture_only() {
        assert_eq!(
            build_filter(&[443], true, 8443),
            "(tcp.DstPort == 443 or tcp.SrcPort == 443)"
        );
        assert_eq!(
            build_filter(&[443, 8443], true, 8443),
            "(tcp.DstPort == 443 or tcp.SrcPort == 443 or tcp.DstPort == 8443 or tcp.SrcPort == 8443)"
        );
        assert_eq!(build_filter(&[], true, 8443), "tcp");
    }

    #[test]
    fn test_build_filter_redirect() {
        let filter = build_filter(&[443], false, 8443);
        assert!(filter.contains("tcp.DstPort == 443"));
        assert!(filter.contains("127.0.0.1"));
        assert!(filter.contains("8443"));
    }

    #[test]
    fn test_default_config() {
        let config = RedirectorConfig::default();
        assert_eq!(config.proxy_port, 8443);
        assert!(config.capture_only);
        assert_eq!(config.filter_ports, vec![443]);
    }
}
