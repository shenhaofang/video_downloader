param(
  [string]$Notes = ""
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$configPath = Join-Path $root "src-tauri\tauri.conf.json"
$bundleDir = Join-Path $root "src-tauri\target\release\bundle\nsis"
$installerScriptResource = "resources/install-media-tools.ps1"
$ffmpegArchiveResource = "resources/vendor/ffmpeg/ffmpeg-win64-lgpl.zip"
$originalConfig = [System.IO.File]::ReadAllText($configPath)
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$previousLocation = Get-Location

function Read-TauriConfig {
  Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
}

function Write-TauriConfig($Config) {
  $json = $Config | ConvertTo-Json -Depth 100
  [System.IO.File]::WriteAllText($configPath, $json, $utf8NoBom)
}

function Set-TauriResources([string[]]$Resources) {
  $config = Read-TauriConfig
  $config.bundle.resources = @($Resources)
  Write-TauriConfig $config
}

function Invoke-TauriBuild {
  Push-Location $root
  try {
    & npm.cmd run tauri -- build --ci
    if ($LASTEXITCODE -ne 0) {
      throw "Tauri build failed with exit code $LASTEXITCODE"
    }
  } finally {
    Pop-Location
  }
}

function Copy-LatestNsisArtifact([string]$DestinationName) {
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

  $destination = Join-Path $bundleDir $DestinationName
  Copy-Item -LiteralPath $installer.FullName -Destination $destination -Force
  Copy-Item -LiteralPath $signaturePath -Destination "$destination.sig" -Force
  Get-Item -LiteralPath $destination
}

try {
  Set-Location $root
  $version = [string](Read-TauriConfig).version
  $fullName = "Video.Downloader_$version`_x64-full-setup.exe"
  $updateName = "Video.Downloader_$version`_x64-app-update.exe"

  Set-TauriResources @($installerScriptResource, $ffmpegArchiveResource)
  Invoke-TauriBuild
  $fullInstaller = Copy-LatestNsisArtifact $fullName

  Set-TauriResources @($installerScriptResource)
  Invoke-TauriBuild
  $updateInstaller = Copy-LatestNsisArtifact $updateName

  [System.IO.File]::WriteAllText($configPath, $originalConfig, $utf8NoBom)

  $metadataScript = Join-Path $PSScriptRoot "create-updater-latest-json.ps1"
  & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $metadataScript `
    -Version $version `
    -Notes $Notes `
    -InstallerPath $updateInstaller.FullName `
    -AssetName $updateInstaller.Name

  Write-Output "Full installer: $($fullInstaller.FullName)"
  Write-Output "App-update installer: $($updateInstaller.FullName)"
} finally {
  [System.IO.File]::WriteAllText($configPath, $originalConfig, $utf8NoBom)
  Set-Location $previousLocation
}
