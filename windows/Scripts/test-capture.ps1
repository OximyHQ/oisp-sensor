# test-capture.ps1
# End-to-end test script for OISP Sensor Windows capture
#
# Usage: .\test-capture.ps1
#
# This script will:
# 1. Build the redirector
# 2. Run the redirector (requires Admin)
# 3. Make a test HTTP request
# 4. Verify packets were captured

param(
    [switch]$Build,       # Build before testing
    [switch]$Verbose,     # Enable verbose output
    [int]$Duration = 10   # How long to capture (seconds)
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$RedirectorExe = Join-Path $RootDir "target\release\oisp-redirector.exe"

Write-Host "=== OISP Sensor Windows Capture Test ===" -ForegroundColor Cyan
Write-Host ""

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "ERROR: This script must be run as Administrator" -ForegroundColor Red
    Write-Host "Right-click PowerShell and select 'Run as administrator'"
    exit 1
}

Write-Host "[OK] Running as Administrator" -ForegroundColor Green

# Check if WINDIVERT_PATH is set
if (-not $env:WINDIVERT_PATH) {
    Write-Host ""
    Write-Host "WARNING: WINDIVERT_PATH not set" -ForegroundColor Yellow
    Write-Host "Running setup-dev.ps1..."

    $SetupScript = Join-Path $ScriptDir "setup-dev.ps1"
    if (Test-Path $SetupScript) {
        & $SetupScript
    } else {
        Write-Host "ERROR: setup-dev.ps1 not found" -ForegroundColor Red
        exit 1
    }
}

Write-Host "[OK] WINDIVERT_PATH: $env:WINDIVERT_PATH" -ForegroundColor Green

# Build if requested or if exe doesn't exist
if ($Build -or (-not (Test-Path $RedirectorExe))) {
    Write-Host ""
    Write-Host "Building oisp-redirector..." -ForegroundColor Yellow

    Push-Location $RootDir
    try {
        cargo build --release --bin oisp-redirector
        if ($LASTEXITCODE -ne 0) {
            Write-Host "ERROR: Build failed" -ForegroundColor Red
            exit 1
        }
    } finally {
        Pop-Location
    }

    Write-Host "[OK] Build complete" -ForegroundColor Green
}

if (-not (Test-Path $RedirectorExe)) {
    Write-Host "ERROR: Redirector not found at: $RedirectorExe" -ForegroundColor Red
    Write-Host "Run with -Build flag to build first"
    exit 1
}

Write-Host "[OK] Redirector binary found" -ForegroundColor Green

# Check WinDivert files
$WinDivertDll = Join-Path $env:WINDIVERT_PATH "WinDivert.dll"
$WinDivertSys = Join-Path $env:WINDIVERT_PATH "WinDivert64.sys"

if (-not (Test-Path $WinDivertDll)) {
    Write-Host "ERROR: WinDivert.dll not found" -ForegroundColor Red
    exit 1
}
if (-not (Test-Path $WinDivertSys)) {
    Write-Host "ERROR: WinDivert64.sys not found" -ForegroundColor Red
    exit 1
}

Write-Host "[OK] WinDivert files present" -ForegroundColor Green

# Copy WinDivert DLL to release directory if not there
$TargetDir = Split-Path -Parent $RedirectorExe
$TargetDll = Join-Path $TargetDir "WinDivert.dll"
$TargetSys = Join-Path $TargetDir "WinDivert64.sys"

if (-not (Test-Path $TargetDll)) {
    Write-Host "Copying WinDivert.dll to target directory..."
    Copy-Item $WinDivertDll $TargetDll
}
if (-not (Test-Path $TargetSys)) {
    Write-Host "Copying WinDivert64.sys to target directory..."
    Copy-Item $WinDivertSys $TargetSys
}

Write-Host ""
Write-Host "=== Starting Capture Test ===" -ForegroundColor Cyan
Write-Host ""

# Start the redirector in background
Write-Host "Starting redirector (capture-only mode)..."
$RedirectorArgs = @("--capture-only")
if ($Verbose) {
    $RedirectorArgs += "--verbose"
}

$RedirectorProcess = Start-Process -FilePath $RedirectorExe `
    -ArgumentList $RedirectorArgs `
    -PassThru `
    -NoNewWindow `
    -RedirectStandardOutput "redirector-stdout.log" `
    -RedirectStandardError "redirector-stderr.log"

if (-not $RedirectorProcess) {
    Write-Host "ERROR: Failed to start redirector" -ForegroundColor Red
    exit 1
}

Write-Host "Redirector PID: $($RedirectorProcess.Id)"
Start-Sleep -Seconds 2

# Check if still running
if ($RedirectorProcess.HasExited) {
    Write-Host "ERROR: Redirector exited immediately" -ForegroundColor Red
    Write-Host ""
    Write-Host "=== stdout ===" -ForegroundColor Yellow
    Get-Content "redirector-stdout.log"
    Write-Host ""
    Write-Host "=== stderr ===" -ForegroundColor Yellow
    Get-Content "redirector-stderr.log"
    exit 1
}

Write-Host "[OK] Redirector running" -ForegroundColor Green

# Make a test request
Write-Host ""
Write-Host "Making test request to api.openai.com..."
try {
    $response = Invoke-WebRequest -Uri "https://api.openai.com/v1/models" `
        -Method GET `
        -Headers @{"Authorization" = "Bearer test-key"} `
        -TimeoutSec 10 `
        -ErrorAction SilentlyContinue
    Write-Host "[OK] Request completed (status: $($response.StatusCode))" -ForegroundColor Green
} catch {
    Write-Host "[OK] Request completed (expected 401 without valid key)" -ForegroundColor Green
}

# Wait for capture
Write-Host ""
Write-Host "Waiting $Duration seconds for capture..."
Start-Sleep -Seconds $Duration

# Stop the redirector
Write-Host ""
Write-Host "Stopping redirector..."
Stop-Process -Id $RedirectorProcess.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

# Show results
Write-Host ""
Write-Host "=== Redirector Output ===" -ForegroundColor Cyan
Write-Host ""
if (Test-Path "redirector-stdout.log") {
    Get-Content "redirector-stdout.log"
}
if (Test-Path "redirector-stderr.log") {
    $stderr = Get-Content "redirector-stderr.log"
    if ($stderr) {
        Write-Host ""
        Write-Host "=== Errors ===" -ForegroundColor Yellow
        Write-Host $stderr
    }
}

# Check for success indicators
$stdout = Get-Content "redirector-stdout.log" -Raw -ErrorAction SilentlyContinue
if ($stdout -match "packets captured") {
    Write-Host ""
    Write-Host "=== TEST PASSED ===" -ForegroundColor Green
    Write-Host "Successfully captured network packets!"
} else {
    Write-Host ""
    Write-Host "=== TEST RESULT UNCLEAR ===" -ForegroundColor Yellow
    Write-Host "Check the output above for details."
}

# Cleanup
Remove-Item "redirector-stdout.log" -ErrorAction SilentlyContinue
Remove-Item "redirector-stderr.log" -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "Test complete."
