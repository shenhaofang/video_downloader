param(
  [string]$ArchivePath = "",

  [string]$ArchiveUrl = "",

  [Parameter(Mandatory = $true)]
  [string]$InstallRoot,

  [Parameter(Mandatory = $true)]
  [string]$ExpectedSha256
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Write-InstallerLog {
  param([Parameter(Mandatory = $true)][string]$Message)
  Write-Output $Message
}

function Test-RequiredMediaTools {
  param([Parameter(Mandatory = $true)][string]$Root)

  $ffmpeg = Join-Path $Root "bin\ffmpeg.exe"
  $ffprobe = Join-Path $Root "bin\ffprobe.exe"
  return ((Test-Path -LiteralPath $ffmpeg -PathType Leaf) -and (Test-Path -LiteralPath $ffprobe -PathType Leaf))
}

if (Test-RequiredMediaTools -Root $InstallRoot) {
  Write-InstallerLog "Required FFmpeg media tools already installed"
  exit 0
}

$downloadRoot = $null
$extractRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("video-downloader-ffmpeg-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $extractRoot -Force | Out-Null

try {
  if (-not $ArchivePath) {
    if (-not $ArchiveUrl) {
      throw "Either ArchivePath or ArchiveUrl is required"
    }

    $downloadRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("video-downloader-ffmpeg-download-" + [System.Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $downloadRoot -Force | Out-Null
    $ArchivePath = Join-Path $downloadRoot "ffmpeg-win64-lgpl.zip"

    Write-InstallerLog "Downloading required FFmpeg archive"
    Invoke-WebRequest -Uri $ArchiveUrl -OutFile $ArchivePath -UseBasicParsing -Headers @{ "User-Agent" = "video-downloader-installer" }
  }

  if (-not (Test-Path -LiteralPath $ArchivePath -PathType Leaf)) {
    throw "FFmpeg archive not found: $ArchivePath"
  }
  if ((Get-Item -LiteralPath $ArchivePath).Length -eq 0) {
    throw "FFmpeg archive download returned an empty file"
  }

  Write-InstallerLog "Verifying FFmpeg archive"
  $actualHash = (Get-FileHash -LiteralPath $ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actualHash -ne $ExpectedSha256.ToLowerInvariant()) {
    throw "FFmpeg archive checksum mismatch. Expected $ExpectedSha256, got $actualHash"
  }

  Write-InstallerLog "Extracting FFmpeg media tools"
  Expand-Archive -LiteralPath $ArchivePath -DestinationPath $extractRoot -Force
  $sourceRoot = Get-ChildItem -LiteralPath $extractRoot -Directory | Select-Object -First 1
  if ($null -eq $sourceRoot) {
    throw "FFmpeg archive does not contain an extracted root directory"
  }

  $ffmpeg = Join-Path $sourceRoot.FullName "bin\ffmpeg.exe"
  $ffprobe = Join-Path $sourceRoot.FullName "bin\ffprobe.exe"
  if (-not (Test-Path -LiteralPath $ffmpeg -PathType Leaf)) {
    throw "ffmpeg.exe missing from archive"
  }
  if (-not (Test-Path -LiteralPath $ffprobe -PathType Leaf)) {
    throw "ffprobe.exe missing from archive"
  }

  if (Test-Path -LiteralPath $InstallRoot) {
    Remove-Item -LiteralPath $InstallRoot -Recurse -Force
  }
  New-Item -ItemType Directory -Path $InstallRoot -Force | Out-Null

  Get-ChildItem -LiteralPath $sourceRoot.FullName -Force |
    Copy-Item -Destination $InstallRoot -Recurse -Force

  Write-InstallerLog "FFmpeg media tools installed"
} finally {
  if (Test-Path -LiteralPath $extractRoot) {
    Remove-Item -LiteralPath $extractRoot -Recurse -Force
  }
  if ($null -ne $downloadRoot -and (Test-Path -LiteralPath $downloadRoot)) {
    Remove-Item -LiteralPath $downloadRoot -Recurse -Force
  }
}
