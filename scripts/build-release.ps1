<#
.SYNOPSIS
  Build release artifacts for Lilac + Orchard.

.NOTES
  Orchard is the single executable (GUI + native messaging host).
  The extension is the only other artifact.
#>
param(
    [string]$OutDir = (Join-Path $PSScriptRoot "..\dist")
)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $root

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Write-Host "Generating application icon..."
& (Join-Path $PSScriptRoot "make-icon.ps1")

Write-Host "Zipping extension..."
$ext = Join-Path $root "extension"
$zip = Join-Path $OutDir "lilac-extension.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path "$ext\*" -DestinationPath $zip

Write-Host "Building Orchard frontend..."
Push-Location (Join-Path $root "apps\orchard")
npm run build
Pop-Location

Write-Host "Building Orchard (Tauri, release)..."
Push-Location (Join-Path $root "apps\orchard\src-tauri")
cargo build --release
Pop-Location
$orchardBin = Join-Path $root "apps\orchard\src-tauri\target\release\orchard.exe"
if (-not (Test-Path $orchardBin)) {
    throw "Build did not produce $orchardBin"
}
Copy-Item $orchardBin (Join-Path $OutDir "orchard.exe") -Force

# Clean up the old split-binary artifact if it is still lying around.
$oldHost = Join-Path $OutDir "lilac-host.exe"
if (Test-Path $oldHost) { Remove-Item $oldHost -Force }

Write-Host ""
Write-Host "Artifacts in $OutDir :"
Get-ChildItem $OutDir
