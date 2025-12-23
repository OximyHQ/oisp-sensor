//! File capture via eBPF

#![cfg(target_os = "linux")]

/// Tracepoints for file events
pub static FILE_TRACEPOINTS: &[(&str, &str)] = &[
    ("syscalls", "sys_enter_openat"),
    ("syscalls", "sys_exit_openat"),
    ("syscalls", "sys_enter_read"),
    ("syscalls", "sys_exit_read"),
    ("syscalls", "sys_enter_write"),
    ("syscalls", "sys_exit_write"),
    ("syscalls", "sys_enter_close"),
];

