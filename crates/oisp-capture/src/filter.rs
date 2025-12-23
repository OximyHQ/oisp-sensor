//! Event filtering for capture

use oisp_core::plugins::RawCaptureEvent;

/// Filter configuration
#[derive(Debug, Clone, Default)]
pub struct CaptureFilter {
    /// Process names to include (empty = all)
    pub include_comms: Vec<String>,

    /// Process names to exclude
    pub exclude_comms: Vec<String>,

    /// PIDs to include (empty = all)
    pub include_pids: Vec<u32>,

    /// PIDs to exclude
    pub exclude_pids: Vec<u32>,

    /// Paths to include (prefix match)
    pub include_paths: Vec<String>,

    /// Paths to exclude (prefix match)
    pub exclude_paths: Vec<String>,
}

impl CaptureFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if an event should be captured
    pub fn should_capture(&self, event: &RawCaptureEvent) -> bool {
        // Check PID filters
        if !self.include_pids.is_empty() && !self.include_pids.contains(&event.pid) {
            return false;
        }
        if self.exclude_pids.contains(&event.pid) {
            return false;
        }

        // Check comm filters
        if let Some(comm) = &event.metadata.comm {
            if !self.include_comms.is_empty()
                && !self.include_comms.iter().any(|c| comm.contains(c))
            {
                return false;
            }
            if self.exclude_comms.iter().any(|c| comm.contains(c)) {
                return false;
            }
        }

        // Check path filters
        if let Some(path) = &event.metadata.path {
            if !self.include_paths.is_empty()
                && !self.include_paths.iter().any(|p| path.starts_with(p))
            {
                return false;
            }
            if self.exclude_paths.iter().any(|p| path.starts_with(p)) {
                return false;
            }
        }

        true
    }

    /// Add a process name to include
    pub fn include_comm(mut self, comm: impl Into<String>) -> Self {
        self.include_comms.push(comm.into());
        self
    }

    /// Add a process name to exclude
    pub fn exclude_comm(mut self, comm: impl Into<String>) -> Self {
        self.exclude_comms.push(comm.into());
        self
    }

    /// Add a PID to include
    pub fn include_pid(mut self, pid: u32) -> Self {
        self.include_pids.push(pid);
        self
    }

    /// Add a PID to exclude
    pub fn exclude_pid(mut self, pid: u32) -> Self {
        self.exclude_pids.push(pid);
        self
    }
}
