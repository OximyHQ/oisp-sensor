//! Linux process information via /proc filesystem
//!
//! Provides process attribution and socket-to-process mapping for Linux.
//! Used to enrich events captured by sslsniff with full process info.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, trace, warn};

/// Process information read from /proc
#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub comm: Option<String>,
    pub exe: Option<String>,
    pub uid: Option<u32>,
    pub cmdline: Option<String>,
}

impl ProcInfo {
    /// Read process info from /proc/{pid}
    pub fn from_pid(pid: u32) -> Option<Self> {
        let proc_path = format!("/proc/{}", pid);
        if !Path::new(&proc_path).exists() {
            return None;
        }

        let mut info = ProcInfo {
            pid,
            ppid: None,
            comm: None,
            exe: None,
            uid: None,
            cmdline: None,
        };

        // Read /proc/{pid}/stat for ppid
        if let Ok(stat) = fs::read_to_string(format!("{}/stat", proc_path)) {
            // Format: pid (comm) state ppid ...
            // Need to handle comm with spaces/parens carefully
            if let Some(ppid) = parse_stat_ppid(&stat) {
                info.ppid = Some(ppid);
            }
        }

        // Read /proc/{pid}/comm for process name
        if let Ok(comm) = fs::read_to_string(format!("{}/comm", proc_path)) {
            info.comm = Some(comm.trim().to_string());
        }

        // Read /proc/{pid}/exe symlink for executable path
        if let Ok(exe) = fs::read_link(format!("{}/exe", proc_path)) {
            // Filter out deleted executables
            let exe_str = exe.to_string_lossy().to_string();
            if !exe_str.contains(" (deleted)") {
                info.exe = Some(exe_str);
            }
        }

        // Read /proc/{pid}/status for uid
        if let Ok(status) = fs::read_to_string(format!("{}/status", proc_path)) {
            for line in status.lines() {
                if line.starts_with("Uid:") {
                    // Uid: real effective saved fs
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(uid) = parts[1].parse::<u32>() {
                            info.uid = Some(uid);
                        }
                    }
                    break;
                }
            }
        }

        // Read /proc/{pid}/cmdline
        if let Ok(cmdline_bytes) = fs::read(format!("{}/cmdline", proc_path)) {
            // cmdline is NUL-separated
            let cmdline: String = cmdline_bytes
                .split(|&b| b == 0)
                .filter(|s| !s.is_empty())
                .map(|s| String::from_utf8_lossy(s).to_string())
                .collect::<Vec<_>>()
                .join(" ");
            if !cmdline.is_empty() {
                info.cmdline = Some(cmdline);
            }
        }

        Some(info)
    }
}

/// Parse PPID from /proc/{pid}/stat
/// Format: pid (comm) state ppid ...
/// The comm can contain spaces and parens, so we find the last ) and parse from there
fn parse_stat_ppid(stat: &str) -> Option<u32> {
    // Find the last ) which ends the comm field
    let close_paren = stat.rfind(')')?;
    let after_comm = &stat[close_paren + 1..];

    // Fields after comm: state ppid pgrp session tty_nr ...
    let mut fields = after_comm.split_whitespace();
    let _state = fields.next()?;
    let ppid_str = fields.next()?;

    ppid_str.parse::<u32>().ok()
}

/// Socket inode to PID mapping
/// Built by scanning /proc/*/fd/* for socket inodes
#[derive(Debug, Default)]
pub struct SocketToPidMap {
    /// Maps socket inode number to (pid, fd)
    inode_to_pid: HashMap<u64, (u32, i32)>,
}

impl SocketToPidMap {
    /// Build a fresh socket-to-PID map by scanning /proc
    pub fn build() -> Self {
        let mut map = SocketToPidMap::default();
        map.refresh();
        map
    }

    /// Refresh the map by rescanning /proc
    pub fn refresh(&mut self) {
        self.inode_to_pid.clear();

        // Read /proc to get all PIDs
        let proc_dir = match fs::read_dir("/proc") {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to read /proc: {}", e);
                return;
            }
        };

        for entry in proc_dir.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Skip non-PID entries
            let pid: u32 = match name_str.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Scan /proc/{pid}/fd/
            let fd_dir = format!("/proc/{}/fd", pid);
            let fd_entries = match fs::read_dir(&fd_dir) {
                Ok(d) => d,
                Err(_) => continue, // Permission denied or process exited
            };

            for fd_entry in fd_entries.flatten() {
                let fd_name = fd_entry.file_name();
                let fd: i32 = match fd_name.to_string_lossy().parse() {
                    Ok(f) => f,
                    Err(_) => continue,
                };

                // Read the symlink target
                let link_target = match fs::read_link(fd_entry.path()) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let target_str = link_target.to_string_lossy();

                // Check if it's a socket: socket:[inode]
                if target_str.starts_with("socket:[") && target_str.ends_with(']') {
                    let inode_str = &target_str[8..target_str.len() - 1];
                    if let Ok(inode) = inode_str.parse::<u64>() {
                        self.inode_to_pid.insert(inode, (pid, fd));
                    }
                }
            }
        }

        trace!("Socket map refreshed: {} entries", self.inode_to_pid.len());
    }

    /// Look up PID and FD for a socket inode
    pub fn get_pid_for_inode(&self, inode: u64) -> Option<(u32, i32)> {
        self.inode_to_pid.get(&inode).copied()
    }
}

/// TCP connection info from /proc/net/tcp
#[derive(Debug, Clone)]
pub struct TcpConnection {
    pub local_addr: std::net::Ipv4Addr,
    pub local_port: u16,
    pub remote_addr: std::net::Ipv4Addr,
    pub remote_port: u16,
    pub inode: u64,
    pub uid: u32,
}

/// Parse /proc/net/tcp to get TCP connection info
/// Returns a map from (local_port, remote_addr, remote_port) to inode
pub fn parse_proc_net_tcp() -> Vec<TcpConnection> {
    let mut connections = Vec::new();

    for path in &["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines().skip(1) {
                // Skip header
                if let Some(conn) = parse_tcp_line(line) {
                    connections.push(conn);
                }
            }
        }
    }

    connections
}

/// Parse a single line from /proc/net/tcp
/// Format: sl local_address rem_address st tx_queue rx_queue tr tm->when retrnsmt uid timeout inode ...
fn parse_tcp_line(line: &str) -> Option<TcpConnection> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 {
        return None;
    }

    // Local address: hex_ip:hex_port
    let (local_addr, local_port) = parse_hex_addr(parts[1])?;

    // Remote address
    let (remote_addr, remote_port) = parse_hex_addr(parts[2])?;

    // UID is at index 7
    let uid: u32 = parts[7].parse().ok()?;

    // Inode is at index 9
    let inode: u64 = parts[9].parse().ok()?;

    Some(TcpConnection {
        local_addr,
        local_port,
        remote_addr,
        remote_port,
        inode,
        uid,
    })
}

/// Parse hex address format: AABBCCDD:PORT (in little-endian for IPv4)
fn parse_hex_addr(hex: &str) -> Option<(std::net::Ipv4Addr, u16)> {
    let parts: Vec<&str> = hex.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    // IP is in little-endian hex
    let ip_hex = u32::from_str_radix(parts[0], 16).ok()?;
    // Convert from network byte order (which /proc shows in little-endian on little-endian systems)
    let ip = std::net::Ipv4Addr::from(ip_hex.to_be());

    let port = u16::from_str_radix(parts[1], 16).ok()?;

    Some((ip, port))
}

/// Find the PID that owns a TCP connection
pub fn find_pid_for_connection(
    local_port: u16,
    remote_addr: std::net::Ipv4Addr,
    remote_port: u16,
    socket_map: &SocketToPidMap,
) -> Option<(u32, i32)> {
    let connections = parse_proc_net_tcp();

    for conn in connections {
        if conn.local_port == local_port
            && conn.remote_addr == remote_addr
            && conn.remote_port == remote_port
        {
            return socket_map.get_pid_for_inode(conn.inode);
        }
    }

    None
}

/// Cache for process info to avoid repeated /proc reads
#[derive(Default)]
pub struct ProcInfoCache {
    cache: HashMap<u32, Option<ProcInfo>>,
}

impl ProcInfoCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get process info for a PID, using cache
    pub fn get(&mut self, pid: u32) -> Option<&ProcInfo> {
        if !self.cache.contains_key(&pid) {
            let info = ProcInfo::from_pid(pid);
            self.cache.insert(pid, info);
        }
        self.cache.get(&pid).and_then(|o| o.as_ref())
    }

    /// Clear the cache (call periodically to handle process churn)
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Remove a specific PID from cache
    pub fn invalidate(&mut self, pid: u32) {
        self.cache.remove(&pid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stat_ppid() {
        // Normal case
        let stat = "1234 (bash) S 1000 1234 1234 0 -1";
        assert_eq!(parse_stat_ppid(stat), Some(1000));

        // Comm with spaces
        let stat = "5678 (Web Content) S 1234 5678 5678 0 -1";
        assert_eq!(parse_stat_ppid(stat), Some(1234));

        // Comm with parens
        let stat = "9999 (my (cool) app) S 100 9999 9999 0 -1";
        assert_eq!(parse_stat_ppid(stat), Some(100));
    }

    #[test]
    fn test_parse_hex_addr() {
        // 127.0.0.1:8080
        // In /proc/net/tcp, 127.0.0.1 appears as 0100007F (little-endian)
        let (ip, port) = parse_hex_addr("0100007F:1F90").unwrap();
        assert_eq!(port, 8080);
        // Note: The IP parsing depends on system endianness
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_proc_info_self() {
        let info = ProcInfo::from_pid(std::process::id()).unwrap();
        assert!(info.comm.is_some());
        assert!(info.exe.is_some());
    }
}
