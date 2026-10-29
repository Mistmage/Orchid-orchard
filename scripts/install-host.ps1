<#
.SYNOPSIS
  Register the Lilac native messaging host for Chrome and Firefox on Windows.

.PARAMETER BinaryPath
  Absolute path to lilac-host.exe. Default: ..\target\debug\lilac-host.exe (relative to this script).

.PARAMETER ExtensionId
  Chrome extension ID (from chrome://extensions). Required for Chrome allowed_origins.
  Firefox uses the fixed id "lilac@orchid" (set in manifest.json).

.PARAMETER RecordsRoot
  Records folder path to store in config. Default: <current directory>\Lilac
  (so running from the project folder uses the project's Lilac folder for dev).

.EXAMPLE
  .\install-host.ps1 -ExtensionId "abcdefghijklmnopabcdefghijklmnop"
  .\install-host.ps1 -ExtensionId "..." -RecordsRoot "D:\Notes\Lilac"
#>
param(
    [string]$BinaryPath = (Join-Path $PSScriptRoot "..\apps\orchard\src-tauri\target\debug\orchard.exe"),
    [string]$ExtensionId = "",
    [string]$RecordsRoot = (Join-Path (Get-Location) "Lilac")
)

$ErrorActionPreference = "Stop"

$BinaryPath = (Resolve-Path $BinaryPath).Path
if (-not (Test-Path $BinaryPath)) {
    throw "Binary not found: $BinaryPath. Build first: cargo build -p lilac-host"
}

$RecordsRoot = [System.IO.Path]::GetFullPath($RecordsRoot)
New-Item -ItemType Directory -Force -Path $RecordsRoot | Out-Null

# Orchard is the single binary; native messaging needs a --host flag, so use a wrapper.
$manifestDir = Join-Path $env:LOCALAPPDATA "Lilac"
New-Item -ItemType Directory -Force -Path $manifestDir | Out-Null
$manifestPath = Join-Path $manifestDir "lilac_host.json"
$wrapperPath = Join-Path $manifestDir "lilac-host.bat"
$wrapper = "@echo off`r`n`"$BinaryPath`" --host"
Set-Content -Path $wrapperPath -Value $wrapper -Encoding ASCII
Write-Host "Wrote wrapper: $wrapperPath"

$allowedOrigins = @()
if ($ExtensionId -ne "") {
    $allowedOrigins = @("chrome-extension://$ExtensionId/")
} else {
    Write-Warning "No -ExtensionId provided. Chrome allowed_origins will be empty; native messaging will fail in Chrome until you re-run with the ID. Get it from chrome://extensions (Developer mode)."
}

$manifest = [ordered]@{
    name = "lilac_host"
    description = "Lilac record storage native host"
    path = $wrapperPath
    type = "stdio"
    allowed_origins = $allowedOrigins
    allowed_extensions = @("lilac@orchid")
}
$manifest | ConvertTo-Json -Depth 4 | Set-Content -Path $manifestPath -Encoding UTF8
Write-Host "Wrote manifest: $manifestPath"

# Chrome registry
$chromeKey = "HKCU:\Software\Google\Chrome\NativeMessagingHosts\lilac_host"
New-Item -Path $chromeKey -Force | Out-Null
Set-ItemProperty -Path $chromeKey -Name "(Default)" -Value $manifestPath
Write-Host "Registered Chrome host."

# Firefox registry
$ffKey = "HKCU:\Software\Mozilla\NativeMessagingHosts\lilac_host"
New-Item -Path $ffKey -Force | Out-Null
Set-ItemProperty -Path $ffKey -Name "(Default)" -Value $manifestPath
Write-Host "Registered Firefox host."

# Write records_root into config.yaml (merge, don't clobber).
$configRoot = Join-Path $env:USERPROFILE ".lilac"
New-Item -ItemType Directory -Force -Path $configRoot | Out-Null
$configPath = Join-Path $configRoot "config.yaml"
$lines = @()
if (Test-Path $configPath) {
    $lines = Get-Content $configPath
    $lines = $lines | Where-Object { $_ -notmatch '^\s*records_root\s*:' }
}
if ($lines.Count -eq 0) {
    $lines = @("version: 1")
}
$lines += "records_root: `"$RecordsRoot`""
$lines | Set-Content -Path $configPath -Encoding UTF8
Write-Host "Set records_root = $RecordsRoot in $configPath"

Write-Host ""
Write-Host "Done. Restart your browser, then load the extension from: ..\extension"
Write-Host "Chrome extension ID needed for allowed_origins: re-run with -ExtensionId <id> if not provided."