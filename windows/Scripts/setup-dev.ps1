# setup-dev.ps1
# One-time setup script for Windows development environment
#
# Usage: .\setup-dev.ps1
#
# This script will:
# 1. Download WinDivert 2.2.2-A (pre-signed driver)
# 2. Extract to windows/deps/
# 3. Set WINDIVERT_PATH environment variable
# 4. Verify the setup works

param(
    [switch]$Force  # Force re-download even if exists
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$WindowsDir = Split-Path -Parent $ScriptDir
$DepsDir = Join-Path $WindowsDir "deps"
$WinDivertVersion = "2.2.2"
$WinDivertVariant = "A"  # A, B, C are identical except signature
$WinDivertUrl = "https://github.com/basil00/WinDivert/releases/download/v$WinDivertVersion/WinDivert-$WinDivertVersion-$WinDivertVariant.zip"
$WinDivertDir = Join-Path $DepsDir "WinDivert-$WinDivertVersion-$WinDivertVariant"

Write-Host "=== OISP Sensor Windows Development Setup ===" -ForegroundColor Cyan
Write-Host ""

# Create deps directory
if (-not (Test-Path $DepsDir)) {
    Write-Host "Creating deps directory..."
    New-Item -ItemType Directory -Path $DepsDir | Out-Null
}

# Download WinDivert if not present
if ((Test-Path $WinDivertDir) -and -not $Force) {
    Write-Host "WinDivert already downloaded at: $WinDivertDir" -ForegroundColor Green
} else {
    Write-Host "Downloading WinDivert $WinDivertVersion..."
    $ZipPath = Join-Path $DepsDir "WinDivert.zip"

    try {
        Invoke-WebRequest -Uri $WinDivertUrl -OutFile $ZipPath -UseBasicParsing
        Write-Host "Download complete."
    } catch {
        Write-Host "ERROR: Failed to download WinDivert" -ForegroundColor Red
        Write-Host "URL: $WinDivertUrl"
        Write-Host "Please download manually and extract to: $WinDivertDir"
        exit 1
    }

    # Extract
    Write-Host "Extracting..."
    if (Test-Path $WinDivertDir) {
        Remove-Item -Recurse -Force $WinDivertDir
    }
    Expand-Archive -Path $ZipPath -DestinationPath $DepsDir
    Remove-Item $ZipPath

    Write-Host "WinDivert extracted to: $WinDivertDir" -ForegroundColor Green
}

# Verify files exist
$RequiredFiles = @(
    "WinDivert.dll",
    "WinDivert64.sys",
    "WinDivert.lib"
)

Write-Host ""
Write-Host "Verifying WinDivert files..."
$AllFilesExist = $true
foreach ($File in $RequiredFiles) {
    $FilePath = Join-Path $WinDivertDir $File
    if (Test-Path $FilePath) {
        Write-Host "  [OK] $File" -ForegroundColor Green
    } else {
        Write-Host "  [MISSING] $File" -ForegroundColor Red
        $AllFilesExist = $false
    }
}

if (-not $AllFilesExist) {
    Write-Host ""
    Write-Host "ERROR: Some required files are missing" -ForegroundColor Red
    exit 1
}

# Check if driver is signed
Write-Host ""
Write-Host "Verifying driver signature..."
$SysFile = Join-Path $WinDivertDir "WinDivert64.sys"
$Signature = Get-AuthenticodeSignature -FilePath $SysFile
if ($Signature.Status -eq "Valid") {
    Write-Host "  Driver signature: VALID" -ForegroundColor Green
    Write-Host "  Signed by: $($Signature.SignerCertificate.Subject)"
} else {
    Write-Host "  WARNING: Driver signature status: $($Signature.Status)" -ForegroundColor Yellow
    Write-Host "  This may cause issues on some Windows configurations"
}

# Set environment variable
Write-Host ""
Write-Host "Setting WINDIVERT_PATH environment variable..."

# Set for current session
$env:WINDIVERT_PATH = $WinDivertDir

# Set permanently for user
[Environment]::SetEnvironmentVariable("WINDIVERT_PATH", $WinDivertDir, "User")
Write-Host "  WINDIVERT_PATH = $WinDivertDir" -ForegroundColor Green

# Verify Rust is installed
Write-Host ""
Write-Host "Checking Rust installation..."
try {
    $RustVersion = & rustc --version 2>&1
    Write-Host "  Rust: $RustVersion" -ForegroundColor Green
} catch {
    Write-Host "  WARNING: Rust not found" -ForegroundColor Yellow
    Write-Host "  Install with: winget install Rustlang.Rustup"
}

# Check for Visual Studio Build Tools
Write-Host ""
Write-Host "Checking build tools..."
$VsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $VsWhere) {
    $VsInstall = & $VsWhere -latest -property installationPath 2>&1
    if ($VsInstall) {
        Write-Host "  Visual Studio: Found at $VsInstall" -ForegroundColor Green
    }
} else {
    Write-Host "  WARNING: Visual Studio Build Tools not found" -ForegroundColor Yellow
    Write-Host "  Install with: winget install Microsoft.VisualStudio.2022.BuildTools"
}

Write-Host ""
Write-Host "=== Setup Complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. Open a NEW terminal (to get updated PATH)"
Write-Host "  2. cd to oisp-sensor directory"
Write-Host "  3. Run: cargo build --release"
Write-Host ""
Write-Host "To test WinDivert (requires Admin):"
Write-Host "  1. Open PowerShell as Administrator"
Write-Host "  2. Run: .\target\release\oisp-redirector.exe"
Write-Host ""
