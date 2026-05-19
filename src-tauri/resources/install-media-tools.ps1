param(
  [Parameter(Mandatory = $true)]
  [string]$ArchivePath,

  [Parameter(Mandatory = $true)]
  [string]$InstallRoot,

  [Parameter(Mandatory = $true)]
  [string]$ExpectedSha256
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $ArchivePath -PathType Leaf)) {
  throw "Archive not found: $ArchivePath"
}

$actualHash = (Get-FileHash -LiteralPath $ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualHash -ne $ExpectedSha256.ToLowerInvariant()) {
  throw "FFmpeg archive checksum mismatch. Expected $ExpectedSha256, got $actualHash"
}

$extractRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("video-downloader-ffmpeg-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $extractRoot -Force | Out-Null

try {
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
} finally {
  if (Test-Path -LiteralPath $extractRoot) {
    Remove-Item -LiteralPath $extractRoot -Recurse -Force
  }
}
