# Build OISP Sensor Windows Application
# Builds both the Rust binaries and the .NET tray application

param(
    [switch]$Release,
    [switch]$SelfContained,
    [string]$OutputDir = "$PSScriptRoot\..\..\target\windows-dist"
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path "$PSScriptRoot\..\.."
$OISPAppDir = "$RepoRoot\windows\OISPApp"
$Configuration = if ($Release) { "Release" } else { "Debug" }

Write-Host "=================================="
Write-Host "OISP Sensor Windows Build"
Write-Host "=================================="
Write-Host "Configuration: $Configuration"
Write-Host "Output: $OutputDir"
Write-Host ""

# Step 1: Generate icons if missing
Write-Host "Step 1: Checking icons..."
$IconPath = "$OISPAppDir\Resources\oisp-icon.ico"
if (-not (Test-Path $IconPath)) {
    Write-Host "  Generating placeholder icons..."
    & "$PSScriptRoot\generate-icons.ps1"
}
else {
    Write-Host "  Icons exist."
}

# Step 2: Build Rust binaries
Write-Host ""
Write-Host "Step 2: Building Rust binaries..."
Push-Location $RepoRoot

$cargoArgs = @("build")
if ($Release) {
    $cargoArgs += "--release"
}
$cargoArgs += "--bin", "oisp-sensor"
$cargoArgs += "--bin", "oisp-redirector"

Write-Host "  cargo $($cargoArgs -join ' ')"
& cargo @cargoArgs

if ($LASTEXITCODE -ne 0) {
    Pop-Location
    throw "Rust build failed"
}
Pop-Location
Write-Host "  Rust build complete."

# Step 3: Build .NET application
Write-Host ""
Write-Host "Step 3: Building .NET application..."
Push-Location $OISPAppDir

$dotnetArgs = @("build", "-c", $Configuration)
Write-Host "  dotnet $($dotnetArgs -join ' ')"
& dotnet @dotnetArgs

if ($LASTEXITCODE -ne 0) {
    Pop-Location
    throw ".NET build failed"
}
Pop-Location
Write-Host "  .NET build complete."

# Step 4: Copy files to output directory
Write-Host ""
Write-Host "Step 4: Creating distribution..."

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
}

$RustBinDir = if ($Release) { "$RepoRoot\target\release" } else { "$RepoRoot\target\debug" }
$DotNetBinDir = "$OISPAppDir\bin\$Configuration\net8.0-windows"

# Copy Rust binaries
Write-Host "  Copying Rust binaries..."
Copy-Item "$RustBinDir\oisp-sensor.exe" -Destination $OutputDir -Force -ErrorAction SilentlyContinue
Copy-Item "$RustBinDir\oisp-redirector.exe" -Destination $OutputDir -Force -ErrorAction SilentlyContinue

# Copy .NET application
Write-Host "  Copying .NET application..."
Copy-Item "$DotNetBinDir\*" -Destination $OutputDir -Recurse -Force -ErrorAction SilentlyContinue

# Copy WinDivert files if available
$WinDivertDir = "$RepoRoot\windows\deps\WinDivert-2.2.2-A\x64"
if (Test-Path $WinDivertDir) {
    Write-Host "  Copying WinDivert..."
    Copy-Item "$WinDivertDir\WinDivert.dll" -Destination $OutputDir -Force
    Copy-Item "$WinDivertDir\WinDivert64.sys" -Destination $OutputDir -Force
}
else {
    Write-Host "  WARNING: WinDivert not found at $WinDivertDir"
    Write-Host "  Run setup-dev.ps1 to download WinDivert"
}

Write-Host ""
Write-Host "=================================="
Write-Host "Build complete!"
Write-Host "=================================="
Write-Host ""
Write-Host "Output directory: $OutputDir"
Write-Host ""
Write-Host "To run:"
Write-Host "  1. Ensure WinDivert files are present"
Write-Host "  2. Run OISPApp.exe"
Write-Host ""

# List output files
Write-Host "Files:"
Get-ChildItem $OutputDir -Name | ForEach-Object { Write-Host "  $_" }
