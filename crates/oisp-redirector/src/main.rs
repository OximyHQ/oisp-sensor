//! OISP Windows Redirector
//!
//! This binary runs with elevated privileges and uses WinDivert to:
//! 1. Capture network packets destined for AI API endpoints
//! 2. Redirect them to the local TLS MITM proxy in oisp-sensor
//! 3. Provide process attribution via socket events
//!
//! Architecture:
//! - Runs as a separate elevated process (requires Administrator)
//! - Communicates with oisp-sensor via Named Pipe IPC
//! - Based on mitmproxy_rs Windows redirector (MIT licensed)
//!
//! Reference: https://github.com/mitmproxy/mitmproxy_rs

#[cfg(windows)]
mod windows_main;

#[cfg(windows)]
fn main() {
    if let Err(e) = windows_main::run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("The OISP Redirector only works on Windows.");
    eprintln!("On macOS, use the Network Extension instead.");
    eprintln!("On Linux, use the eBPF capture.");
    std::process::exit(1);
}
