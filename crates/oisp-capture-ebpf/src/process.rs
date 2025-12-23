//! Process capture via eBPF tracepoints

/// Tracepoints for process events
pub static PROCESS_TRACEPOINTS: &[(&str, &str)] = &[
    ("sched", "sched_process_exec"),
    ("sched", "sched_process_exit"),
    ("sched", "sched_process_fork"),
];
