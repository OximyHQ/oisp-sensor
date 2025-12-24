//! Build script for oisp-capture-ebpf
//!
//! On Linux, this embeds the sslsniff binary (libbpf-based) for SSL capture.
//! The sslsniff binary is built separately (during Docker build or via make).

use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo about our custom cfg flags
    println!("cargo::rustc-check-cfg=cfg(embedded_sslsniff)");

    println!("cargo:rerun-if-env-changed=OISP_SSLSNIFF_PATH");

    // Only embed on Linux
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "linux" {
        println!("cargo:warning=sslsniff only available on Linux");
        return;
    }

    // Try to find and embed sslsniff
    if let Err(e) = embed_sslsniff() {
        println!(
            "cargo:warning=Failed to embed sslsniff: {}. \
             Sensor will look for sslsniff in PATH at runtime.",
            e
        );
    }
}

fn embed_sslsniff() -> Result<(), String> {
    let out_dir = env::var("OUT_DIR").map_err(|e| format!("OUT_DIR not set: {}", e))?;
    let out_path = PathBuf::from(&out_dir).join("sslsniff");

    // Check for explicit path via environment variable
    if let Ok(path) = env::var("OISP_SSLSNIFF_PATH") {
        let src = PathBuf::from(&path);
        if src.exists() {
            std::fs::copy(&src, &out_path)
                .map_err(|e| format!("Failed to copy sslsniff from {}: {}", path, e))?;
            println!("cargo:rustc-cfg=embedded_sslsniff");
            println!(
                "cargo:warning=Embedded sslsniff from OISP_SSLSNIFF_PATH ({} bytes)",
                std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0)
            );
            return Ok(());
        }
    }

    // Look for sslsniff in common build locations
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map_err(|e| format!("CARGO_MANIFEST_DIR not set: {}", e))?;
    let manifest_path = PathBuf::from(&manifest_dir);
    let workspace_root = manifest_path
        .parent() // crates/
        .and_then(|p| p.parent()) // oisp-sensor/
        .ok_or("Failed to find workspace root")?;

    // Check bpf/ directory (where we build sslsniff)
    let bpf_sslsniff = workspace_root.join("bpf").join("sslsniff");
    if bpf_sslsniff.exists() {
        println!("cargo:rerun-if-changed={}", bpf_sslsniff.display());
        std::fs::copy(&bpf_sslsniff, &out_path)
            .map_err(|e| format!("Failed to copy sslsniff: {}", e))?;
        println!("cargo:rustc-cfg=embedded_sslsniff");
        println!(
            "cargo:warning=Embedded sslsniff from bpf/ ({} bytes)",
            std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0)
        );
        return Ok(());
    }

    // Check /usr/local/bin (for Docker builds)
    let usr_local_sslsniff = PathBuf::from("/usr/local/bin/sslsniff");
    if usr_local_sslsniff.exists() {
        std::fs::copy(&usr_local_sslsniff, &out_path)
            .map_err(|e| format!("Failed to copy sslsniff from /usr/local/bin: {}", e))?;
        println!("cargo:rustc-cfg=embedded_sslsniff");
        println!(
            "cargo:warning=Embedded sslsniff from /usr/local/bin ({} bytes)",
            std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0)
        );
        return Ok(());
    }

    Err(format!(
        "sslsniff not found. Looked in:\n  \
         - OISP_SSLSNIFF_PATH env var\n  \
         - {:?}\n  \
         - /usr/local/bin/sslsniff\n\
         Build sslsniff with: cd bpf && make sslsniff",
        bpf_sslsniff
    ))
}
