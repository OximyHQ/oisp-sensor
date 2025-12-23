//! Performance and resource metrics for OISP Sensor
//!
//! Provides metrics collection for monitoring sensor health and performance.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Global metrics collector
#[derive(Debug)]
pub struct MetricsCollector {
    /// When the collector was started
    start_time: Instant,
    /// Capture metrics
    pub capture: CaptureMetrics,
    /// Pipeline metrics
    pub pipeline: PipelineMetrics,
    /// Process resource metrics (pid -> ProcessMetrics)
    pub processes: parking_lot::RwLock<HashMap<u32, ProcessMetrics>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            capture: CaptureMetrics::default(),
            pipeline: PipelineMetrics::default(),
            processes: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Export metrics in Prometheus format
    pub fn to_prometheus(&self) -> String {
        let mut output = String::new();

        // Uptime
        output.push_str("# HELP oisp_uptime_seconds Time since sensor started\n");
        output.push_str("# TYPE oisp_uptime_seconds gauge\n");
        output.push_str(&format!(
            "oisp_uptime_seconds {}\n\n",
            self.uptime_seconds()
        ));

        // Capture metrics
        output.push_str("# HELP oisp_capture_events_total Total events captured\n");
        output.push_str("# TYPE oisp_capture_events_total counter\n");
        output.push_str(&format!(
            "oisp_capture_events_total{{type=\"ssl\"}} {}\n",
            self.capture.ssl_events.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "oisp_capture_events_total{{type=\"network\"}} {}\n",
            self.capture.network_events.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "oisp_capture_events_total{{type=\"process\"}} {}\n",
            self.capture.process_events.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "oisp_capture_events_total{{type=\"file\"}} {}\n\n",
            self.capture.file_events.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP oisp_capture_bytes_total Total bytes captured\n");
        output.push_str("# TYPE oisp_capture_bytes_total counter\n");
        output.push_str(&format!(
            "oisp_capture_bytes_total {}\n\n",
            self.capture.bytes_captured.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP oisp_capture_errors_total Total capture errors\n");
        output.push_str("# TYPE oisp_capture_errors_total counter\n");
        output.push_str(&format!(
            "oisp_capture_errors_total {}\n\n",
            self.capture.errors.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP oisp_capture_dropped_total Total events dropped\n");
        output.push_str("# TYPE oisp_capture_dropped_total counter\n");
        output.push_str(&format!(
            "oisp_capture_dropped_total {}\n\n",
            self.capture.dropped.load(Ordering::Relaxed)
        ));

        // Pipeline metrics
        output.push_str(
            "# HELP oisp_pipeline_events_processed_total Total events processed by pipeline\n",
        );
        output.push_str("# TYPE oisp_pipeline_events_processed_total counter\n");
        output.push_str(&format!(
            "oisp_pipeline_events_processed_total {}\n\n",
            self.pipeline.events_processed.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP oisp_pipeline_events_exported_total Total events exported\n");
        output.push_str("# TYPE oisp_pipeline_events_exported_total counter\n");
        output.push_str(&format!(
            "oisp_pipeline_events_exported_total {}\n\n",
            self.pipeline.events_exported.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP oisp_pipeline_ai_events_total Total AI events detected\n");
        output.push_str("# TYPE oisp_pipeline_ai_events_total counter\n");
        output.push_str(&format!(
            "oisp_pipeline_ai_events_total {}\n\n",
            self.pipeline.ai_events.load(Ordering::Relaxed)
        ));

        // Ring buffer metrics
        output.push_str("# HELP oisp_ringbuf_polls_total Total ring buffer poll operations\n");
        output.push_str("# TYPE oisp_ringbuf_polls_total counter\n");
        output.push_str(&format!(
            "oisp_ringbuf_polls_total {}\n\n",
            self.capture.ringbuf_polls.load(Ordering::Relaxed)
        ));

        // Process metrics
        let processes = self.processes.read();
        if !processes.is_empty() {
            output.push_str("# HELP oisp_process_cpu_percent CPU usage percentage per process\n");
            output.push_str("# TYPE oisp_process_cpu_percent gauge\n");
            for (pid, metrics) in processes.iter() {
                output.push_str(&format!(
                    "oisp_process_cpu_percent{{pid=\"{}\",comm=\"{}\"}} {:.2}\n",
                    pid, metrics.comm, metrics.cpu_percent
                ));
            }
            output.push('\n');

            output.push_str(
                "# HELP oisp_process_memory_rss_bytes Resident set size in bytes per process\n",
            );
            output.push_str("# TYPE oisp_process_memory_rss_bytes gauge\n");
            for (pid, metrics) in processes.iter() {
                output.push_str(&format!(
                    "oisp_process_memory_rss_bytes{{pid=\"{}\",comm=\"{}\"}} {}\n",
                    pid, metrics.comm, metrics.memory_rss_bytes
                ));
            }
            output.push('\n');

            output.push_str(
                "# HELP oisp_process_memory_vms_bytes Virtual memory size in bytes per process\n",
            );
            output.push_str("# TYPE oisp_process_memory_vms_bytes gauge\n");
            for (pid, metrics) in processes.iter() {
                output.push_str(&format!(
                    "oisp_process_memory_vms_bytes{{pid=\"{}\",comm=\"{}\"}} {}\n",
                    pid, metrics.comm, metrics.memory_vms_bytes
                ));
            }
            output.push('\n');
        }

        output
    }

    /// Export metrics as JSON
    pub fn to_json(&self) -> serde_json::Value {
        let processes = self.processes.read();
        let process_metrics: HashMap<String, serde_json::Value> = processes
            .iter()
            .map(|(pid, m)| {
                (
                    pid.to_string(),
                    serde_json::json!({
                        "comm": m.comm,
                        "cpu_percent": m.cpu_percent,
                        "memory_rss_bytes": m.memory_rss_bytes,
                        "memory_vms_bytes": m.memory_vms_bytes,
                        "last_updated_ns": m.last_updated_ns,
                    }),
                )
            })
            .collect();

        serde_json::json!({
            "uptime_seconds": self.uptime_seconds(),
            "capture": {
                "ssl_events": self.capture.ssl_events.load(Ordering::Relaxed),
                "network_events": self.capture.network_events.load(Ordering::Relaxed),
                "process_events": self.capture.process_events.load(Ordering::Relaxed),
                "file_events": self.capture.file_events.load(Ordering::Relaxed),
                "bytes_captured": self.capture.bytes_captured.load(Ordering::Relaxed),
                "errors": self.capture.errors.load(Ordering::Relaxed),
                "dropped": self.capture.dropped.load(Ordering::Relaxed),
                "ringbuf_polls": self.capture.ringbuf_polls.load(Ordering::Relaxed),
            },
            "pipeline": {
                "events_processed": self.pipeline.events_processed.load(Ordering::Relaxed),
                "events_exported": self.pipeline.events_exported.load(Ordering::Relaxed),
                "ai_events": self.pipeline.ai_events.load(Ordering::Relaxed),
            },
            "processes": process_metrics,
        })
    }

    /// Update process metrics from /proc
    #[cfg(target_os = "linux")]
    pub fn update_process_metrics(&self, pid: u32) {
        let mut processes = self.processes.write();

        // Get existing metrics or create new
        let existing = processes.get(&pid).cloned();

        if let Some(new_metrics) = read_process_metrics_with_prev(pid, existing) {
            processes.insert(pid, new_metrics);
        }
    }

    /// Update process metrics (non-Linux stub)
    #[cfg(not(target_os = "linux"))]
    pub fn update_process_metrics(&self, _pid: u32) {
        // No-op on non-Linux
    }

    /// Update metrics for all tracked processes
    #[cfg(target_os = "linux")]
    pub fn update_all_process_metrics(&self, pids: &[u32]) {
        for &pid in pids {
            self.update_process_metrics(pid);
        }
    }

    /// Update metrics for all tracked processes (non-Linux stub)
    #[cfg(not(target_os = "linux"))]
    pub fn update_all_process_metrics(&self, _pids: &[u32]) {
        // No-op on non-Linux
    }

    /// Remove stale process metrics (processes that have exited)
    pub fn cleanup_stale_processes(&self, active_pids: &[u32]) {
        let mut processes = self.processes.write();
        processes.retain(|pid, _| active_pids.contains(pid));
    }

    /// Get list of tracked PIDs
    pub fn tracked_pids(&self) -> Vec<u32> {
        self.processes.read().keys().copied().collect()
    }

    /// Add a PID to track (will be updated on next update cycle)
    pub fn track_pid(&self, pid: u32, comm: String) {
        let mut processes = self.processes.write();
        processes
            .entry(pid)
            .or_insert_with(|| ProcessMetrics::new(comm));
    }
}

/// Capture-related metrics
#[derive(Debug, Default)]
pub struct CaptureMetrics {
    pub ssl_events: AtomicU64,
    pub network_events: AtomicU64,
    pub process_events: AtomicU64,
    pub file_events: AtomicU64,
    pub bytes_captured: AtomicU64,
    pub errors: AtomicU64,
    pub dropped: AtomicU64,
    pub ringbuf_polls: AtomicU64,
}

/// Pipeline-related metrics
#[derive(Debug, Default)]
pub struct PipelineMetrics {
    pub events_processed: AtomicU64,
    pub events_exported: AtomicU64,
    pub ai_events: AtomicU64,
}

/// Per-process resource metrics
#[derive(Debug, Clone)]
pub struct ProcessMetrics {
    pub comm: String,
    pub cpu_percent: f64,
    pub memory_rss_bytes: u64,
    pub memory_vms_bytes: u64,
    pub last_updated_ns: u64,
    // For CPU calculation (only used on Linux)
    #[allow(dead_code)]
    prev_utime: u64,
    #[allow(dead_code)]
    prev_stime: u64,
    #[allow(dead_code)]
    prev_time_ns: u64,
}

impl ProcessMetrics {
    pub fn new(comm: String) -> Self {
        Self {
            comm,
            cpu_percent: 0.0,
            memory_rss_bytes: 0,
            memory_vms_bytes: 0,
            last_updated_ns: 0,
            prev_utime: 0,
            prev_stime: 0,
            prev_time_ns: 0,
        }
    }
}

/// Read process metrics from /proc/[pid]/stat and /proc/[pid]/statm
/// Takes previous metrics to calculate CPU percentage
#[cfg(target_os = "linux")]
pub fn read_process_metrics_with_prev(
    pid: u32,
    prev: Option<ProcessMetrics>,
) -> Option<ProcessMetrics> {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Read /proc/[pid]/stat
    let stat_path = format!("/proc/{}/stat", pid);
    let stat_content = fs::read_to_string(&stat_path).ok()?;

    // Parse stat file - format: pid (comm) state ppid pgrp session tty_nr tpgid flags
    //                          minflt cminflt majflt cmajflt utime stime cutime cstime ...
    // We need: comm (field 2), utime (field 14), stime (field 15)
    let comm_start = stat_content.find('(')?;
    let comm_end = stat_content.rfind(')')?;
    let comm = stat_content[comm_start + 1..comm_end].to_string();

    let rest = &stat_content[comm_end + 2..]; // Skip ") "
    let fields: Vec<&str> = rest.split_whitespace().collect();

    // utime is at index 11 (0-based after comm), stime is at index 12
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;

    // Read /proc/[pid]/statm for memory
    let statm_path = format!("/proc/{}/statm", pid);
    let statm_content = fs::read_to_string(&statm_path).ok()?;
    let statm_fields: Vec<&str> = statm_content.split_whitespace().collect();

    // Fields: size resident shared text lib data dt
    // size = total VM pages, resident = RSS pages
    let vms_pages: u64 = statm_fields.first()?.parse().ok()?;
    let rss_pages: u64 = statm_fields.get(1)?.parse().ok()?;

    // Page size is typically 4096 bytes
    let page_size: u64 = 4096;
    let memory_rss_bytes = rss_pages * page_size;
    let memory_vms_bytes = vms_pages * page_size;

    let now_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos() as u64;

    // Calculate CPU percentage if we have previous data
    let (cpu_percent, prev_utime, prev_stime, prev_time_ns) = if let Some(prev) = prev {
        // Calculate delta
        let delta_time_ns = now_ns.saturating_sub(prev.prev_time_ns);
        let delta_utime = utime.saturating_sub(prev.prev_utime);
        let delta_stime = stime.saturating_sub(prev.prev_stime);
        let delta_cpu_ticks = delta_utime + delta_stime;

        // Convert ticks to nanoseconds (assuming 100 ticks/sec = 10ms per tick)
        // USER_HZ is typically 100 on Linux
        let tick_ns: u64 = 10_000_000; // 10ms in ns
        let delta_cpu_ns = delta_cpu_ticks * tick_ns;

        // Calculate percentage
        let cpu_percent = if delta_time_ns > 0 {
            (delta_cpu_ns as f64 / delta_time_ns as f64) * 100.0
        } else {
            0.0
        };

        (cpu_percent, utime, stime, now_ns)
    } else {
        (0.0, utime, stime, now_ns)
    };

    Some(ProcessMetrics {
        comm,
        cpu_percent,
        memory_rss_bytes,
        memory_vms_bytes,
        last_updated_ns: now_ns,
        prev_utime,
        prev_stime,
        prev_time_ns,
    })
}

/// Read process metrics from /proc/[pid]/stat and /proc/[pid]/statm (without previous data)
#[cfg(target_os = "linux")]
pub fn read_process_metrics(pid: u32) -> Option<ProcessMetrics> {
    read_process_metrics_with_prev(pid, None)
}

/// Non-Linux stub
#[cfg(not(target_os = "linux"))]
pub fn read_process_metrics(_pid: u32) -> Option<ProcessMetrics> {
    None
}

/// Non-Linux stub
#[cfg(not(target_os = "linux"))]
pub fn read_process_metrics_with_prev(
    _pid: u32,
    _prev: Option<ProcessMetrics>,
) -> Option<ProcessMetrics> {
    None
}

/// Shared metrics instance
pub type SharedMetrics = Arc<MetricsCollector>;

/// Create a new shared metrics collector
pub fn create_metrics() -> SharedMetrics {
    Arc::new(MetricsCollector::new())
}
