<#
.SYNOPSIS
  Register the Lilac native messaging host for Chrome and Firefox on Windows.

.PARAMETER BinaryPath
  Absolute path to lilac-host.exe. Default: ..\target\debug\lilac-host.exe (relative to this script).

.PARAMETER ExtensionId
  Chrome extension ID (from chrome://extensions). Required for Chrome allowed_origins.
  Firefox uses the fixed id "lilac@orchid" (set in manifest.json).

.EXAMPLE
  .\install-host.ps1 -ExtensionId "abcdefghijklmnopabcdefghijklmnop"
#>
param(
    [string]$BinaryPath = (Join-Path $PSScriptRoot "..\target\debug\lilac-host.exe"),
    [string]$ExtensionId = ""
)

$ErrorActionPreference = "Stop"

$BinaryPath = (Resolve-Path $BinaryPath).Path
if (-not (Test-Path $BinaryPath)) {
    throw "Binary not found: $BinaryPath. Build first: cargo build -p lilac-host"
}

$manifestDir = Join-Path $env:LOCALAPPDATA "Lilac"
New-Item -ItemType Directory -Force -Path $manifestDir | Out-Null
$manifestPath = Join-Path $manifestDir "lilac_host.json"

$allowedOrigins = @()
if ($ExtensionId -ne "") {
    $allowedOrigins = @("chrome-extension://$ExtensionId/")
} else {
    Write-Warning "No -ExtensionId provided. Chrome allowed_origins will be empty; native messaging will fail in Chrome until you re-run with the ID. Get it from chrome://extensions (Developer mode)."
}

$manifest = [ordered]@{
    name = "lilac_host"
    description = "Lilac record storage native host"
    path = $BinaryPath
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

Write-Host ""
Write-Host "Done. Restart your browser, then load the extension from: ..\extension"
Write-Host "Chrome extension ID needed for allowed_origins: re-run with -ExtensionId <id> if not provided."