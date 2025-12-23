//! SSL/TLS capture via eBPF uprobes

#![cfg(target_os = "linux")]

use std::path::Path;

/// Common SSL library paths
pub static SSL_LIBRARY_PATHS: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/libssl.so.3",
    "/usr/lib/x86_64-linux-gnu/libssl.so.1.1",
    "/lib/x86_64-linux-gnu/libssl.so.3",
    "/lib/x86_64-linux-gnu/libssl.so.1.1",
    "/usr/lib64/libssl.so.3",
    "/usr/lib64/libssl.so.1.1",
    "/usr/lib/libssl.so",
];

/// Find available SSL libraries on the system
pub fn find_ssl_libraries() -> Vec<String> {
    SSL_LIBRARY_PATHS
        .iter()
        .filter(|p| Path::new(p).exists())
        .map(|p| p.to_string())
        .collect()
}

/// Functions to attach uprobes to
pub static SSL_FUNCTIONS: &[&str] = &[
    "SSL_read",
    "SSL_read_ex",
    "SSL_write",
    "SSL_write_ex",
];

/// Resolve function offset in a library
pub fn get_function_offset(library_path: &str, function_name: &str) -> Option<usize> {
    // TODO: Use goblin or object crate to parse ELF and find symbol offset
    None
}

