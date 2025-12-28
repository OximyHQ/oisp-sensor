//! Connection state tracking
//!
//! Tracks TCP connections and associates them with process IDs using
//! Windows APIs (GetExtendedTcpTable).

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};
use tracing::{debug, trace, warn};

use super::windivert_capture::{PacketInfo, TcpFlags};

/// Information about a tracked connection
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Local address (our side)
    pub local_addr: SocketAddr,

    /// Remote address (server)
    pub remote_addr: SocketAddr,

    /// Process ID (if known)
    pub pid: Option<u32>,

    /// Process name (if known)
    pub process_name: Option<String>,

    /// Connection state
    pub state: ConnectionState,

    /// When the connection was first seen
    pub first_seen: Instant,

    /// When we last saw activity
    pub last_seen: Instant,

    /// Bytes sent
    pub bytes_sent: u64,

    /// Bytes received
    pub bytes_received: u64,
}

/// TCP connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// SYN sent, waiting for SYN-ACK
    SynSent,
    /// SYN-ACK received, waiting for ACK
    SynReceived,
    /// Connection established
    Established,
    /// FIN sent, waiting for FIN-ACK
    FinWait,
    /// Connection closing
    Closing,
    /// Connection closed
    Closed,
}

/// Key for identifying a connection
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ConnectionKey {
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
}

impl ConnectionKey {
    fn new(local: SocketAddr, remote: SocketAddr) -> Self {
        Self {
            local_addr: local,
            remote_addr: remote,
        }
    }

    /// Create reverse key (for matching return traffic)
    fn reverse(&self) -> Self {
        Self {
            local_addr: self.remote_addr,
            remote_addr: self.local_addr,
        }
    }
}

/// Tracks TCP connections and their process associations
pub struct ConnectionTracker {
    /// Active connections
    connections: HashMap<ConnectionKey, ConnectionInfo>,

    /// Connection timeout (remove stale connections)
    timeout: Duration,

    /// Last time we cleaned up stale connections
    last_cleanup: Instant,
}

impl ConnectionTracker {
    /// Create a new connection tracker
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            timeout: Duration::from_secs(300), // 5 minute timeout
            last_cleanup: Instant::now(),
        }
    }

    /// Process a packet and update connection tracking
    ///
    /// Returns `Some(ConnectionInfo)` for new or significant connection events
    pub fn process_packet(&mut self, packet: &PacketInfo) -> Option<ConnectionInfo> {
        // Clean up stale connections periodically
        if self.last_cleanup.elapsed() > Duration::from_secs(60) {
            self.cleanup_stale();
            self.last_cleanup = Instant::now();
        }

        let tcp_info = packet.tcp_info.as_ref()?;
        let key = ConnectionKey::new(tcp_info.src_addr, tcp_info.dst_addr);
        let reverse_key = key.reverse();

        // Check if this is outgoing (we're the source) or incoming (we're the dest)
        let is_outgoing = self.is_local_addr(&tcp_info.src_addr);

        // Determine the canonical key (always local -> remote)
        let canonical_key = if is_outgoing {
            key.clone()
        } else {
            reverse_key.clone()
        };

        // Handle based on TCP flags
        if tcp_info.flags.is_handshake_start() {
            // New connection (SYN)
            return self.handle_syn(packet, &canonical_key, is_outgoing);
        } else if tcp_info.flags.is_handshake_ack() {
            // Connection acknowledged (SYN-ACK)
            return self.handle_syn_ack(packet, &canonical_key, is_outgoing);
        } else if tcp_info.flags.is_connection_end() {
            // Connection ending (FIN or RST)
            return self.handle_fin(packet, &canonical_key);
        } else if tcp_info.flags.ack && tcp_info.payload_len > 0 {
            // Data packet
            return self.handle_data(packet, &canonical_key, is_outgoing);
        }

        None
    }

    /// Handle SYN packet (connection start)
    fn handle_syn(
        &mut self,
        packet: &PacketInfo,
        key: &ConnectionKey,
        is_outgoing: bool,
    ) -> Option<ConnectionInfo> {
        let tcp_info = packet.tcp_info.as_ref()?;

        // Try to get process ID
        let pid = if is_outgoing {
            self.lookup_pid_for_local_port(tcp_info.src_addr.port())
        } else {
            None
        };

        let process_name = pid.and_then(|p| self.get_process_name(p));

        let conn_info = ConnectionInfo {
            local_addr: if is_outgoing {
                tcp_info.src_addr
            } else {
                tcp_info.dst_addr
            },
            remote_addr: if is_outgoing {
                tcp_info.dst_addr
            } else {
                tcp_info.src_addr
            },
            pid,
            process_name: process_name.clone(),
            state: ConnectionState::SynSent,
            first_seen: Instant::now(),
            last_seen: Instant::now(),
            bytes_sent: 0,
            bytes_received: 0,
        };

        debug!(
            "New connection: {} -> {} (PID: {:?}, Process: {:?})",
            conn_info.local_addr, conn_info.remote_addr, conn_info.pid, process_name
        );

        self.connections.insert(key.clone(), conn_info.clone());
        Some(conn_info)
    }

    /// Handle SYN-ACK packet
    fn handle_syn_ack(
        &mut self,
        _packet: &PacketInfo,
        key: &ConnectionKey,
        _is_outgoing: bool,
    ) -> Option<ConnectionInfo> {
        if let Some(conn) = self.connections.get_mut(key) {
            conn.state = ConnectionState::SynReceived;
            conn.last_seen = Instant::now();
            trace!(
                "Connection SYN-ACK: {} -> {}",
                conn.local_addr,
                conn.remote_addr
            );
        }
        None
    }

    /// Handle FIN or RST packet
    fn handle_fin(&mut self, _packet: &PacketInfo, key: &ConnectionKey) -> Option<ConnectionInfo> {
        if let Some(conn) = self.connections.get_mut(key) {
            conn.state = ConnectionState::Closing;
            conn.last_seen = Instant::now();
            debug!(
                "Connection closing: {} -> {} (sent: {} bytes, recv: {} bytes)",
                conn.local_addr, conn.remote_addr, conn.bytes_sent, conn.bytes_received
            );
            return Some(conn.clone());
        }
        None
    }

    /// Handle data packet
    fn handle_data(
        &mut self,
        packet: &PacketInfo,
        key: &ConnectionKey,
        is_outgoing: bool,
    ) -> Option<ConnectionInfo> {
        let tcp_info = packet.tcp_info.as_ref()?;

        if let Some(conn) = self.connections.get_mut(key) {
            conn.state = ConnectionState::Established;
            conn.last_seen = Instant::now();

            if is_outgoing {
                conn.bytes_sent += tcp_info.payload_len as u64;
            } else {
                conn.bytes_received += tcp_info.payload_len as u64;
            }

            // Return connection info for significant data packets
            if tcp_info.payload_len > 0 {
                return Some(conn.clone());
            }
        } else {
            // Connection not tracked, might have started before we did
            // Try to create a new entry
            let pid = if is_outgoing {
                self.lookup_pid_for_local_port(tcp_info.src_addr.port())
            } else {
                self.lookup_pid_for_local_port(tcp_info.dst_addr.port())
            };

            let process_name = pid.and_then(|p| self.get_process_name(p));

            let conn_info = ConnectionInfo {
                local_addr: if is_outgoing {
                    tcp_info.src_addr
                } else {
                    tcp_info.dst_addr
                },
                remote_addr: if is_outgoing {
                    tcp_info.dst_addr
                } else {
                    tcp_info.src_addr
                },
                pid,
                process_name,
                state: ConnectionState::Established,
                first_seen: Instant::now(),
                last_seen: Instant::now(),
                bytes_sent: if is_outgoing {
                    tcp_info.payload_len as u64
                } else {
                    0
                },
                bytes_received: if !is_outgoing {
                    tcp_info.payload_len as u64
                } else {
                    0
                },
            };

            self.connections.insert(key.clone(), conn_info.clone());
            return Some(conn_info);
        }

        None
    }

    /// Check if an address is local
    fn is_local_addr(&self, addr: &SocketAddr) -> bool {
        match addr.ip() {
            IpAddr::V4(ip) => {
                ip.is_loopback() ||
                ip.is_private() ||
                // Could also check against local interfaces
                true // For now, assume outgoing
            }
            IpAddr::V6(ip) => ip.is_loopback(),
        }
    }

    /// Look up process ID for a local port
    fn lookup_pid_for_local_port(&self, port: u16) -> Option<u32> {
        // Use Windows API to get the TCP table
        #[cfg(windows)]
        {
            self.get_pid_from_tcp_table(port)
        }

        #[cfg(not(windows))]
        {
            None
        }
    }

    /// Get PID from Windows TCP table
    #[cfg(windows)]
    fn get_pid_from_tcp_table(&self, local_port: u16) -> Option<u32> {
        use windows::Win32::NetworkManagement::IpHelper::{
            GetExtendedTcpTable, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL,
        };
        use windows::Win32::Networking::WinSock::AF_INET;

        unsafe {
            let mut size: u32 = 0;

            // First call to get required size
            let _ = GetExtendedTcpTable(
                None,
                &mut size,
                false,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );

            if size == 0 {
                return None;
            }

            // Allocate buffer
            let mut buffer = vec![0u8; size as usize];

            // Second call to get data
            let result = GetExtendedTcpTable(
                Some(buffer.as_mut_ptr() as *mut _),
                &mut size,
                false,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );

            if result.is_err() {
                return None;
            }

            // Parse the table
            let table = &*(buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
            let entries =
                std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize);

            for entry in entries {
                // Convert port from network byte order
                let entry_port = u16::from_be(entry.dwLocalPort as u16);
                if entry_port == local_port {
                    return Some(entry.dwOwningPid);
                }
            }

            None
        }
    }

    /// Get process name from PID
    fn get_process_name(&self, pid: u32) -> Option<String> {
        #[cfg(windows)]
        {
            self.get_process_name_windows(pid)
        }

        #[cfg(not(windows))]
        {
            None
        }
    }

    /// Get process name on Windows
    #[cfg(windows)]
    fn get_process_name_windows(&self, pid: u32) -> Option<String> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
        };

        unsafe {
            let handle =
                OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

            let mut name_buf = [0u16; 260];
            let len = GetModuleBaseNameW(handle, None, &mut name_buf);

            let _ = CloseHandle(handle);

            if len > 0 {
                Some(String::from_utf16_lossy(&name_buf[..len as usize]))
            } else {
                None
            }
        }
    }

    /// Get number of active connections
    pub fn active_connections(&self) -> usize {
        self.connections
            .values()
            .filter(|c| c.state != ConnectionState::Closed)
            .count()
    }

    /// Clean up stale connections
    fn cleanup_stale(&mut self) {
        let timeout = self.timeout;
        let before = self.connections.len();

        self.connections.retain(|_, conn| {
            conn.last_seen.elapsed() < timeout && conn.state != ConnectionState::Closed
        });

        let removed = before - self.connections.len();
        if removed > 0 {
            debug!("Cleaned up {} stale connections", removed);
        }
    }
}

impl Default for ConnectionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_key_reverse() {
        let local = "192.168.1.100:12345".parse().unwrap();
        let remote = "93.184.216.34:443".parse().unwrap();

        let key = ConnectionKey::new(local, remote);
        let reverse = key.reverse();

        assert_eq!(reverse.local_addr, remote);
        assert_eq!(reverse.remote_addr, local);
    }
}
