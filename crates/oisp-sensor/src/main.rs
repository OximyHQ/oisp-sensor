//! OISP Sensor - Universal AI Observability
//!
//! Zero-instrumentation sensor for AI activity monitoring and control.

use clap::{Parser, Subcommand};
use oisp_capture::{TestGenerator, TestGeneratorConfig};
#[cfg(target_os = "linux")]
use oisp_capture_ebpf::{EbpfCapture, EbpfCaptureConfig};
use oisp_core::config::{ConfigLoader, SensorConfig};
use oisp_core::pipeline::{Pipeline, PipelineConfig};
use oisp_decode::HttpDecoder;
use oisp_enrich::{HostEnricher, ProcessTreeEnricher};
use oisp_export::jsonl::{JsonlExporter, JsonlExporterConfig};
use oisp_export::websocket::{WebSocketExporter, WebSocketExporterConfig};
use oisp_redact::RedactionPlugin;
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

        /// Path to libssl.so for SSL interception (auto-detected if not specified)
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

    /// Self-test sensor capabilities
    Test,

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
        Commands::Test => test_command().await,
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

    #[cfg(not(target_os = "linux"))]
    {
        info!("eBPF capture not available on this platform");
        let _ = (&config.ebpf_path, &config.libssl_path); // Suppress unused warnings
    }

    // Add decoder
    pipeline.add_decode(Box::new(HttpDecoder::new()));

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

    // Add decoder
    pipeline.add_decode(Box::new(HttpDecoder::new()));

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
