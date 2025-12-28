# Build OISP Sensor Windows Installer
# Requires: NSIS 3.x installed

param(
    [switch]$SkipBuild,
    [string]$Version = "0.1.0"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path "$ScriptDir\..\.."

Write-Host "============================================"
Write-Host "OISP Sensor Installer Build"
Write-Host "Version: $Version"
Write-Host "============================================"
Write-Host ""

# Step 1: Build all components
if (-not $SkipBuild) {
    Write-Host "Step 1: Building components..."

    # Build Rust binaries
    Write-Host "  Building Rust binaries..."
    Push-Location $RepoRoot
    cargo build --release --bin oisp-sensor --bin oisp-redirector
    if ($LASTEXITCODE -ne 0) {
        Pop-Location
        throw "Rust build failed"
    }
    Pop-Location

    # Build .NET application
    Write-Host "  Building .NET application..."
    Push-Location "$RepoRoot\windows\OISPApp"
    dotnet build -c Release
    if ($LASTEXITCODE -ne 0) {
        Pop-Location
        throw ".NET build failed"
    }
    Pop-Location

    Write-Host "  Build complete."
} else {
    Write-Host "Step 1: Skipping build (--SkipBuild specified)"
}

# Step 2: Verify required files
Write-Host ""
Write-Host "Step 2: Verifying required files..."

$requiredFiles = @(
    "$RepoRoot\target\release\oisp-sensor.exe",
    "$RepoRoot\target\release\oisp-redirector.exe",
    "$RepoRoot\windows\OISPApp\bin\Release\net8.0-windows\OISPApp.exe",
    "$RepoRoot\windows\deps\WinDivert-2.2.2-A\x64\WinDivert.dll",
    "$RepoRoot\windows\deps\WinDivert-2.2.2-A\x64\WinDivert64.sys"
)

$missing = @()
foreach ($file in $requiredFiles) {
    if (-not (Test-Path $file)) {
        $missing += $file
    }
}

if ($missing.Count -gt 0) {
    Write-Host "  ERROR: Missing required files:"
    foreach ($file in $missing) {
        Write-Host "    - $file"
    }

    if ($missing -match "WinDivert") {
        Write-Host ""
        Write-Host "  WinDivert not found. Run setup-dev.ps1 to download it."
    }

    throw "Missing required files"
}

Write-Host "  All required files present."

# Step 3: Create/verify resources
Write-Host ""
Write-Host "Step 3: Preparing resources..."

# Create placeholder icon if missing
$iconPath = "$ScriptDir\resources\oisp-icon.ico"
if (-not (Test-Path $iconPath)) {
    Write-Host "  WARNING: Icon file missing. Creating placeholder..."
    # Copy from OISPApp resources if available
    $appIcon = "$RepoRoot\windows\OISPApp\Resources\oisp-icon.ico"
    if (Test-Path $appIcon) {
        Copy-Item $appIcon $iconPath
    } else {
        # Create minimal placeholder
        [byte[]]$minIcon = @(0,0,1,0,1,0,1,1,0,0,1,0,32,0,40,0,0,0,22,0,0,0)
        [System.IO.File]::WriteAllBytes($iconPath, $minIcon)
    }
}

# Create welcome bitmap if missing (164x314 BMP for NSIS MUI)
$welcomeBmp = "$ScriptDir\resources\welcome.bmp"
if (-not (Test-Path $welcomeBmp)) {
    Write-Host "  WARNING: Welcome bitmap missing. NSIS may use default."
    # Could generate a placeholder bitmap here
}

# Step 4: Find NSIS
Write-Host ""
Write-Host "Step 4: Locating NSIS..."

$nsisPath = $null
$nsisPaths = @(
    "${env:ProgramFiles(x86)}\NSIS\makensis.exe",
    "$env:ProgramFiles\NSIS\makensis.exe",
    "C:\Program Files (x86)\NSIS\makensis.exe",
    "C:\Program Files\NSIS\makensis.exe"
)

foreach ($path in $nsisPaths) {
    if (Test-Path $path) {
        $nsisPath = $path
        break
    }
}

# Also check PATH
if (-not $nsisPath) {
    $inPath = Get-Command makensis.exe -ErrorAction SilentlyContinue
    if ($inPath) {
        $nsisPath = $inPath.Source
    }
}

if (-not $nsisPath) {
    Write-Host "  ERROR: NSIS not found."
    Write-Host ""
    Write-Host "  Please install NSIS from: https://nsis.sourceforge.io/"
    Write-Host "  Or use: winget install NSIS.NSIS"
    throw "NSIS not found"
}

Write-Host "  Found NSIS: $nsisPath"

# Step 5: Build installer
Write-Host ""
Write-Host "Step 5: Building installer..."

$nsiScript = "$ScriptDir\oisp-sensor.nsi"

Push-Location $ScriptDir
& $nsisPath "/DVERSION=$Version" $nsiScript
$exitCode = $LASTEXITCODE
Pop-Location

if ($exitCode -ne 0) {
    throw "NSIS build failed with exit code $exitCode"
}

# Step 6: Verify output
$installerPath = "$ScriptDir\oisp-sensor-setup.exe"
if (Test-Path $installerPath) {
    $size = (Get-Item $installerPath).Length / 1MB
    Write-Host ""
    Write-Host "============================================"
    Write-Host "Installer built successfully!"
    Write-Host "============================================"
    Write-Host ""
    Write-Host "Output: $installerPath"
    Write-Host "Size:   $([math]::Round($size, 2)) MB"
    Write-Host ""
    Write-Host "To test: Run the installer as Administrator"
} else {
    throw "Installer output not found"
}
