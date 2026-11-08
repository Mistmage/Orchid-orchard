# Generate a real multi-size Windows icon from the Orchard flower mark.
# Drawing is done with GDI+ so the build has no external image dependencies.
param(
    [string]$OutDir = (Join-Path $PSScriptRoot "..\apps\orchard\src-tauri\icons")
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$OutDir = [System.IO.Path]::GetFullPath($OutDir)
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function New-FlowerBitmap([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.Clear([System.Drawing.Color]::Transparent)
    $s = $size / 64.0
    $g.ScaleTransform($s, $s)

    $bg = New-Object System.Drawing.SolidBrush([System.Drawing.ColorTranslator]::FromHtml("#1e1e2e"))
    $g.FillEllipse($bg, 2, 2, 60, 60)
    $bg.Dispose()

    $purpleTop = [System.Drawing.ColorTranslator]::FromHtml("#c084fc")
    $purpleBottom = [System.Drawing.ColorTranslator]::FromHtml("#7c3aed")
    $rect = New-Object System.Drawing.RectangleF(23, 0, 18, 28)
    for ($i = 0; $i -lt 5; $i++) {
        $state = $g.Save()
        $g.TranslateTransform(32, 32)
        $g.RotateTransform(72 * $i)
        $g.TranslateTransform(-32, -32)
        $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect, $purpleTop, $purpleBottom, 90.0)
        $g.FillEllipse($brush, $rect)
        $brush.Dispose()
        $g.Restore($state)
    }

    $center = New-Object System.Drawing.SolidBrush([System.Drawing.ColorTranslator]::FromHtml("#f5c2e7"))
    $g.FillEllipse($center, 25, 25, 14, 14)
    $center.Dispose()
    $hole = New-Object System.Drawing.SolidBrush([System.Drawing.ColorTranslator]::FromHtml("#1e1e2e"))
    $g.FillEllipse($hole, 29, 29, 6, 6)
    $hole.Dispose()

    $g.Dispose()
    return $bmp
}

$sizes = @(16, 24, 32, 48, 64, 128, 256)
$pngs = @()
foreach ($size in $sizes) {
    $bmp = New-FlowerBitmap $size
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $pngs += ,$ms.ToArray()
    if ($size -eq 256) { $bmp.Save((Join-Path $OutDir "icon.png"), [System.Drawing.Imaging.ImageFormat]::Png) }
    $bmp.Dispose()
    $ms.Dispose()
}

# ICO container with PNG-compressed entries (supported by Windows Vista+).
$icoPath = Join-Path $OutDir "icon.ico"
$fs = [System.IO.File]::Create($icoPath)
$bw = New-Object System.IO.BinaryWriter($fs)
$bw.Write([UInt16]0)              # reserved
$bw.Write([UInt16]1)              # type: icon
$bw.Write([UInt16]$sizes.Count)   # image count
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $size = $sizes[$i]
    $data = $pngs[$i]
    $bw.Write([Byte]$(if ($size -ge 256) { 0 } else { $size }))
    $bw.Write([Byte]$(if ($size -ge 256) { 0 } else { $size }))
    $bw.Write([Byte]0)             # palette
    $bw.Write([Byte]0)             # reserved
    $bw.Write([UInt16]1)           # planes
    $bw.Write([UInt16]32)          # bit depth
    $bw.Write([UInt32]$data.Length)
    $bw.Write([UInt32]$offset)
    $offset += $data.Length
}
foreach ($data in $pngs) { $bw.Write($data) }
$bw.Flush(); $bw.Dispose(); $fs.Dispose()

Write-Host "Wrote $icoPath ($((Get-Item $icoPath).Length) bytes, sizes: $($sizes -join ', '))"
Write-Host "Wrote $(Join-Path $OutDir 'icon.png')"
