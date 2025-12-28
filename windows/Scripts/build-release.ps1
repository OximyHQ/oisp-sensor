# build-release.ps1
# Build OISP Sensor Windows components for release
#
# Usage: .\build-release.ps1 [-Clean] [-CopyDeps]

param(
    [switch]$Clean,    # Clean before building
    [switch]$CopyDeps  # Copy WinDivert files to output
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$OutputDir = Join-Path $RootDir "target\release"

Write-Host "=== OISP Sensor Windows Build ===" -ForegroundColor Cyan
Write-Host ""

# Check prerequisites
Write-Host "Checking prerequisites..."

# Rust
try {
    $rustVersion = & rustc --version 2>&1
    Write-Host "[OK] Rust: $rustVersion" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Rust not found. Install with: winget install Rustlang.Rustup" -ForegroundColor Red
    exit 1
}

# WINDIVERT_PATH
if (-not $env:WINDIVERT_PATH) {
    Write-Host "[WARNING] WINDIVERT_PATH not set" -ForegroundColor Yellow
    Write-Host "Running setup-dev.ps1 first..."
    $SetupScript = Join-Path $ScriptDir "setup-dev.ps1"
    if (Test-Path $SetupScript) {
        & $SetupScript
    }
}

if ($env:WINDIVERT_PATH) {
    Write-Host "[OK] WINDIVERT_PATH: $env:WINDIVERT_PATH" -ForegroundColor Green
}

Write-Host ""

# Clean if requested
if ($Clean) {
    Write-Host "Cleaning previous build..."
    Push-Location $RootDir
    cargo clean
    Pop-Location
}

# Build
Write-Host "Building release binaries..."
Push-Location $RootDir

try {
    # Build the main sensor
    Write-Host ""
    Write-Host "Building oisp-sensor..."
    cargo build --release --bin oisp-sensor
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] oisp-sensor build failed" -ForegroundColor Red
        exit 1
    }
    Write-Host "[OK] oisp-sensor built" -ForegroundColor Green

    # Build the redirector
    Write-Host ""
    Write-Host "Building oisp-redirector..."
    cargo build --release --bin oisp-redirector
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] oisp-redirector build failed" -ForegroundColor Red
        exit 1
    }
    Write-Host "[OK] oisp-redirector built" -ForegroundColor Green

} finally {
    Pop-Location
}

# Copy dependencies if requested
if ($CopyDeps -and $env:WINDIVERT_PATH) {
    Write-Host ""
    Write-Host "Copying WinDivert files..."

    $Files = @("WinDivert.dll", "WinDivert64.sys", "WinDivert.lib")
    foreach ($File in $Files) {
        $Src = Join-Path $env:WINDIVERT_PATH $File
        $Dst = Join-Path $OutputDir $File
        if (Test-Path $Src) {
            Copy-Item $Src $Dst -Force
            Write-Host "  Copied: $File"
        }
    }
}

Write-Host ""
Write-Host "=== Build Complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Output directory: $OutputDir"
Write-Host ""
Write-Host "Binaries:"

$Binaries = @("oisp-sensor.exe", "oisp-redirector.exe")
foreach ($Binary in $Binaries) {
    $Path = Join-Path $OutputDir $Binary
    if (Test-Path $Path) {
        $Size = (Get-Item $Path).Length / 1MB
        Write-Host "  $Binary ($("{0:N2}" -f $Size) MB)" -ForegroundColor Green
    } else {
        Write-Host "  $Binary (not found)" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. Run as Administrator: .\target\release\oisp-redirector.exe"
Write-Host "  2. In another terminal: .\target\release\oisp-sensor.exe record --output events.jsonl"
Write-Host ""
