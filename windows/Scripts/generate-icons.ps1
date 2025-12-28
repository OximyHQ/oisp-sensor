# Generate placeholder icons for OISP App
# This creates simple colored square icons as placeholders

param(
    [string]$OutputDir = "$PSScriptRoot\..\OISPApp\Resources"
)

# Ensure output directory exists
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
}

# Load System.Drawing for icon creation
Add-Type -AssemblyName System.Drawing

function Create-PlaceholderIcon {
    param(
        [string]$OutputPath,
        [System.Drawing.Color]$Color,
        [string]$Letter = "O"
    )

    # Create a multi-resolution icon
    $sizes = @(16, 32, 48, 256)
    $bitmaps = @()

    foreach ($size in $sizes) {
        $bitmap = New-Object System.Drawing.Bitmap($size, $size)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)

        # Fill background
        $brush = New-Object System.Drawing.SolidBrush($Color)
        $graphics.FillRectangle($brush, 0, 0, $size, $size)

        # Draw border
        $pen = New-Object System.Drawing.Pen([System.Drawing.Color]::White, [Math]::Max(1, $size / 16))
        $graphics.DrawRectangle($pen, 1, 1, $size - 3, $size - 3)

        # Draw letter
        $fontSize = [Math]::Max(8, $size * 0.6)
        $font = New-Object System.Drawing.Font("Arial", $fontSize, [System.Drawing.FontStyle]::Bold)
        $textBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
        $textSize = $graphics.MeasureString($Letter, $font)
        $x = ($size - $textSize.Width) / 2
        $y = ($size - $textSize.Height) / 2
        $graphics.DrawString($Letter, $font, $textBrush, $x, $y)

        $bitmaps += $bitmap

        $graphics.Dispose()
        $brush.Dispose()
        $pen.Dispose()
        $font.Dispose()
        $textBrush.Dispose()
    }

    # Save as ICO using the largest bitmap (simplified - real ICO would embed all sizes)
    $largestBitmap = $bitmaps[-1]
    $icon = [System.Drawing.Icon]::FromHandle($largestBitmap.GetHicon())

    $stream = [System.IO.File]::Create($OutputPath)
    $icon.Save($stream)
    $stream.Close()

    # Cleanup
    foreach ($bmp in $bitmaps) {
        $bmp.Dispose()
    }

    Write-Host "Created: $OutputPath"
}

try {
    # Create inactive icon (gray)
    $inactivePath = Join-Path $OutputDir "oisp-icon.ico"
    Create-PlaceholderIcon -OutputPath $inactivePath -Color ([System.Drawing.Color]::FromArgb(100, 100, 100)) -Letter "O"

    # Create active icon (green)
    $activePath = Join-Path $OutputDir "oisp-icon-active.ico"
    Create-PlaceholderIcon -OutputPath $activePath -Color ([System.Drawing.Color]::FromArgb(34, 139, 34)) -Letter "O"

    Write-Host ""
    Write-Host "Placeholder icons created successfully!"
    Write-Host "Replace with proper icons for production use."
}
catch {
    Write-Host "Error creating icons: $_"
    Write-Host ""
    Write-Host "Creating minimal placeholder files instead..."

    # Create minimal 1x1 ICO files as fallback
    # ICO header + 1 entry + BMP data for 1x1 pixel
    $icoHeader = [byte[]](0, 0, 1, 0, 1, 0)  # ICO magic + 1 image
    $icoEntry = [byte[]](1, 1, 0, 0, 1, 0, 32, 0, 40, 0, 0, 0, 22, 0, 0, 0)  # 1x1, 32bpp
    $bmpHeader = [byte[]](40, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 32, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)

    # Gray pixel
    $grayPixel = [byte[]](100, 100, 100, 255, 0, 0, 0, 0)
    $grayIco = $icoHeader + $icoEntry + $bmpHeader + $grayPixel
    [System.IO.File]::WriteAllBytes((Join-Path $OutputDir "oisp-icon.ico"), $grayIco)

    # Green pixel
    $greenPixel = [byte[]](34, 139, 34, 255, 0, 0, 0, 0)
    $greenIco = $icoHeader + $icoEntry + $bmpHeader + $greenPixel
    [System.IO.File]::WriteAllBytes((Join-Path $OutputDir "oisp-icon-active.ico"), $greenIco)

    Write-Host "Minimal placeholder icons created."
}
