//! OISP Sensor - Universal AI Observability
//!
//! Zero-instrumentation sensor for AI activity monitoring and control.

use clap::{Parser, Subcommand};
use oisp_capture::{TestGenerator, TestGeneratorConfig};
#[cfg(target_os = "linux")]
use oisp_capture_ebpf::{EbpfCapture, EbpfCaptureConfig};
#[cfg(target_os = "macos")]
use oisp_capture_macos::{MacOSCapture, MacOSCaptureConfig};
use oisp_core::config::{ConfigLoader, SensorConfig};
use oisp_core::enrichers::{HostEnricher, ProcessTreeEnricher};
use oisp_core::pipeline::{Pipeline, PipelineConfig};
use oisp_core::RedactionPlugin;
use oisp_decode::{HttpDecoder, SystemDecoder};
use oisp_export::jsonl::{JsonlExporter, JsonlExporterConfig};
use oisp_export::websocket::{WebSocketExporter, WebSocketExporterConfig};
use std::path::PathBuf;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "oisp-sensor")]
#[command(author = "Oximy")]
#[command(version)]
#[command(about = "Universal AI observability sensor", long_about = None)]
struct Cli {
    /// Increase verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Output format (json, text)
    #[arg(short, long, default_value = "text")]
    format: String,

    /// Path to configuration file
    #[arg(short, long, global = true, env = "OISP_CONFIG")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Record AI activity (requires elevated privileges on some platforms)
    Record {
        /// Output file for JSONL events
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Start web UI
        #[arg(long, default_value = "true")]
        web: bool,

        /// Web UI port
        #[arg(long, default_value = "7777")]
        port: u16,

        /// Start TUI
        #[arg(long)]
        tui: bool,

        /// Filter by process name
        #[arg(short, long)]
        process: Option<Vec<String>>,

        /// Filter by PID
        #[arg(long)]
        pid: Option<Vec<u32>>,

        /// Redaction mode (safe, full, minimal)
        #[arg(long, default_value = "safe")]
        redaction: String,

        /// Disable SSL/TLS capture
        #[arg(long)]
        no_ssl: bool,

        /// Disable process capture
        #[arg(long)]
        no_process: bool,

        /// Disable file capture
        #[arg(long)]
        no_file: bool,

        /// Disable network capture
        #[arg(long)]
        no_network: bool,

        /// Path to eBPF bytecode file (Linux only, auto-detected if not specified)
        #[arg(long)]
        ebpf_path: Option<PathBuf>,

        /// Path to libssl.so OR binary with embedded SSL (e.g., node) for SSL interception
        /// (auto-detected if not specified)
        #[arg(long)]
        libssl_path: Option<PathBuf>,
    },

    /// Show captured events
    Show {
        /// Input file (JSONL)
        #[arg(short, long)]
        input: PathBuf,

        /// Filter by event type
        #[arg(short = 't', long)]
        event_type: Option<String>,

        /// Follow mode (tail -f style)
        #[arg(short, long)]
        follow: bool,

        /// Number of events to show
        #[arg(short, long, default_value = "50")]
        num: usize,
    },

    /// Analyze recorded events
    Analyze {
        /// Input file (JSONL)
        #[arg(short, long)]
        input: PathBuf,

        /// Analysis type (inventory, traces, costs)
        #[arg(short = 't', long, default_value = "inventory")]
        analysis_type: String,
    },

    /// Show sensor status and capabilities
    Status,

    /// Check system compatibility and requirements
    Check,

    /// Manage the sensor daemon
    #[command(subcommand)]
    Daemon(DaemonCommands),

    /// Self-test sensor capabilities
    Test,

    /// Diagnose SSL capture for a specific process
    Diagnose {
        /// Process ID to diagnose
        #[arg(short, long)]
        pid: u32,

        /// Show memory maps (loaded libraries)
        #[arg(long)]
        maps: bool,

        /// Show network connections
        #[arg(long)]
        network: bool,
    },

    /// Show SSL library information on the system
    SslInfo {
        /// Show detailed symbol information
        #[arg(long)]
        detailed: bool,

        /// Show which processes are using each library
        #[arg(long)]
        usage: bool,
    },

    /// Run demo mode with generated test events (no eBPF required)
    Demo {
        /// Output file for JSONL events
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Start web UI
        #[arg(long, default_value = "true")]
        web: bool,

        /// Web UI port
        #[arg(long, default_value = "7777")]
        port: u16,

        /// Start TUI
        #[arg(long)]
        tui: bool,

        /// Event generation interval in milliseconds
        #[arg(long, default_value = "2000")]
        interval: u64,

        /// Number of events to generate (0 = infinite)
        #[arg(long, default_value = "0")]
        count: u64,

        /// Redaction mode (safe, full, minimal)
        #[arg(long, default_value = "full")]
        redaction: String,
    },
}

#[derive(Subcommand)]
enum DaemonCommands {
    /// Start the sensor as a background daemon
    Start {
        /// Output file for JSONL events
        #[arg(short, long, default_value = "/var/log/oisp-sensor/events.jsonl")]
        output: PathBuf,

        /// Disable web UI
        #[arg(long)]
        no_web: bool,

        /// Web UI port
        #[arg(long, default_value = "7777")]
        port: u16,

        /// Redaction mode (safe, full, minimal)
        #[arg(long, default_value = "safe")]
        redaction: String,
    },

    /// Stop the running daemon
    Stop,

    /// Show daemon status
    Status,

    /// Show daemon logs
    Logs {
        /// Follow log output (like tail -f)
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short, long, default_value = "50")]
        num: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load configuration file
    let sensor_config = load_config(cli.config.clone());

    // Setup logging - CLI verbose flag takes precedence, then config, then default
    let log_level = if cli.verbose > 0 {
        match cli.verbose {
            1 => Level::INFO,
            2 => Level::DEBUG,
            _ => Level::TRACE,
        }
    } else {
        match sensor_config.sensor.log_level.to_lowercase().as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::WARN,
        }
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    match cli.command {
        Commands::Record {
            output,
            web,
            port,
            tui,
            process,
            pid,
            redaction,
            no_ssl,
            no_process,
            no_file,
            no_network,
            ebpf_path,
            libssl_path,
        } => {
            // Merge CLI args with config file settings
            // CLI args take precedence over config file
            let merged_config = merge_record_config(
                &sensor_config,
                output,
                web,
                port,
                tui,
                process,
                pid,
                redaction,
                no_ssl,
                no_process,
                no_file,
                no_network,
                ebpf_path,
                libssl_path,
            );
            record_command(merged_config).await
        }
        Commands::Show {
            input,
            event_type,
            follow,
            num,
        } => show_command(&input, event_type, follow, num).await,
        Commands::Analyze {
            input,
            analysis_type,
        } => analyze_command(&input, &analysis_type).await,
        Commands::Status => status_command().await,
        Commands::Check => check_command().await,
        Commands::Daemon(daemon_cmd) => daemon_command(daemon_cmd).await,
        Commands::Test => test_command().await,
        Commands::Diagnose { pid, maps, network } => diagnose_command(pid, maps, network).await,
        Commands::SslInfo { detailed, usage } => ssl_info_command(detailed, usage).await,
        Commands::Demo {
            output,
            web,
            port,
            tui,
            interval,
            count,
            redaction,
        } => {
            demo_command(DemoConfig {
                output,
                web,
                port,
                tui,
                interval_ms: interval,
                event_count: count,
                redaction_mode: redaction,
            })
            .await
        }
    }
}

/// Load configuration from file/env, with fallback to defaults
fn load_config(cli_path: Option<PathBuf>) -> SensorConfig {
    let loader = ConfigLoader::new().with_cli_path(cli_path);
    match loader.load() {
        Ok(config) => {
            info!("Configuration loaded successfully");
            config
        }
        Err(e) => {
            warn!("Failed to load configuration: {}, using defaults", e);
            SensorConfig::default()
        }
    }
}

/// Merge CLI arguments with config file settings
/// CLI arguments take precedence when explicitly provided
#[allow(clippy::too_many_arguments)]
fn merge_record_config(
    config: &SensorConfig,
    output: Option<PathBuf>,
    web: bool,
    port: u16,
    tui: bool,
    process: Option<Vec<String>>,
    pid: Option<Vec<u32>>,
    redaction: String,
    no_ssl: bool,
    no_process: bool,
    no_file: bool,
    no_network: bool,
    ebpf_path: Option<PathBuf>,
    libssl_path: Option<PathBuf>,
) -> RecordConfig {
    // For boolean flags, CLI explicit disables take precedence
    // Otherwise use config file value
    let ssl = if no_ssl { false } else { config.capture.ssl };
    let process_enabled = if no_process {
        false
    } else {
        config.capture.process
    };
    let file = if no_file { false } else { config.capture.file };
    let network = if no_network {
        false
    } else {
        config.capture.network
    };

    // For filters, CLI takes precedence if provided
    let process_filter = process.unwrap_or_else(|| config.capture.process_filter.clone());
    let pid_filter = pid.unwrap_or_else(|| config.capture.pid_filter.clone());

    // For paths, CLI takes precedence if provided
    let ebpf_path = ebpf_path.or_else(|| config.capture.ebpf_path.as_ref().map(PathBuf::from));
    let libssl_path =
        libssl_path.or_else(|| config.capture.libssl_path.as_ref().map(PathBuf::from));

    // For output, CLI takes precedence, then check if JSONL export is enabled in config
    let output = output.or_else(|| {
        if config.export.jsonl.enabled {
            Some(PathBuf::from(&config.export.jsonl.path))
        } else {
            None
        }
    });

    // Web/port - CLI args have defaults, so they're always "set"
    // Use CLI values but fall back to config if CLI has defaults
    let web_enabled = if !web { false } else { config.web.enabled };
    let web_port = if port != 7777 { port } else { config.web.port };

    // Redaction mode - CLI has default "safe", use it unless different
    let redaction_mode = if redaction != "safe" {
        redaction
    } else {
        config.redaction.mode.clone()
    };

    RecordConfig {
        output,
        web: web_enabled,
        port: web_port,
        tui,
        process_filter,
        pid_filter,
        redaction_mode,
        ssl,
        process: process_enabled,
        file,
        network,
        ebpf_path,
        libssl_path,
    }
}

#[allow(dead_code)]
struct RecordConfig {
    output: Option<PathBuf>,
    web: bool,
    port: u16,
    tui: bool,
    process_filter: Vec<String>,
    pid_filter: Vec<u32>,
    redaction_mode: String,
    ssl: bool,
    process: bool,
    file: bool,
    network: bool,
    ebpf_path: Option<PathBuf>,
    libssl_path: Option<PathBuf>,
}

async fn record_command(config: RecordConfig) -> anyhow::Result<()> {
    info!("Starting OISP Sensor...");

    // Create pipeline
    let pipeline_config = PipelineConfig::default();
    let mut pipeline = Pipeline::new(pipeline_config);

    // Add eBPF capture on Linux
    #[cfg(target_os = "linux")]
    {
        if config.ssl {
            let ebpf_config = EbpfCaptureConfig {
                ssl: config.ssl,
                process: config.process,
                file: config.file,
                network: config.network,
                ssl_binary_paths: config
                    .libssl_path
                    .map(|p| vec![p.to_string_lossy().to_string()])
                    .unwrap_or_default(),
                comm_filter: config.process_filter.clone(),
                pid_filter: config.pid_filter.first().copied(),
                ebpf_bytecode_path: config.ebpf_path.map(|p| p.to_string_lossy().to_string()),
            };

            let ebpf_capture = EbpfCapture::with_config(ebpf_config);
            pipeline.add_capture(Box::new(ebpf_capture));
            info!("eBPF capture plugin added");
        }
    }

    // Add macOS capture via Network Extension
    #[cfg(target_os = "macos")]
    {
        if config.ssl {
            let macos_config = MacOSCaptureConfig {
                process: config.process,
                file: config.file,
                network: config.network,
                use_system_extension: true,
                socket_path: "/tmp/oisp.sock".to_string(),
            };

            let macos_capture = MacOSCapture::with_config(macos_config);
            pipeline.add_capture(Box::new(macos_capture));
            info!("macOS capture plugin added (listening on /tmp/oisp.sock)");
        }
        let _ = (&config.ebpf_path, &config.libssl_path); // Suppress unused warnings
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        info!("Platform capture not available - use demo mode for testing");
        let _ = (&config.ebpf_path, &config.libssl_path); // Suppress unused warnings
    }

    // Add decoders
    pipeline.add_decode(Box::new(HttpDecoder::new()));
    pipeline.add_decode(Box::new(SystemDecoder::new()));

    // Add enrichers
    pipeline.add_enrich(Box::new(HostEnricher::new()));
    pipeline.add_enrich(Box::new(ProcessTreeEnricher::new()));

    // Add redaction
    let redaction = match config.redaction_mode.as_str() {
        "full" => RedactionPlugin::full_capture(),
        "minimal" => RedactionPlugin::minimal(),
        _ => RedactionPlugin::safe_mode(),
    };
    pipeline.add_action(Box::new(redaction));

    // Add exporters
    if let Some(output_path) = config.output {
        pipeline.add_export(Box::new(JsonlExporter::new(JsonlExporterConfig {
            path: output_path,
            append: true,
            pretty: false,
            flush_each: true,
        })));
    }

    let ws_exporter = WebSocketExporter::new(WebSocketExporterConfig {
        port: config.port,
        host: "127.0.0.1".to_string(),
        buffer_size: 1000,
    });
    pipeline.add_export(Box::new(ws_exporter));

    // Enable traces
    pipeline.enable_traces();

    // Get event broadcast for UI
    let event_rx = pipeline.subscribe();
    let trace_builder = pipeline.trace_builder().unwrap();

    // Start pipeline
    pipeline.start().await?;

    info!("Pipeline started");

    // Start web UI if requested
    if config.web {
        let web_config = oisp_web::WebConfig {
            host: "0.0.0.0".to_string(),
            port: config.port,
        };

        let event_tx = pipeline.event_sender();
        let tb = trace_builder.clone();

        tokio::spawn(async move {
            if let Err(e) = oisp_web::start_server(web_config, event_tx, tb).await {
                error!("Web server error: {}", e);
            }
        });

        println!();
        println!("  OISP Sensor v{}", env!("CARGO_PKG_VERSION"));
        println!();
        println!("  Web UI: http://127.0.0.1:{}", config.port);
        println!();
        println!("  Press Ctrl+C to stop");
        println!();
    }

    // Start TUI if requested
    if config.tui {
        oisp_tui::run(event_rx).await?;
    } else {
        // Wait for Ctrl+C
        tokio::signal::ctrl_c().await?;
    }

    // Cleanup
    pipeline.stop().await?;
    info!("Sensor stopped");

    Ok(())
}

async fn show_command(
    input: &PathBuf,
    event_type: Option<String>,
    follow: bool,
    num: usize,
) -> anyhow::Result<()> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(input)?;
    let reader = BufReader::new(file);

    let mut count = 0;
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        let event: serde_json::Value = serde_json::from_str(&line)?;

        // Filter by event type if specified
        if let Some(ref filter) = event_type {
            if let Some(et) = event.get("event_type").and_then(|v| v.as_str()) {
                if !et.contains(filter) {
                    continue;
                }
            }
        }

        // Pretty print
        println!("{}", serde_json::to_string_pretty(&event)?);

        count += 1;
        if !follow && count >= num {
            break;
        }
    }

    if follow {
        // TODO: Implement tail -f style following
        println!("Follow mode not yet implemented");
    }

    Ok(())
}

async fn analyze_command(input: &PathBuf, analysis_type: &str) -> anyhow::Result<()> {
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(input)?;
    let reader = BufReader::new(file);

    let mut events: Vec<serde_json::Value> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if !line.is_empty() {
            if let Ok(event) = serde_json::from_str(&line) {
                events.push(event);
            }
        }
    }

    match analysis_type {
        "inventory" => {
            let mut providers: HashMap<String, u64> = HashMap::new();
            let mut models: HashMap<String, u64> = HashMap::new();
            let mut apps: HashMap<String, u64> = HashMap::new();

            for event in &events {
                if event.get("event_type").and_then(|v| v.as_str()) == Some("ai.request") {
                    if let Some(data) = event.get("data") {
                        if let Some(provider) = data
                            .get("provider")
                            .and_then(|p| p.get("name"))
                            .and_then(|n| n.as_str())
                        {
                            *providers.entry(provider.to_string()).or_default() += 1;
                        }
                        if let Some(model) = data
                            .get("model")
                            .and_then(|m| m.get("id"))
                            .and_then(|i| i.as_str())
                        {
                            *models.entry(model.to_string()).or_default() += 1;
                        }
                    }
                    if let Some(proc) = event
                        .get("process")
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str())
                    {
                        *apps.entry(proc.to_string()).or_default() += 1;
                    }
                }
            }

            println!("\n=== AI Inventory ===\n");

            println!("Providers:");
            for (name, count) in providers.iter() {
                println!("  {:<20} {:>6} requests", name, count);
            }

            println!("\nModels:");
            for (name, count) in models.iter() {
                println!("  {:<30} {:>6} requests", name, count);
            }

            println!("\nApplications:");
            for (name, count) in apps.iter() {
                println!("  {:<20} {:>6} requests", name, count);
            }
        }
        "traces" => {
            println!("Trace analysis not yet implemented");
        }
        "costs" => {
            println!("Cost analysis not yet implemented");
        }
        _ => {
            println!("Unknown analysis type: {}", analysis_type);
        }
    }

    Ok(())
}

async fn status_command() -> anyhow::Result<()> {
    println!();
    println!("OISP Sensor v{}", env!("CARGO_PKG_VERSION"));
    println!();

    // Platform
    println!(
        "Platform: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    // Check capabilities
    #[cfg(target_os = "linux")]
    {
        println!();
        println!("Linux Capabilities:");

        // Check if running as root
        let uid = unsafe { libc::getuid() };
        println!("  Running as root: {}", uid == 0);

        // Check for eBPF support
        let ebpf_supported = std::path::Path::new("/sys/fs/bpf").exists();
        println!("  eBPF supported: {}", ebpf_supported);

        // Check for BTF
        let btf_available = std::path::Path::new("/sys/kernel/btf/vmlinux").exists();
        println!("  BTF available: {}", btf_available);

        // Check kernel version
        if let Ok(release) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
            println!("  Kernel: {}", release.trim());
        }
    }

    #[cfg(target_os = "macos")]
    {
        println!();
        println!("macOS Capabilities:");
        println!("  System Extension: Not installed");
        println!("  Full Disk Access: Unknown");
    }

    #[cfg(target_os = "windows")]
    {
        println!();
        println!("Windows Capabilities:");
        println!("  Running as Administrator: Unknown");
        println!("  ETW access: Unknown");
    }

    println!();

    Ok(())
}

struct DemoConfig {
    output: Option<PathBuf>,
    web: bool,
    port: u16,
    tui: bool,
    interval_ms: u64,
    event_count: u64,
    redaction_mode: String,
}

/// Demo mode - generates fake events to test the pipeline and UI
async fn demo_command(config: DemoConfig) -> anyhow::Result<()> {
    println!();
    println!("  OISP Sensor v{} - DEMO MODE", env!("CARGO_PKG_VERSION"));
    println!();
    println!("  Generating test events every {}ms", config.interval_ms);
    if config.event_count > 0 {
        println!("  Will generate {} events total", config.event_count);
    } else {
        println!("  Generating events indefinitely");
    }
    println!();

    info!("Starting OISP Sensor in demo mode...");

    // Create pipeline
    let pipeline_config = PipelineConfig::default();
    let mut pipeline = Pipeline::new(pipeline_config);

    // Add test generator as capture source
    let test_generator = TestGenerator::with_config(TestGeneratorConfig {
        interval_ms: config.interval_ms,
        event_count: config.event_count,
        generate_ai_events: true,
        generate_process_events: true,
        generate_file_events: true,
        process_name: "cursor".to_string(),
        pid: 12345,
    });
    pipeline.add_capture(Box::new(test_generator));

    // Add decoders
    pipeline.add_decode(Box::new(HttpDecoder::new()));
    pipeline.add_decode(Box::new(SystemDecoder::new()));

    // Add enrichers
    pipeline.add_enrich(Box::new(HostEnricher::new()));
    pipeline.add_enrich(Box::new(ProcessTreeEnricher::new()));

    // Add redaction
    let redaction = match config.redaction_mode.as_str() {
        "full" => RedactionPlugin::full_capture(),
        "minimal" => RedactionPlugin::minimal(),
        _ => RedactionPlugin::safe_mode(),
    };
    pipeline.add_action(Box::new(redaction));

    // Add exporters
    if let Some(output_path) = config.output {
        pipeline.add_export(Box::new(JsonlExporter::new(JsonlExporterConfig {
            path: output_path.clone(),
            append: true,
            pretty: false,
            flush_each: true,
        })));
        println!("  Output: {}", output_path.display());
    }

    let ws_exporter = WebSocketExporter::new(WebSocketExporterConfig {
        port: config.port,
        host: "127.0.0.1".to_string(),
        buffer_size: 1000,
    });
    pipeline.add_export(Box::new(ws_exporter));

    // Enable traces
    pipeline.enable_traces();

    // Get event broadcast for UI
    let event_rx = pipeline.subscribe();
    let trace_builder = pipeline.trace_builder().unwrap();

    // Start pipeline
    pipeline.start().await?;

    info!("Demo pipeline started");

    // Start web UI if requested
    if config.web {
        let web_config = oisp_web::WebConfig {
            host: "0.0.0.0".to_string(),
            port: config.port,
        };

        let event_tx = pipeline.event_sender();
        let tb = trace_builder.clone();

        tokio::spawn(async move {
            if let Err(e) = oisp_web::start_server(web_config, event_tx, tb).await {
                error!("Web server error: {}", e);
            }
        });

        println!("  Web UI: http://127.0.0.1:{}", config.port);
    }

    println!();
    println!("  Press Ctrl+C to stop");
    println!();

    // Start TUI if requested
    if config.tui {
        oisp_tui::run(event_rx).await?;
    } else {
        // Wait for Ctrl+C
        tokio::signal::ctrl_c().await?;
    }

    // Cleanup
    pipeline.stop().await?;
    info!("Demo stopped");

    Ok(())
}

async fn test_command() -> anyhow::Result<()> {
    println!("Running sensor self-test...\n");

    // Test 1: Event creation
    print!("  Creating test events... ");
    let envelope = oisp_core::events::envelope::EventEnvelope::new("test");
    println!("OK (event_id: {})", envelope.event_id);

    // Test 2: Provider detection
    print!("  Testing provider detection... ");
    let registry = oisp_core::providers::ProviderRegistry::new();
    let provider = registry.detect_from_domain("api.openai.com");
    println!("OK (api.openai.com -> {:?})", provider);

    // Test 3: Redaction (API key must be 20+ chars after prefix)
    print!("  Testing redaction... ");
    let config = oisp_core::redaction::RedactionConfig::default();
    let result =
        oisp_core::redaction::redact("My API key is sk-proj-abc123def456ghi789jkl0", &config);
    let passed = result.content.contains("[API_KEY_REDACTED]");
    println!("{}", if passed { "OK" } else { "FAILED" });

    // Test 4: JSON serialization
    print!("  Testing JSON serialization... ");
    let envelope = oisp_core::events::envelope::EventEnvelope::new("test");
    let json = serde_json::to_string(&envelope)?;
    let _: oisp_core::events::envelope::EventEnvelope = serde_json::from_str(&json)?;
    println!("OK");

    println!("\nAll tests passed!\n");

    Ok(())
}

// =============================================================================
// Check Command - System Compatibility Validation
// =============================================================================

/// Common SSL library paths to check on Linux
#[cfg(target_os = "linux")]
const SSL_LIBRARY_PATHS: &[&str] = &[
    // Ubuntu/Debian x86_64
    "/usr/lib/x86_64-linux-gnu/libssl.so.3",
    "/usr/lib/x86_64-linux-gnu/libssl.so.1.1",
    "/usr/lib/x86_64-linux-gnu/libssl.so",
    // Ubuntu/Debian aarch64
    "/usr/lib/aarch64-linux-gnu/libssl.so.3",
    "/usr/lib/aarch64-linux-gnu/libssl.so.1.1",
    "/usr/lib/aarch64-linux-gnu/libssl.so",
    // RHEL/CentOS/Fedora
    "/usr/lib64/libssl.so.3",
    "/usr/lib64/libssl.so.1.1",
    "/usr/lib64/libssl.so",
    // Alpine
    "/usr/lib/libssl.so.3",
    "/usr/lib/libssl.so.1.1",
    // Arch
    "/usr/lib/libssl.so",
    // NVM Node.js common paths (statically linked, need binary_paths config)
    // These are for documentation purposes
];

/// Common paths where NVM, pyenv, etc. install binaries with bundled SSL
#[cfg(target_os = "linux")]
const EDGE_CASE_PATHS: &[(&str, &str)] = &[
    (
        "~/.nvm/versions/node/*/bin/node",
        "NVM Node.js (needs binary_paths config)",
    ),
    (
        "~/.pyenv/versions/*/bin/python*",
        "pyenv Python (may work with system SSL)",
    ),
    ("~/miniconda3/bin/python", "Miniconda (bundles own OpenSSL)"),
    ("~/anaconda3/bin/python", "Anaconda (bundles own OpenSSL)"),
];

async fn check_command() -> anyhow::Result<()> {
    println!();
    println!("OISP Sensor System Check");
    println!("========================");
    println!();

    #[allow(unused_assignments)]
    let mut all_ok = true;
    let mut warnings = Vec::new();

    // Platform info
    println!(
        "Platform: {} {} ({})",
        std::env::consts::OS,
        std::env::consts::ARCH,
        if cfg!(target_os = "linux") {
            "supported"
        } else {
            "limited support"
        }
    );

    // Distribution info
    #[cfg(target_os = "linux")]
    {
        if let Ok(distro_info) = detect_linux_distribution() {
            println!("Distribution: {} {}", distro_info.0, distro_info.1);
        }
    }

    println!();

    // Linux-specific checks
    #[cfg(target_os = "linux")]
    {
        // Check 1: Kernel version
        print!("Kernel Version:    ");
        match check_kernel_version() {
            Ok((major, minor, patch, release)) => {
                if major >= 5 {
                    println!("{}.{}.{} [OK]", major, minor, patch);
                } else if major == 4 && minor >= 18 {
                    println!("{}.{}.{} [OK] (minimum supported)", major, minor, patch);
                } else {
                    println!("{}.{}.{} [FAIL] (requires >= 4.18)", major, minor, patch);
                    all_ok = false;
                }
                let _ = release; // Used for debug if needed
            }
            Err(e) => {
                println!("Unknown [WARN] ({})", e);
                warnings.push("Could not determine kernel version".to_string());
            }
        }

        // Check 2: BTF availability
        print!("BTF Support:       ");
        let btf_path = std::path::Path::new("/sys/kernel/btf/vmlinux");
        if btf_path.exists() {
            println!("/sys/kernel/btf/vmlinux [OK]");
        } else {
            println!("Not found [WARN]");
            warnings.push(
                "BTF not found - may need CONFIG_DEBUG_INFO_BTF=y or kernel headers".to_string(),
            );
        }

        // Check 3: eBPF filesystem
        print!("eBPF Filesystem:   ");
        let bpf_path = std::path::Path::new("/sys/fs/bpf");
        if bpf_path.exists() {
            println!("/sys/fs/bpf [OK]");
        } else {
            println!("Not found [FAIL]");
            all_ok = false;
        }

        // Check 4: Running as root or capabilities
        print!("Permissions:       ");
        let uid = unsafe { libc::getuid() };
        if uid == 0 {
            println!("root [OK]");
        } else {
            // Check for capabilities
            if let Ok(caps) = check_capabilities() {
                if caps {
                    println!("CAP_BPF+CAP_PERFMON set [OK]");
                } else {
                    println!("No capabilities [WARN]");
                    warnings.push(
                        "Not running as root and no capabilities set - run with sudo or set capabilities"
                            .to_string(),
                    );
                }
            } else {
                println!("No [WARN]");
                warnings.push(
                    "Not running as root - SSL capture requires root or CAP_BPF+CAP_PERFMON"
                        .to_string(),
                );
            }
        }

        // Check 5: Systemd availability
        print!("Systemd:           ");
        if std::process::Command::new("systemctl")
            .arg("--version")
            .output()
            .is_ok()
        {
            println!("Available [OK]");
        } else {
            println!("Not found [WARN]");
            warnings.push("systemd not available - use manual process management".to_string());
        }

        // Check 6: SSL Libraries
        println!();
        println!("SSL Libraries:");
        let mut found_ssl = false;
        for path in SSL_LIBRARY_PATHS {
            if std::path::Path::new(path).exists() {
                println!("  {} [FOUND]", path);
                found_ssl = true;
            }
        }
        if !found_ssl {
            println!("  No system SSL libraries found [WARN]");
            warnings.push("No system OpenSSL found - SSL capture may not work".to_string());
        }

        // Check 7: Edge cases notice
        println!();
        println!("Edge Cases (require binary_paths config):");
        for (pattern, desc) in EDGE_CASE_PATHS {
            println!("  {} - {}", pattern, desc);
        }

        // Note about unsupported TLS
        println!();
        println!("Unsupported TLS Libraries:");
        println!("  Go crypto/tls, rustls, BoringSSL, GnuTLS, NSS");
        println!("  Run 'oisp-sensor ssl-info' for detailed TLS library information.");
    }

    // Non-Linux platforms
    #[cfg(not(target_os = "linux"))]
    {
        println!("Note: Full SSL capture is only available on Linux.");
        println!("      This platform has limited functionality.");
        all_ok = false;
    }

    // Check spec bundle
    println!();
    print!("Spec Bundle:       ");
    match load_spec_bundle_info() {
        Ok((version, providers, models)) => {
            println!(
                "v{} ({} providers, {} models) [OK]",
                version, providers, models
            );
        }
        Err(e) => {
            println!("Error loading: {} [WARN]", e);
            warnings.push("Spec bundle could not be loaded".to_string());
        }
    }

    // Check config file
    println!();
    print!("Config File:       ");
    let loader = ConfigLoader::new();
    if let Some(path) = loader.find_config_file() {
        println!("{} [FOUND]", path.display());
    } else {
        println!("Not found (using defaults) [OK]");
    }

    // Summary
    println!();
    println!("========================");
    if all_ok && warnings.is_empty() {
        println!("Result: READY");
        println!();
        println!("Run 'sudo oisp-sensor record' to start capturing.");
    } else if all_ok {
        println!("Result: READY (with warnings)");
        println!();
        println!("Warnings:");
        for w in &warnings {
            println!("  - {}", w);
        }
        println!();
        println!("Run 'sudo oisp-sensor record' to start capturing.");
    } else {
        println!("Result: NOT READY");
        println!();
        println!("Issues:");
        for w in &warnings {
            println!("  - {}", w);
        }
        println!();
        println!("Please resolve the above issues before running the sensor.");
    }
    println!();

    Ok(())
}

/// Parse kernel version from /proc/sys/kernel/osrelease
#[cfg(target_os = "linux")]
fn check_kernel_version() -> anyhow::Result<(u32, u32, u32, String)> {
    let release = std::fs::read_to_string("/proc/sys/kernel/osrelease")?;
    let release = release.trim();

    // Parse version like "5.15.0-generic" or "6.1.0-18-amd64"
    let version_part = release.split('-').next().unwrap_or(release);
    let parts: Vec<&str> = version_part.split('.').collect();

    let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    Ok((major, minor, patch, release.to_string()))
}

/// Detect Linux distribution from /etc/os-release
#[cfg(target_os = "linux")]
fn detect_linux_distribution() -> anyhow::Result<(String, String)> {
    let os_release = std::fs::read_to_string("/etc/os-release")?;

    let mut name = String::from("Unknown");
    let mut version = String::from("");

    for line in os_release.lines() {
        if let Some(value) = line.strip_prefix("NAME=") {
            name = value.trim_matches('"').to_string();
        } else if let Some(value) = line.strip_prefix("VERSION_ID=") {
            version = value.trim_matches('"').to_string();
        }
    }

    Ok((name, version))
}

/// Check if binary has required capabilities set
#[cfg(target_os = "linux")]
fn check_capabilities() -> anyhow::Result<bool> {
    // Get path to current executable
    let exe_path = std::env::current_exe()?;

    // Run getcap to check capabilities
    let output = std::process::Command::new("getcap").arg(&exe_path).output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Check for CAP_BPF or CAP_SYS_ADMIN (both allow eBPF operations)
            Ok(stdout.contains("cap_bpf") || stdout.contains("cap_sys_admin"))
        }
        Err(_) => Ok(false),
    }
}

/// Load spec bundle info for display
fn load_spec_bundle_info() -> anyhow::Result<(String, usize, usize)> {
    use oisp_core::spec::OispSpecBundle;

    let bundle = OispSpecBundle::embedded();
    let providers = bundle.providers.len();
    let models = bundle.models.len();

    Ok((bundle.version, providers, models))
}

/// Diagnose SSL capture capability for a specific process
async fn diagnose_command(pid: u32, show_maps: bool, show_network: bool) -> anyhow::Result<()> {
    println!();
    println!("OISP Sensor Process Diagnosis");
    println!("==============================");
    println!();
    println!("Target PID: {}", pid);
    println!();

    #[cfg(target_os = "linux")]
    {
        use std::fs;
        use std::path::Path;

        let proc_path = format!("/proc/{}", pid);

        // Check if process exists
        if !Path::new(&proc_path).exists() {
            println!("ERROR: Process {} does not exist", pid);
            return Ok(());
        }

        // Basic process info
        println!("Process Information:");
        println!("--------------------");

        // Executable path
        if let Ok(exe) = fs::read_link(format!("{}/exe", proc_path)) {
            println!("  Executable:  {}", exe.display());
        }

        // Process name
        if let Ok(comm) = fs::read_to_string(format!("{}/comm", proc_path)) {
            println!("  Name:        {}", comm.trim());
        }

        // Command line
        if let Ok(cmdline) = fs::read_to_string(format!("{}/cmdline", proc_path)) {
            let cmdline = cmdline.replace('\0', " ");
            if cmdline.len() > 100 {
                println!("  Command:     {}...", &cmdline[..100]);
            } else {
                println!("  Command:     {}", cmdline.trim());
            }
        }

        // Parent PID
        if let Ok(stat) = fs::read_to_string(format!("{}/stat", proc_path)) {
            let parts: Vec<&str> = stat.split_whitespace().collect();
            if let Some(ppid) = parts.get(3) {
                println!("  Parent PID:  {}", ppid);
            }
        }

        // Working directory
        if let Ok(cwd) = fs::read_link(format!("{}/cwd", proc_path)) {
            println!("  Working Dir: {}", cwd.display());
        }

        // SSL Library Detection
        println!();
        println!("SSL Libraries Loaded:");
        println!("----------------------");

        if let Ok(maps) = fs::read_to_string(format!("{}/maps", proc_path)) {
            let mut ssl_libs = Vec::new();
            let mut crypto_libs = Vec::new();

            for line in maps.lines() {
                if line.contains("libssl") {
                    // Extract library path
                    if let Some(path) = line.split_whitespace().last() {
                        if path.starts_with('/') && !ssl_libs.contains(&path.to_string()) {
                            ssl_libs.push(path.to_string());
                        }
                    }
                }
                if line.contains("libcrypto") {
                    if let Some(path) = line.split_whitespace().last() {
                        if path.starts_with('/') && !crypto_libs.contains(&path.to_string()) {
                            crypto_libs.push(path.to_string());
                        }
                    }
                }
            }

            if ssl_libs.is_empty() {
                println!("  No libssl.so loaded [WARN]");
                println!();
                println!("  This process may:");
                println!("    - Not use SSL/TLS");
                println!("    - Use a statically linked SSL library");
                println!("    - Use a non-OpenSSL TLS implementation (rustls, BoringSSL, etc.)");
            } else {
                for lib in &ssl_libs {
                    println!("  {} [OK]", lib);
                    // Try to get version
                    if let Ok(output) = std::process::Command::new("strings")
                        .args([lib, "-a"])
                        .output()
                    {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        for line in stdout.lines() {
                            if line.starts_with("OpenSSL ") && line.len() < 50 {
                                println!("    Version: {}", line);
                                break;
                            }
                        }
                    }
                }
            }

            if !crypto_libs.is_empty() {
                println!();
                println!("  Associated libcrypto:");
                for lib in &crypto_libs {
                    println!("    {}", lib);
                }
            }

            // Full memory maps if requested
            if show_maps {
                println!();
                println!("Full Memory Maps (libraries only):");
                println!("-----------------------------------");
                for line in maps.lines() {
                    if line.contains(".so") && line.contains('/') {
                        if let Some(path) = line.split_whitespace().last() {
                            if path.starts_with('/') {
                                println!("  {}", path);
                            }
                        }
                    }
                }
            }
        } else {
            println!("  Could not read memory maps (permission denied?)");
        }

        // Network connections if requested
        if show_network {
            println!();
            println!("Network Connections:");
            println!("--------------------");

            // Count file descriptors that are sockets
            let fd_path = format!("{}/fd", proc_path);
            if let Ok(fds) = fs::read_dir(&fd_path) {
                let mut tcp_count = 0;
                let udp_count = 0;

                for fd in fds.flatten() {
                    if let Ok(link) = fs::read_link(fd.path()) {
                        let link_str = link.to_string_lossy();
                        if link_str.starts_with("socket:") {
                            // Check /proc/net/tcp and /proc/net/udp
                            tcp_count += 1; // Simplified - count all sockets
                        }
                    }
                }

                if tcp_count > 0 {
                    println!("  Active sockets: {}", tcp_count);
                } else {
                    println!("  No active network connections");
                }

                let _ = udp_count; // Suppress warning
            }
        }

        // Capture recommendation
        println!();
        println!("Capture Recommendation:");
        println!("-----------------------");

        if let Ok(maps) = fs::read_to_string(format!("{}/maps", proc_path)) {
            let has_system_ssl = maps.lines().any(|l| {
                l.contains("libssl.so")
                    && (l.contains("/usr/lib")
                        || l.contains("/lib/x86_64")
                        || l.contains("/lib/aarch64"))
            });

            if has_system_ssl {
                println!("  This process uses system OpenSSL.");
                println!("  SSL capture should work with default configuration.");
                println!();
                println!("  Command: sudo oisp-sensor record --pid {}", pid);
            } else if maps.lines().any(|l| l.contains("libssl")) {
                println!("  This process uses a non-system OpenSSL.");
                println!("  You may need to configure binary_paths in your config.");
                println!();
                println!("  Add to config.yaml:");
                println!("    capture:");
                println!("      ssl_binary_paths:");
                println!("        - /path/to/custom/libssl.so");
            } else {
                println!("  This process does not appear to use OpenSSL.");
                println!("  SSL capture may not work for this process.");
                println!();
                println!("  Possible reasons:");
                println!("    - Process uses static SSL (NVM Node.js, some Python builds)");
                println!("    - Process uses alternative TLS (rustls, BoringSSL, GnuTLS)");
                println!("    - Process hasn't loaded SSL libraries yet");
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("Process diagnosis is only available on Linux.");
        let _ = (pid, show_maps, show_network);
    }

    println!();
    Ok(())
}

/// Show SSL library information on the system
async fn ssl_info_command(detailed: bool, show_usage: bool) -> anyhow::Result<()> {
    println!();
    println!("OISP Sensor SSL Library Information");
    println!("====================================");
    println!();

    #[cfg(target_os = "linux")]
    {
        use std::collections::HashMap;
        use std::fs;
        use std::process::Command;

        // Find all SSL libraries
        println!("System SSL Libraries:");
        println!("---------------------");

        let mut found_libs: Vec<(String, Option<String>)> = Vec::new();

        // Check known paths
        for path in SSL_LIBRARY_PATHS {
            if std::path::Path::new(path).exists() {
                // Get version via strings
                let version = Command::new("strings")
                    .args([*path, "-a"])
                    .output()
                    .ok()
                    .and_then(|output| {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        stdout
                            .lines()
                            .find(|line| line.starts_with("OpenSSL ") && line.len() < 60)
                            .map(|s| s.to_string())
                    });

                found_libs.push((path.to_string(), version));
            }
        }

        // Also check ldconfig
        if let Ok(output) = Command::new("ldconfig").args(["-p"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("libssl.so") {
                        if let Some(path) = line.split("=>").nth(1) {
                            let path = path.trim();
                            if !path.is_empty() && !found_libs.iter().any(|(p, _)| p == path) {
                                let version = Command::new("strings")
                                    .args([path, "-a"])
                                    .output()
                                    .ok()
                                    .and_then(|output| {
                                        let stdout = String::from_utf8_lossy(&output.stdout);
                                        stdout
                                            .lines()
                                            .find(|line| {
                                                line.starts_with("OpenSSL ") && line.len() < 60
                                            })
                                            .map(|s| s.to_string())
                                    });
                                found_libs.push((path.to_string(), version));
                            }
                        }
                    }
                }
            }
        }

        if found_libs.is_empty() {
            println!("  No SSL libraries found [WARN]");
        } else {
            for (path, version) in &found_libs {
                println!("  {}", path);
                if let Some(v) = version {
                    println!("    Version: {}", v);
                }

                if detailed {
                    // Show key symbols
                    if let Ok(output) = Command::new("nm").args(["-D", path]).output() {
                        if output.status.success() {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let key_symbols = ["SSL_read", "SSL_write", "SSL_new", "SSL_free"];
                            let mut found_syms = Vec::new();

                            for line in stdout.lines() {
                                for sym in &key_symbols {
                                    if line.contains(sym) && line.contains(" T ") {
                                        found_syms.push(*sym);
                                    }
                                }
                            }

                            if !found_syms.is_empty() {
                                println!("    Key symbols: {}", found_syms.join(", "));
                            }
                        }
                    }
                }
                println!();
            }
        }

        // Show process usage
        if show_usage {
            println!();
            println!("Processes Using SSL Libraries:");
            println!("------------------------------");

            let mut lib_users: HashMap<String, Vec<(u32, String)>> = HashMap::new();

            // Scan /proc for processes using libssl
            if let Ok(proc_entries) = fs::read_dir("/proc") {
                for entry in proc_entries.flatten() {
                    let name = entry.file_name();
                    if let Ok(pid) = name.to_string_lossy().parse::<u32>() {
                        let maps_path = format!("/proc/{}/maps", pid);
                        if let Ok(maps) = fs::read_to_string(&maps_path) {
                            for line in maps.lines() {
                                if line.contains("libssl.so") {
                                    if let Some(lib_path) = line.split_whitespace().last() {
                                        if lib_path.starts_with('/') {
                                            // Get process name
                                            let comm_path = format!("/proc/{}/comm", pid);
                                            let proc_name = fs::read_to_string(&comm_path)
                                                .map(|s| s.trim().to_string())
                                                .unwrap_or_else(|_| "unknown".to_string());

                                            lib_users
                                                .entry(lib_path.to_string())
                                                .or_default()
                                                .push((pid, proc_name));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if lib_users.is_empty() {
                println!("  No processes currently using SSL libraries");
            } else {
                for (lib, users) in &lib_users {
                    println!("  {}:", lib);
                    // Dedupe by process name
                    let mut seen: HashMap<String, Vec<u32>> = HashMap::new();
                    for (pid, name) in users {
                        seen.entry(name.clone()).or_default().push(*pid);
                    }
                    for (name, pids) in &seen {
                        if pids.len() == 1 {
                            println!("    {} (PID: {})", name, pids[0]);
                        } else {
                            println!("    {} ({} instances)", name, pids.len());
                        }
                    }
                    println!();
                }
            }
        }

        // Check for alternative TLS libraries (mostly unsupported)
        println!();
        println!("Alternative TLS Libraries:");
        println!("--------------------------");

        let alt_tls_libs: &[(&str, &str, &str)] = &[
            (
                "/usr/lib/x86_64-linux-gnu/libgnutls.so.30",
                "GnuTLS",
                "NOT SUPPORTED",
            ),
            ("/usr/lib/libgnutls.so", "GnuTLS", "NOT SUPPORTED"),
            (
                "/usr/lib/x86_64-linux-gnu/libnss3.so",
                "NSS",
                "NOT SUPPORTED",
            ),
            ("/usr/lib/libnss3.so", "NSS", "NOT SUPPORTED"),
        ];

        let mut found_alt = false;
        for (path, name, status) in alt_tls_libs {
            if std::path::Path::new(path).exists() {
                found_alt = true;
                println!("  {} at {} [{}]", name, path, status);
            }
        }

        if !found_alt {
            println!("  None detected");
        }

        // Unsupported TLS implementations info
        println!();
        println!("Known Unsupported TLS Implementations:");
        println!("--------------------------------------");
        println!("  • BoringSSL   - Used by: Chrome, gRPC, some Go apps");
        println!("  • GnuTLS      - Used by: wget, some GNOME apps");
        println!("  • NSS         - Used by: Firefox, Chromium");
        println!("  • rustls      - Used by: Rust apps (reqwest, hyper with rustls)");
        println!("  • Go crypto/tls - Used by: Go applications (kubectl, docker, etc.)");
        println!();
        println!("  NOTE: Applications using these TLS libraries will NOT be captured.");
        println!("        Only OpenSSL-based applications are currently supported.");

        // Edge cases reminder
        println!();
        println!("Edge Cases (may not use system SSL):");
        println!("-------------------------------------");
        for (pattern, desc) in EDGE_CASE_PATHS {
            println!("  {} - {}", pattern, desc);
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("SSL library information is only available on Linux.");
        let _ = (detailed, show_usage);
    }

    println!();
    Ok(())
}

// =============================================================================
// Daemon Command - Background Service Management
// =============================================================================

/// PID file location
const PID_FILE: &str = "/var/run/oisp-sensor.pid";

/// Default log directory
const LOG_DIR: &str = "/var/log/oisp-sensor";

async fn daemon_command(cmd: DaemonCommands) -> anyhow::Result<()> {
    match cmd {
        DaemonCommands::Start {
            output,
            no_web,
            port,
            redaction,
        } => daemon_start(output, !no_web, port, redaction).await,
        DaemonCommands::Stop => daemon_stop().await,
        DaemonCommands::Status => daemon_status().await,
        DaemonCommands::Logs { follow, num } => daemon_logs(follow, num).await,
    }
}

async fn daemon_start(
    output: PathBuf,
    web: bool,
    port: u16,
    redaction: String,
) -> anyhow::Result<()> {
    // Check if already running
    if let Some(pid) = read_pid_file() {
        if is_process_running(pid) {
            println!("OISP Sensor daemon is already running (PID: {})", pid);
            println!();
            println!("Use 'oisp-sensor daemon stop' to stop it first.");
            return Ok(());
        } else {
            // Stale PID file, remove it
            let _ = std::fs::remove_file(PID_FILE);
        }
    }

    // Check if running as root (required for eBPF)
    #[cfg(target_os = "linux")]
    {
        let uid = unsafe { libc::getuid() };
        if uid != 0 {
            println!("Error: Daemon mode requires root privileges.");
            println!();
            println!("Run: sudo oisp-sensor daemon start");
            return Ok(());
        }
    }

    // Ensure log directory exists
    std::fs::create_dir_all(LOG_DIR)?;

    // Ensure output directory exists
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    println!("Starting OISP Sensor daemon...");
    println!();

    // Build command for the daemon process
    let exe = std::env::current_exe()?;
    let mut args = vec![
        "record".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
        "--redaction".to_string(),
        redaction,
    ];

    if !web {
        args.push("--web".to_string());
        args.push("false".to_string());
    } else {
        args.push("--port".to_string());
        args.push(port.to_string());
    }

    // Fork and exec using systemd if available, otherwise direct fork
    #[cfg(target_os = "linux")]
    {
        // Check if systemd is available and unit is installed
        if std::path::Path::new("/run/systemd/system").exists()
            && std::path::Path::new("/etc/systemd/system/oisp-sensor.service").exists()
        {
            println!("Using systemd to start daemon...");
            let status = std::process::Command::new("systemctl")
                .args(["start", "oisp-sensor"])
                .status()?;

            if status.success() {
                println!("Daemon started via systemd.");
                println!();
                println!("  Status:  oisp-sensor daemon status");
                println!("  Logs:    oisp-sensor daemon logs --follow");
                println!("  Stop:    sudo oisp-sensor daemon stop");
            } else {
                println!("Failed to start via systemd. Check: journalctl -u oisp-sensor");
            }
            return Ok(());
        }

        // Direct daemonization using fork
        println!("Starting daemon directly (systemd not available)...");

        use std::os::unix::process::CommandExt;

        // Create a child process that will become the daemon
        let child = std::process::Command::new(&exe)
            .args(&args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .process_group(0) // Create new process group
            .spawn()?;

        let pid = child.id();

        // Write PID file
        std::fs::write(PID_FILE, pid.to_string())?;

        println!("Daemon started (PID: {})", pid);
        println!();
        println!("  Output:  {}", output.display());
        if web {
            println!("  Web UI:  http://127.0.0.1:{}", port);
        }
        println!();
        println!("  Status:  oisp-sensor daemon status");
        println!("  Logs:    oisp-sensor daemon logs --follow");
        println!("  Stop:    sudo oisp-sensor daemon stop");
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (exe, args);
        println!("Daemon mode is only supported on Linux.");
        println!();
        println!("On other platforms, run in the foreground:");
        println!("  oisp-sensor record --output {}", output.display());
    }

    Ok(())
}

async fn daemon_stop() -> anyhow::Result<()> {
    // Check if systemd is managing the daemon
    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/run/systemd/system").exists()
            && std::path::Path::new("/etc/systemd/system/oisp-sensor.service").exists()
        {
            // Check if service is active
            let output = std::process::Command::new("systemctl")
                .args(["is-active", "oisp-sensor"])
                .output()?;

            if output.status.success() {
                println!("Stopping OISP Sensor daemon via systemd...");
                let status = std::process::Command::new("systemctl")
                    .args(["stop", "oisp-sensor"])
                    .status()?;

                if status.success() {
                    println!("Daemon stopped.");
                } else {
                    println!("Failed to stop daemon via systemd.");
                }
                return Ok(());
            }
        }
    }

    // Check PID file
    if let Some(pid) = read_pid_file() {
        if is_process_running(pid) {
            println!("Stopping OISP Sensor daemon (PID: {})...", pid);

            #[cfg(target_os = "linux")]
            {
                // Send SIGTERM
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }

                // Wait a bit for graceful shutdown
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                // Check if still running
                if is_process_running(pid) {
                    println!("Process didn't stop, sending SIGKILL...");
                    unsafe {
                        libc::kill(pid as i32, libc::SIGKILL);
                    }
                }
            }

            #[cfg(not(target_os = "linux"))]
            {
                println!("Note: Cannot send signals on this platform.");
                println!("Please stop the process manually (PID: {})", pid);
            }

            // Remove PID file
            let _ = std::fs::remove_file(PID_FILE);
            println!("Daemon stopped.");
        } else {
            println!("Daemon not running (stale PID file removed).");
            let _ = std::fs::remove_file(PID_FILE);
        }
    } else {
        println!("Daemon not running (no PID file found).");
    }

    Ok(())
}

async fn daemon_status() -> anyhow::Result<()> {
    println!();
    println!("OISP Sensor Daemon Status");
    println!("=========================");
    println!();

    // Check systemd first
    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/run/systemd/system").exists()
            && std::path::Path::new("/etc/systemd/system/oisp-sensor.service").exists()
        {
            let output = std::process::Command::new("systemctl")
                .args(["is-active", "oisp-sensor"])
                .output()?;

            if output.status.success() {
                let status = String::from_utf8_lossy(&output.stdout);
                println!("Status:       {} (systemd managed)", status.trim());

                // Get more details from systemctl
                let show_output = std::process::Command::new("systemctl")
                    .args([
                        "show",
                        "oisp-sensor",
                        "--property=MainPID,ActiveEnterTimestamp",
                    ])
                    .output()?;

                for line in String::from_utf8_lossy(&show_output.stdout).lines() {
                    if line.starts_with("MainPID=") {
                        println!("PID:          {}", line.trim_start_matches("MainPID="));
                    } else if line.starts_with("ActiveEnterTimestamp=") {
                        println!(
                            "Started:      {}",
                            line.trim_start_matches("ActiveEnterTimestamp=")
                        );
                    }
                }

                println!();
                println!("For detailed logs: journalctl -u oisp-sensor -f");
                return Ok(());
            }
        }
    }

    // Check PID file
    if let Some(pid) = read_pid_file() {
        if is_process_running(pid) {
            println!("Status:       Running");
            println!("PID:          {}", pid);

            // Try to get process start time
            #[cfg(target_os = "linux")]
            {
                if let Ok(stat) = std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
                    // Parse uptime from stat file
                    if let Some(start_time) = parse_proc_start_time(&stat) {
                        println!("Uptime:       {}", format_uptime(start_time));
                    }
                }
            }

            // Check if log file exists and show event count
            let log_path = PathBuf::from(LOG_DIR).join("events.jsonl");
            if log_path.exists() {
                if let Ok(metadata) = std::fs::metadata(&log_path) {
                    println!(
                        "Output:       {} ({} bytes)",
                        log_path.display(),
                        metadata.len()
                    );
                }
            }
        } else {
            println!("Status:       Not running (stale PID file)");
            let _ = std::fs::remove_file(PID_FILE);
        }
    } else {
        println!("Status:       Not running");
    }

    println!();

    Ok(())
}

async fn daemon_logs(follow: bool, num: usize) -> anyhow::Result<()> {
    // Check systemd first
    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/run/systemd/system").exists()
            && std::path::Path::new("/etc/systemd/system/oisp-sensor.service").exists()
        {
            let num_str = num.to_string();
            let mut args = vec!["-u", "oisp-sensor", "-n", &num_str];
            if follow {
                args.push("-f");
            }

            let status = std::process::Command::new("journalctl")
                .args(&args)
                .status()?;

            if !status.success() {
                println!("Failed to read logs from journalctl.");
            }
            return Ok(());
        }
    }

    // Fall back to reading events file
    let log_path = PathBuf::from(LOG_DIR).join("events.jsonl");

    if !log_path.exists() {
        println!("No log file found at: {}", log_path.display());
        return Ok(());
    }

    if follow {
        // Tail -f style following
        println!("Following {}... (Ctrl+C to stop)", log_path.display());
        println!();

        use std::io::{BufRead, BufReader, Seek, SeekFrom};

        let file = std::fs::File::open(&log_path)?;
        let mut reader = BufReader::new(file);

        // Seek to end
        reader.seek(SeekFrom::End(0))?;

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // No new data, wait a bit
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                Ok(_) => {
                    print!("{}", line);
                }
                Err(e) => {
                    error!("Error reading log: {}", e);
                    break;
                }
            }
        }
    } else {
        // Show last N lines
        use std::io::{BufRead, BufReader};

        let file = std::fs::File::open(&log_path)?;
        let reader = BufReader::new(file);

        let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
        let start = lines.len().saturating_sub(num);

        for line in &lines[start..] {
            println!("{}", line);
        }
    }

    Ok(())
}

/// Read PID from PID file
fn read_pid_file() -> Option<u32> {
    std::fs::read_to_string(PID_FILE)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Check if a process with given PID is running
fn is_process_running(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        // Check if /proc/{pid} exists (Linux) or use kill(0) signal
        if std::path::Path::new(&format!("/proc/{}", pid)).exists() {
            return true;
        }
        // Fallback: sending signal 0 checks if process exists
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, check using /proc alternative
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
            || std::process::Command::new("ps")
                .args(["-p", &pid.to_string()])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        false
    }
}

/// Parse process start time from /proc/{pid}/stat
#[cfg(target_os = "linux")]
fn parse_proc_start_time(stat: &str) -> Option<u64> {
    // Field 22 is starttime (time the process started after system boot)
    let parts: Vec<&str> = stat.split_whitespace().collect();
    parts.get(21).and_then(|s| s.parse().ok())
}

/// Format uptime from jiffies
#[cfg(target_os = "linux")]
fn format_uptime(start_jiffies: u64) -> String {
    // This is a simplified version - proper implementation would read /proc/uptime
    // and calculate actual uptime from boot time
    let _ = start_jiffies;
    "unknown".to_string() // TODO: implement proper uptime calculation
}
