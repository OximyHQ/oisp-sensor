//! Network capture via eBPF

/// Tracepoints for network events
pub static NETWORK_TRACEPOINTS: &[(&str, &str)] = &[
    ("syscalls", "sys_enter_connect"),
    ("syscalls", "sys_exit_connect"),
    ("syscalls", "sys_enter_accept"),
    ("syscalls", "sys_exit_accept"),
    ("syscalls", "sys_enter_accept4"),
    ("syscalls", "sys_exit_accept4"),
];
