<#
.SYNOPSIS
  Register the Lilac native messaging host for Chrome and Firefox on Windows.

.DESCRIPTION
  Orchard is a single executable: the browser launches orchard.exe and it
  detects host mode from the browser's --parent-window argument / stdio pipes.
  There is no separate lilac-host.exe to install.

.PARAMETER BinaryPath
  Absolute path to orchard.exe. Default: ..\apps\orchard\src-tauri\target\debug\orchard.exe

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
    throw "Binary not found: $BinaryPath. Build first: cd apps\orchard\src-tauri; cargo build"
}

$RecordsRoot = [System.IO.Path]::GetFullPath($RecordsRoot)
New-Item -ItemType Directory -Force -Path $RecordsRoot | Out-Null

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
Write-Host "Wrote manifest: $manifestPath (host: $BinaryPath)"

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

# Write records_root into the shared Orchard config (merge, don't clobber).
# Read as one string: an old installer could leave the file with no newlines,
# and Get-Content would then return a single line and `+=` would concatenate.
$configRoot = Join-Path $env:USERPROFILE ".orchard\.lilac"
New-Item -ItemType Directory -Force -Path $configRoot | Out-Null
$configPath = Join-Path $configRoot "config.yaml"
$text = ""
if (Test-Path $configPath) {
    $text = [System.IO.File]::ReadAllText($configPath)
}
$text = $text.TrimStart([char]0xFEFF)
# Drop every previous records_root scalar, including concatenated ones.
$text = [regex]::Replace($text, "records_root:\s*""[^""]*""", "")
$text = [regex]::Replace($text, "records_root:\s*'[^']*'", "")
$text = [regex]::Replace($text, "records_root:\s*[^\s]+", "")
$lines = @($text -split "\r?\n" | Where-Object { $_.Trim() -ne "" })
if (-not ($lines | Where-Object { $_ -match '^\s*version\s*:' })) {
    $lines = @("version: 1") + $lines
}
$yamlPath = $RecordsRoot.Replace("'", "''")
$lines += "records_root: '$yamlPath'"
[System.IO.File]::WriteAllLines($configPath, $lines, (New-Object System.Text.UTF8Encoding($false)))
Write-Host "Set records_root = $RecordsRoot in $configPath"

Write-Host ""
Write-Host "Done. Restart your browser, then load the extension from: ..\extension"
Write-Host "Chrome extension ID needed for allowed_origins: re-run with -ExtensionId <id> if not provided."
