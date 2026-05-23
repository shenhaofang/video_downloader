param(
  [string]$Repo = "shenhaofang/video_downloader",
  [string]$Version = "",
  [string]$Notes = "",
  [string]$AssetName = ""
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$tauriConfigPath = Join-Path $root "src-tauri\tauri.conf.json"
$bundleDir = Join-Path $root "src-tauri\target\release\bundle\nsis"

if (-not $Version) {
  $config = Get-Content -LiteralPath $tauriConfigPath -Raw | ConvertFrom-Json
  $Version = [string]$config.version
}

$installer = Get-ChildItem -LiteralPath $bundleDir -Filter "*_x64-setup.exe" |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1
if ($null -eq $installer) {
  throw "NSIS installer not found under $bundleDir"
}

$signaturePath = "$($installer.FullName).sig"
if (-not (Test-Path -LiteralPath $signaturePath -PathType Leaf)) {
  throw "Updater signature not found: $signaturePath"
}

if (-not $AssetName) {
  $AssetName = $installer.Name -replace "\s+", "."
}

$encodedAssetName = [System.Uri]::EscapeDataString($AssetName)
$releaseTag = "v$Version"
$json = [ordered]@{
  version = $Version
  notes = $Notes
  pub_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
  platforms = [ordered]@{
    "windows-x86_64" = [ordered]@{
      signature = (Get-Content -LiteralPath $signaturePath -Raw).Trim()
      url = "https://github.com/$Repo/releases/download/$releaseTag/$encodedAssetName"
    }
  }
} | ConvertTo-Json -Depth 8

$outputPath = Join-Path $bundleDir "latest.json"
$json | Set-Content -LiteralPath $outputPath -Encoding UTF8
Write-Output $outputPath
