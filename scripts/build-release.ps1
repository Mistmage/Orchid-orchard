<#
.SYNOPSIS
  Build release artifacts for Lilac (host binary + extension zip).
#>
param(
    [string]$OutDir = (Join-Path $PSScriptRoot "..\dist")
)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $root

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Write-Host "Building lilac-host (release)..."
cargo build -p lilac-host --release
$bin = Join-Path $root "target\release\lilac-host.exe"
if (-not (Test-Path $bin)) { $bin = Join-Path $root "target\release\lilac-host" }
Copy-Item $bin (Join-Path $OutDir "lilac-host$([System.IO.Path]::GetExtension($bin))") -Force

Write-Host "Zipping extension..."
$ext = Join-Path $root "extension"
$zip = Join-Path $OutDir "lilac-extension.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path "$ext\*" -DestinationPath $zip

Write-Host ""
Write-Host "Artifacts in $OutDir :"
Get-ChildItem $OutDir