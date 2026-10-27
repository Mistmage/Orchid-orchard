param()
$ErrorActionPreference = "SilentlyContinue"
$manifestDir = Join-Path $env:LOCALAPPDATA "Lilac"
$manifestPath = Join-Path $manifestDir "lilac_host.json"
Remove-Item -Path $manifestPath -Force
Remove-Item -Path "HKCU:\Software\Google\Chrome\NativeMessagingHosts\lilac_host" -Recurse -Force
Remove-Item -Path "HKCU:\Software\Mozilla\NativeMessagingHosts\lilac_host" -Recurse -Force
Write-Host "Uninstalled Lilac native host (Chrome + Firefox)."
