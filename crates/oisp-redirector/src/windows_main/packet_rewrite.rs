//! Packet rewriting for traffic redirection
//!
//! This module handles modifying captured packets to redirect traffic
//! to our local proxy. It modifies destination addresses and recalculates
//! TCP/IP checksums.

use std::net::{Ipv4Addr, SocketAddr};
use tracing::debug;

/// Represents a rewrite target for traffic redirection
#[derive(Debug, Clone)]
pub struct RewriteTarget {
    /// Original destination address
    pub original_dst: SocketAddr,
    /// New destination (our proxy)
    pub new_dst: SocketAddr,
}

/// Rewrite an IPv4 packet's destination to redirect to proxy
///
/// This function modifies the packet in-place:
/// 1. Changes destination IP to 127.0.0.1
/// 2. Changes destination port to proxy port
/// 3. Recalculates IP header checksum
/// 4. Recalculates TCP checksum
///
/// Returns true if rewrite was successful
#[cfg(target_os = "windows")]
pub fn rewrite_ipv4_dst(packet_data: &mut [u8], new_dst_addr: Ipv4Addr, new_dst_port: u16) -> bool {
    // Minimum size check: IP header (20) + TCP header (20)
    if packet_data.len() < 40 {
        return false;
    }

    // Parse IP header
    let ip_version = (packet_data[0] >> 4) & 0x0F;
    if ip_version != 4 {
        return false; // Only IPv4 supported
    }

    let ip_header_len = ((packet_data[0] & 0x0F) as usize) * 4;
    if ip_header_len < 20 || packet_data.len() < ip_header_len {
        return false;
    }

    let protocol = packet_data[9];
    if protocol != 6 {
        return false; // Only TCP supported
    }

    // Store original destination IP (bytes 16-19)
    let _original_dst_ip = [
        packet_data[16],
        packet_data[17],
        packet_data[18],
        packet_data[19],
    ];

    // Modify destination IP
    let new_addr_octets = new_dst_addr.octets();
    packet_data[16] = new_addr_octets[0];
    packet_data[17] = new_addr_octets[1];
    packet_data[18] = new_addr_octets[2];
    packet_data[19] = new_addr_octets[3];

    // Zero out IP checksum before recalculating (bytes 10-11)
    packet_data[10] = 0;
    packet_data[11] = 0;

    // Calculate new IP checksum
    let ip_checksum = calculate_ip_checksum(&packet_data[..ip_header_len]);
    packet_data[10] = (ip_checksum >> 8) as u8;
    packet_data[11] = (ip_checksum & 0xFF) as u8;

    // TCP header starts after IP header
    let tcp_start = ip_header_len;
    if packet_data.len() < tcp_start + 20 {
        return false;
    }

    // Modify destination port (bytes 2-3 of TCP header)
    packet_data[tcp_start + 2] = (new_dst_port >> 8) as u8;
    packet_data[tcp_start + 3] = (new_dst_port & 0xFF) as u8;

    // Zero out TCP checksum before recalculating (bytes 16-17 of TCP header)
    packet_data[tcp_start + 16] = 0;
    packet_data[tcp_start + 17] = 0;

    // Calculate new TCP checksum (includes pseudo-header)
    let tcp_len = packet_data.len() - tcp_start;
    let tcp_checksum = calculate_tcp_checksum(
        &packet_data[..ip_header_len],
        &packet_data[tcp_start..],
        tcp_len,
    );
    packet_data[tcp_start + 16] = (tcp_checksum >> 8) as u8;
    packet_data[tcp_start + 17] = (tcp_checksum & 0xFF) as u8;

    debug!("Rewrote packet: dst -> {}:{}", new_dst_addr, new_dst_port);

    true
}

/// Non-Windows stub
#[cfg(not(target_os = "windows"))]
pub fn rewrite_ipv4_dst(
    _packet_data: &mut [u8],
    _new_dst_addr: Ipv4Addr,
    _new_dst_port: u16,
) -> bool {
    false
}

/// Rewrite source address for return packets (from proxy back to client)
#[cfg(target_os = "windows")]
pub fn rewrite_ipv4_src(packet_data: &mut [u8], new_src_addr: Ipv4Addr, new_src_port: u16) -> bool {
    // Minimum size check
    if packet_data.len() < 40 {
        return false;
    }

    let ip_version = (packet_data[0] >> 4) & 0x0F;
    if ip_version != 4 {
        return false;
    }

    let ip_header_len = ((packet_data[0] & 0x0F) as usize) * 4;
    if ip_header_len < 20 || packet_data.len() < ip_header_len {
        return false;
    }

    let protocol = packet_data[9];
    if protocol != 6 {
        return false;
    }

    // Modify source IP (bytes 12-15)
    let new_addr_octets = new_src_addr.octets();
    packet_data[12] = new_addr_octets[0];
    packet_data[13] = new_addr_octets[1];
    packet_data[14] = new_addr_octets[2];
    packet_data[15] = new_addr_octets[3];

    // Recalculate IP checksum
    packet_data[10] = 0;
    packet_data[11] = 0;
    let ip_checksum = calculate_ip_checksum(&packet_data[..ip_header_len]);
    packet_data[10] = (ip_checksum >> 8) as u8;
    packet_data[11] = (ip_checksum & 0xFF) as u8;

    // Modify source port (bytes 0-1 of TCP header)
    let tcp_start = ip_header_len;
    packet_data[tcp_start] = (new_src_port >> 8) as u8;
    packet_data[tcp_start + 1] = (new_src_port & 0xFF) as u8;

    // Recalculate TCP checksum
    packet_data[tcp_start + 16] = 0;
    packet_data[tcp_start + 17] = 0;
    let tcp_len = packet_data.len() - tcp_start;
    let tcp_checksum = calculate_tcp_checksum(
        &packet_data[..ip_header_len],
        &packet_data[tcp_start..],
        tcp_len,
    );
    packet_data[tcp_start + 16] = (tcp_checksum >> 8) as u8;
    packet_data[tcp_start + 17] = (tcp_checksum & 0xFF) as u8;

    true
}

#[cfg(not(target_os = "windows"))]
pub fn rewrite_ipv4_src(
    _packet_data: &mut [u8],
    _new_src_addr: Ipv4Addr,
    _new_src_port: u16,
) -> bool {
    false
}

/// Calculate IP header checksum (RFC 1071)
fn calculate_ip_checksum(header: &[u8]) -> u16 {
    let mut sum: u32 = 0;

    // Sum 16-bit words
    for i in (0..header.len()).step_by(2) {
        let word = if i + 1 < header.len() {
            ((header[i] as u32) << 8) | (header[i + 1] as u32)
        } else {
            (header[i] as u32) << 8
        };
        sum = sum.wrapping_add(word);
    }

    // Fold 32-bit sum to 16 bits
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    // One's complement
    !sum as u16
}

/// Calculate TCP checksum with pseudo-header
fn calculate_tcp_checksum(ip_header: &[u8], tcp_segment: &[u8], tcp_len: usize) -> u16 {
    let mut sum: u32 = 0;

    // Pseudo-header: src IP, dst IP, zero, protocol, TCP length
    // Source IP (bytes 12-15)
    sum = sum.wrapping_add(((ip_header[12] as u32) << 8) | (ip_header[13] as u32));
    sum = sum.wrapping_add(((ip_header[14] as u32) << 8) | (ip_header[15] as u32));

    // Destination IP (bytes 16-19)
    sum = sum.wrapping_add(((ip_header[16] as u32) << 8) | (ip_header[17] as u32));
    sum = sum.wrapping_add(((ip_header[18] as u32) << 8) | (ip_header[19] as u32));

    // Protocol (TCP = 6)
    sum = sum.wrapping_add(6);

    // TCP length
    sum = sum.wrapping_add(tcp_len as u32);

    // Sum TCP segment
    for i in (0..tcp_segment.len()).step_by(2) {
        let word = if i + 1 < tcp_segment.len() {
            ((tcp_segment[i] as u32) << 8) | (tcp_segment[i + 1] as u32)
        } else {
            (tcp_segment[i] as u32) << 8
        };
        sum = sum.wrapping_add(word);
    }

    // Fold 32-bit sum to 16 bits
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    // One's complement
    !sum as u16
}

/// Extract TCP info from packet for NAT table management
pub fn extract_tcp_info(packet_data: &[u8]) -> Option<TcpExtractedInfo> {
    if packet_data.len() < 40 {
        return None;
    }

    let ip_version = (packet_data[0] >> 4) & 0x0F;
    if ip_version != 4 {
        return None;
    }

    let ip_header_len = ((packet_data[0] & 0x0F) as usize) * 4;
    let protocol = packet_data[9];
    if protocol != 6 {
        return None;
    }

    let tcp_start = ip_header_len;
    if packet_data.len() < tcp_start + 20 {
        return None;
    }

    // Extract addresses and ports
    let src_ip = Ipv4Addr::new(
        packet_data[12],
        packet_data[13],
        packet_data[14],
        packet_data[15],
    );
    let dst_ip = Ipv4Addr::new(
        packet_data[16],
        packet_data[17],
        packet_data[18],
        packet_data[19],
    );
    let src_port = ((packet_data[tcp_start] as u16) << 8) | (packet_data[tcp_start + 1] as u16);
    let dst_port = ((packet_data[tcp_start + 2] as u16) << 8) | (packet_data[tcp_start + 3] as u16);

    // TCP flags (byte 13 of TCP header)
    let flags_byte = packet_data[tcp_start + 13];
    let syn = (flags_byte & 0x02) != 0;
    let ack = (flags_byte & 0x10) != 0;
    let fin = (flags_byte & 0x01) != 0;
    let rst = (flags_byte & 0x04) != 0;

    Some(TcpExtractedInfo {
        src_ip,
        dst_ip,
        src_port,
        dst_port,
        syn,
        ack,
        fin,
        rst,
    })
}

/// Extracted TCP info from packet
#[derive(Debug, Clone)]
pub struct TcpExtractedInfo {
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
    pub src_port: u16,
    pub dst_port: u16,
    pub syn: bool,
    pub ack: bool,
    pub fin: bool,
    pub rst: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_checksum() {
        // Example from RFC 1071
        let header = [
            0x45, 0x00, 0x00, 0x73, 0x00, 0x00, 0x40, 0x00, 0x40, 0x11, 0x00, 0x00, 0xc0, 0xa8,
            0x00, 0x01, 0xc0, 0xa8, 0x00, 0xc7,
        ];
        let checksum = calculate_ip_checksum(&header);
        // The checksum should be placed at bytes 10-11
        assert_ne!(checksum, 0);
    }

    #[test]
    fn test_extract_tcp_info() {
        // Minimal TCP SYN packet
        let mut packet = vec![0u8; 60];
        // IP header
        packet[0] = 0x45; // Version 4, IHL 5
        packet[9] = 6; // Protocol TCP
                       // Source IP: 192.168.1.100
        packet[12] = 192;
        packet[13] = 168;
        packet[14] = 1;
        packet[15] = 100;
        // Dest IP: 93.184.216.34
        packet[16] = 93;
        packet[17] = 184;
        packet[18] = 216;
        packet[19] = 34;
        // TCP header (starts at byte 20)
        packet[20] = 0xC0; // Src port high
        packet[21] = 0x00; // Src port low (49152)
        packet[22] = 0x01; // Dst port high
        packet[23] = 0xBB; // Dst port low (443)
        packet[33] = 0x02; // SYN flag

        let info = extract_tcp_info(&packet).unwrap();
        assert_eq!(info.src_ip, Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(info.dst_ip, Ipv4Addr::new(93, 184, 216, 34));
        assert_eq!(info.src_port, 49152);
        assert_eq!(info.dst_port, 443);
        assert!(info.syn);
        assert!(!info.ack);
    }
}
