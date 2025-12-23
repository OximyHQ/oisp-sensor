//! Process tree enrichment

use async_trait::async_trait;
use oisp_core::events::OispEvent;
use oisp_core::plugins::{EnrichPlugin, Plugin, PluginInfo, PluginResult};
use std::any::Any;
use std::collections::HashMap;
use std::sync::RwLock;

/// Process tree enricher
pub struct ProcessTreeEnricher {
    /// Cache of process info by PID
    #[allow(dead_code)]
    process_cache: RwLock<HashMap<u32, CachedProcess>>,
}

#[derive(Debug, Clone)]
struct CachedProcess {
    ppid: Option<u32>,
    exe: Option<String>,
    name: Option<String>,
    cmdline: Option<String>,
}

impl ProcessTreeEnricher {
    pub fn new() -> Self {
        Self {
            process_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Get process info from /proc (Linux) or equivalent
    fn get_process_info(&self, pid: u32) -> Option<CachedProcess> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            let proc_path = format!("/proc/{}", pid);

            let ppid = fs::read_to_string(format!("{}/stat", proc_path))
                .ok()
                .and_then(|stat| {
                    let parts: Vec<&str> = stat.split_whitespace().collect();
                    parts.get(3).and_then(|s| s.parse().ok())
                });

            let exe = fs::read_link(format!("{}/exe", proc_path))
                .ok()
                .map(|p| p.to_string_lossy().to_string());

            let cmdline = fs::read_to_string(format!("{}/cmdline", proc_path))
                .ok()
                .map(|s| s.replace('\0', " ").trim().to_string());

            let name = fs::read_to_string(format!("{}/comm", proc_path))
                .ok()
                .map(|s| s.trim().to_string());

            Some(CachedProcess {
                ppid,
                exe,
                name,
                cmdline,
            })
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = pid;
            None
        }
    }

    /// Build the process tree for a given PID
    pub fn get_process_tree(&self, pid: u32) -> Vec<u32> {
        let mut tree = vec![pid];
        let mut current = pid;
        let mut seen = std::collections::HashSet::new();

        while let Some(info) = self.get_process_info(current) {
            if let Some(ppid) = info.ppid {
                if ppid == 0 || ppid == 1 || seen.contains(&ppid) {
                    break;
                }
                seen.insert(ppid);
                tree.push(ppid);
                current = ppid;
            } else {
                break;
            }
        }

        tree
    }
}

impl Default for ProcessTreeEnricher {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginInfo for ProcessTreeEnricher {
    fn name(&self) -> &str {
        "process-tree-enricher"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Enriches events with process tree information"
    }
}

impl Plugin for ProcessTreeEnricher {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl EnrichPlugin for ProcessTreeEnricher {
    async fn enrich(&self, event: &mut OispEvent) -> PluginResult<()> {
        let envelope = match event {
            OispEvent::AiRequest(e) => &mut e.envelope,
            OispEvent::AiResponse(e) => &mut e.envelope,
            OispEvent::ProcessExec(e) => &mut e.envelope,
            OispEvent::NetworkConnect(e) => &mut e.envelope,
            _ => return Ok(()),
        };

        if let Some(proc) = &mut envelope.process {
            // Enrich with parent info if missing
            if proc.ppid.is_none() {
                if let Some(info) = self.get_process_info(proc.pid) {
                    proc.ppid = info.ppid;
                    if proc.exe.is_none() {
                        proc.exe = info.exe;
                    }
                    if proc.name.is_none() {
                        proc.name = info.name;
                    }
                    if proc.cmdline.is_none() {
                        proc.cmdline = info.cmdline;
                    }
                }
            }
        }

        Ok(())
    }
}
