//! WinDivert capture wrapper
//!
//! Provides a safe Rust interface to WinDivert for packet capture.
//! Based on patterns from mitmproxy_rs.
//!
//! This module only fully functions on Windows.

use anyhow::{Context, Result};
use std::net::SocketAddr;
use tracing::{debug, trace};

#[cfg(windows)]
use internet_packet::InternetPacket;
#[cfg(windows)]
use windivert::address::WinDivertAddress;
#[cfg(windows)]
use windivert::prelude::*;

/// Information about a captured packet
#[derive(Debug, Clone)]
pub struct PacketInfo {
    /// Raw packet data
    pub data: Vec<u8>,

    /// Parsed packet information (if TCP)
    pub tcp_info: Option<TcpPacketInfo>,

    /// Is outbound traffic
    pub outbound: bool,

    /// Interface index
    pub interface_index: u32,

    /// Subinterface index
    pub subinterface_index: u32,
}

/// Parsed TCP packet information
#[derive(Debug, Clone)]
pub struct TcpPacketInfo {
    /// Source address (IP:port)
    pub src_addr: SocketAddr,

    /// Destination address (IP:port)
    pub dst_addr: SocketAddr,

    /// Source port (convenience accessor)
    pub src_port: u16,

    /// Destination port (convenience accessor)
    pub dst_port: u16,

    /// TCP flags
    pub flags: TcpFlags,

    /// Sequence number
    pub seq: u32,

    /// Acknowledgment number
    pub ack: u32,

    /// Payload length
    pub payload_len: usize,
}

/// TCP flags
#[derive(Debug, Clone, Default)]
pub struct TcpFlags {
    pub syn: bool,
    pub ack: bool,
    pub fin: bool,
    pub rst: bool,
    pub psh: bool,
}

impl TcpFlags {
    pub fn from_flags(flags: u8) -> Self {
        Self {
            fin: flags & 0x01 != 0,
            syn: flags & 0x02 != 0,
            rst: flags & 0x04 != 0,
            psh: flags & 0x08 != 0,
            ack: flags & 0x10 != 0,
        }
    }

    pub fn is_handshake_start(&self) -> bool {
        self.syn && !self.ack
    }

    pub fn is_handshake_ack(&self) -> bool {
        self.syn && self.ack
    }

    pub fn is_connection_end(&self) -> bool {
        self.fin || self.rst
    }
}

// =============================================================================
// WINDOWS IMPLEMENTATION
// =============================================================================

/// WinDivert capture wrapper
#[cfg(windows)]
pub struct WinDivertCapture {
    /// WinDivert handle for network layer
    handle: WinDivert<NetworkLayer>,

    /// Whether we're in capture-only mode (passthrough)
    #[allow(dead_code)]
    capture_only: bool,

    /// Buffer for receiving packets
    recv_buffer: Vec<u8>,
}

#[cfg(windows)]
impl WinDivertCapture {
    /// Create a new WinDivert capture instance
    ///
    /// # Arguments
    /// * `filter` - WinDivert filter expression
    /// * `capture_only` - If true, packets are re-injected unchanged
    pub fn new(filter: &str, capture_only: bool) -> Result<Self> {
        debug!("Opening WinDivert with filter: {}", filter);

        // Open WinDivert handle with default priority (0) and no special flags
        let handle = WinDivert::network(filter, 0, WinDivertFlags::new())
            .context("Failed to open WinDivert handle. Ensure you have Administrator privileges and WinDivert is installed.")?;

        debug!("WinDivert handle opened successfully");

        Ok(Self {
            handle,
            capture_only,
            recv_buffer: vec![0u8; 65535], // Max packet size
        })
    }

    /// Receive a packet from WinDivert
    ///
    /// Returns `Ok(Some(PacketInfo))` if a packet was received,
    /// `Ok(None)` on timeout, or `Err` on error.
    pub fn recv_packet(&mut self) -> Result<Option<PacketInfo>> {
        // Receive packet
        match self.handle.recv(Some(&mut self.recv_buffer)) {
            Ok(packet) => {
                let data = packet.data.to_vec();
                let address = packet.address;

                // Extract address info
                let outbound = address.outbound();
                let interface_index = address.interface_index();
                let subinterface_index = address.subinterface_index();

                // Try to parse as TCP
                let tcp_info = self.parse_tcp_packet(&data);

                if let Some(ref info) = tcp_info {
                    trace!(
                        "Packet: {} -> {} [{}] {}",
                        info.src_addr,
                        info.dst_addr,
                        Self::flags_to_string(&info.flags),
                        if outbound { "OUT" } else { "IN" }
                    );
                }

                Ok(Some(PacketInfo {
                    data,
                    tcp_info,
                    outbound,
                    interface_index,
                    subinterface_index,
                }))
            }
            Err(e) => {
                // Check if it's a timeout (which is expected)
                let error_str = format!("{:?}", e);
                if error_str.contains("timeout")
                    || error_str.contains("Timeout")
                    || error_str.contains("timed out")
                {
                    Ok(None)
                } else {
                    Err(anyhow::anyhow!("WinDivert recv error: {:?}", e))
                }
            }
        }
    }

    /// Re-inject a packet
    pub fn send_packet(&self, packet: &PacketInfo) -> Result<()> {
        // Reconstruct the address from packet info
        let mut address = WinDivertAddress::<NetworkLayer>::default();
        address.set_outbound(packet.outbound);
        address.set_interface_index(packet.interface_index);
        address.set_subinterface_index(packet.subinterface_index);

        let windivert_packet = WinDivertPacket {
            address,
            data: packet.data.as_slice().into(),
        };

        self.handle
            .send(&windivert_packet)
            .context("Failed to re-inject packet")?;

        Ok(())
    }

    /// Parse TCP packet information
    fn parse_tcp_packet(&self, data: &[u8]) -> Option<TcpPacketInfo> {
        use internet_packet::TransportProtocol;

        let packet = InternetPacket::try_from(data.to_vec()).ok()?;

        // Check if this is a TCP packet
        if packet.protocol() != TransportProtocol::Tcp {
            return None;
        }

        let src_port = packet.src_port();
        let dst_port = packet.dst_port();
        let src_ip = packet.src_ip();
        let dst_ip = packet.dst_ip();

        let src_addr = SocketAddr::new(src_ip, src_port);
        let dst_addr = SocketAddr::new(dst_ip, dst_port);

        Some(TcpPacketInfo {
            src_addr,
            dst_addr,
            src_port,
            dst_port,
            flags: TcpFlags::from_flags(packet.tcp_flags()),
            seq: packet.tcp_sequence_number(),
            ack: packet.tcp_acknowledgement_number(),
            payload_len: packet.payload().len(),
        })
    }

    /// Convert TCP flags to string for logging
    fn flags_to_string(flags: &TcpFlags) -> String {
        let mut s = String::new();
        if flags.syn {
            s.push('S');
        }
        if flags.ack {
            s.push('A');
        }
        if flags.fin {
            s.push('F');
        }
        if flags.rst {
            s.push('R');
        }
        if flags.psh {
            s.push('P');
        }
        if s.is_empty() {
            s.push('-');
        }
        s
    }
}

#[cfg(windows)]
impl Drop for WinDivertCapture {
    fn drop(&mut self) {
        debug!("Closing WinDivert handle");
        // WinDivert handle is closed automatically when dropped
    }
}

// =============================================================================
// NON-WINDOWS STUB (for cross-compilation)
// =============================================================================

/// WinDivert capture wrapper (stub for non-Windows)
#[cfg(not(windows))]
pub struct WinDivertCapture;

#[cfg(not(windows))]
impl WinDivertCapture {
    /// Create a new WinDivert capture instance (stub)
    pub fn new(_filter: &str, _capture_only: bool) -> Result<Self> {
        Err(anyhow::anyhow!("WinDivert is only available on Windows"))
    }

    /// Receive a packet from WinDivert (stub)
    pub fn recv_packet(&mut self) -> Result<Option<PacketInfo>> {
        Err(anyhow::anyhow!("WinDivert is only available on Windows"))
    }

    /// Re-inject a packet (stub)
    pub fn send_packet(&self, _packet: &PacketInfo) -> Result<()> {
        Err(anyhow::anyhow!("WinDivert is only available on Windows"))
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_flags() {
        let flags = TcpFlags::from_flags(0x02); // SYN
        assert!(flags.syn);
        assert!(!flags.ack);
        assert!(flags.is_handshake_start());

        let flags = TcpFlags::from_flags(0x12); // SYN+ACK
        assert!(flags.syn);
        assert!(flags.ack);
        assert!(flags.is_handshake_ack());

        let flags = TcpFlags::from_flags(0x11); // FIN+ACK
        assert!(flags.fin);
        assert!(flags.ack);
        assert!(flags.is_connection_end());
    }
}
