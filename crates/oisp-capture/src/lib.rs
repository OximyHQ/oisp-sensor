//! Capture abstraction layer
//!
//! Provides a unified interface for platform-specific capture implementations.

use oisp_core::plugins::{CapturePlugin, RawCaptureEvent, PluginResult};
use tokio::sync::mpsc;

pub mod filter;

/// Capture configuration
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Enable SSL/TLS capture
    pub ssl: bool,
    
    /// Enable process capture
    pub process: bool,
    
    /// Enable file capture
    pub file: bool,
    
    /// Enable network capture
    pub network: bool,
    
    /// Process name filter (empty = all)
    pub process_filter: Vec<String>,
    
    /// PID filter (empty = all)
    pub pid_filter: Vec<u32>,
    
    /// Additional binary paths for SSL detection
    pub ssl_binary_paths: Vec<String>,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            ssl: true,
            process: true,
            file: true,
            network: true,
            process_filter: Vec::new(),
            pid_filter: Vec::new(),
            ssl_binary_paths: Vec::new(),
        }
    }
}

/// Create platform-appropriate capture plugins
pub fn create_capture_plugins(_config: &CaptureConfig) -> Vec<Box<dyn CapturePlugin>> {
    let plugins: Vec<Box<dyn CapturePlugin>> = Vec::new();
    
    #[cfg(target_os = "linux")]
    {
        // Use eBPF on Linux
        // plugins.push(Box::new(oisp_capture_ebpf::EbpfCapture::new(config)));
    }
    
    #[cfg(target_os = "macos")]
    {
        // Use ESF/Network Extension on macOS
        // plugins.push(Box::new(oisp_capture_macos::MacOSCapture::new(config)));
    }
    
    #[cfg(target_os = "windows")]
    {
        // Use ETW on Windows
        // plugins.push(Box::new(oisp_capture_windows::WindowsCapture::new(config)));
    }
    
    plugins
}

/// Unified capture manager
pub struct CaptureManager {
    #[allow(dead_code)]
    config: CaptureConfig,
    plugins: Vec<Box<dyn CapturePlugin>>,
}

impl CaptureManager {
    pub fn new(config: CaptureConfig) -> Self {
        let plugins = create_capture_plugins(&config);
        Self { config, plugins }
    }
    
    pub fn add_plugin(&mut self, plugin: Box<dyn CapturePlugin>) {
        self.plugins.push(plugin);
    }
    
    pub async fn start(&mut self, tx: mpsc::Sender<RawCaptureEvent>) -> PluginResult<()> {
        for plugin in &mut self.plugins {
            plugin.start(tx.clone()).await?;
        }
        Ok(())
    }
    
    pub async fn stop(&mut self) -> PluginResult<()> {
        for plugin in &mut self.plugins {
            plugin.stop().await?;
        }
        Ok(())
    }
}

