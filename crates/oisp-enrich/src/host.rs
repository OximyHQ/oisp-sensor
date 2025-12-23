//! Host information enrichment

use async_trait::async_trait;
use oisp_core::events::Host;
use oisp_core::events::OispEvent;
use oisp_core::plugins::{EnrichPlugin, Plugin, PluginInfo, PluginResult};
use std::any::Any;
use std::sync::OnceLock;

static HOST_INFO: OnceLock<Host> = OnceLock::new();

/// Host enricher
pub struct HostEnricher;

impl HostEnricher {
    pub fn new() -> Self {
        // Initialize host info once
        HOST_INFO.get_or_init(|| {
            let hostname = hostname::get()
                .map(|h: std::ffi::OsString| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string());

            let os_version = get_os_version();

            Host {
                hostname,
                device_id: get_device_id(),
                os: Some(std::env::consts::OS.to_string()),
                os_version,
                arch: Some(std::env::consts::ARCH.to_string()),
            }
        });

        Self
    }
}

fn get_os_version() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|line| line.starts_with("PRETTY_NAME="))
                    .map(|line| {
                        line.trim_start_matches("PRETTY_NAME=")
                            .trim_matches('"')
                            .to_string()
                    })
            })
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .map(|v| format!("macOS {}", v))
    }

    #[cfg(target_os = "windows")]
    {
        Some("Windows".to_string())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

fn get_device_id() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/etc/machine-id")
            .ok()
            .map(|s| s.trim().to_string())
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
            .ok()
            .and_then(|o| {
                let output = String::from_utf8_lossy(&o.stdout);
                output
                    .lines()
                    .find(|line| line.contains("IOPlatformUUID"))
                    .and_then(|line| line.split('"').nth(3).map(String::from))
            })
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

impl Default for HostEnricher {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginInfo for HostEnricher {
    fn name(&self) -> &str {
        "host-enricher"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Enriches events with host information"
    }
}

impl Plugin for HostEnricher {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl EnrichPlugin for HostEnricher {
    async fn enrich(&self, event: &mut OispEvent) -> PluginResult<()> {
        let envelope = match event {
            OispEvent::AiRequest(e) => &mut e.envelope,
            OispEvent::AiResponse(e) => &mut e.envelope,
            OispEvent::ProcessExec(e) => &mut e.envelope,
            OispEvent::NetworkConnect(e) => &mut e.envelope,
            OispEvent::FileWrite(e) => &mut e.envelope,
            _ => return Ok(()),
        };

        if envelope.host.is_none() {
            envelope.host = HOST_INFO.get().cloned();
        }

        Ok(())
    }
}
