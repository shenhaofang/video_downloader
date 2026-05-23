param(
  [string]$ArchivePath = "",

  [string]$ArchiveUrl = "",

  [Parameter(Mandatory = $true)]
  [string]$InstallRoot,

  [Parameter(Mandatory = $true)]
  [string]$ExpectedSha256,

  [int]$DownloadTimeoutSeconds = 600,

  [int]$ConnectTimeoutSeconds = 20,

  [int]$DownloadStallTimeoutSeconds = 120,

  [int]$DownloadRetries = 2
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

function Quote-ProcessArgument {
  param([Parameter(Mandatory = $true)][string]$Value)
  '"' + $Value.Replace('"', '\"') + '"'
}

function Get-FileSizeOrZero {
  param([Parameter(Mandatory = $true)][string]$Path)

  $item = Get-Item -LiteralPath $Path -ErrorAction SilentlyContinue
  if ($null -eq $item) {
    return 0
  }
  return $item.Length
}

function Write-DownloadProgress {
  param([Parameter(Mandatory = $true)][long]$Bytes)

  Write-InstallerLog ("Downloading FFmpeg archive: {0:N1} MB" -f ($Bytes / 1MB))
}

function Save-ArchiveWithCurl {
  param(
    [Parameter(Mandatory = $true)][string]$CurlPath,
    [Parameter(Mandatory = $true)][string]$Url,
    [Parameter(Mandatory = $true)][string]$Destination
  )

  $arguments = @(
    "--fail",
    "--location",
    "--silent",
    "--show-error",
    "--connect-timeout", [string]$ConnectTimeoutSeconds,
    "--max-time", [string]$DownloadTimeoutSeconds,
    "--speed-time", [string]$DownloadStallTimeoutSeconds,
    "--speed-limit", "1024",
    "--retry", [string]$DownloadRetries,
    "--retry-delay", "2",
    "--retry-all-errors",
    "--output", $Destination,
    "--user-agent", "video-downloader-installer",
    $Url
  )
  $argumentLine = ($arguments | ForEach-Object { Quote-ProcessArgument ([string]$_) }) -join " "

  $process = Start-Process -FilePath $CurlPath -ArgumentList $argumentLine -PassThru -WindowStyle Hidden
  $startedAt = Get-Date
  $lastBytes = -1
  $lastChangeAt = Get-Date
  $lastReportedBytes = -1

  while (-not $process.HasExited) {
    Start-Sleep -Seconds 5
    $bytes = Get-FileSizeOrZero -Path $Destination
    if ($bytes -ne $lastBytes) {
      $lastBytes = $bytes
      $lastChangeAt = Get-Date
    }
    if ($bytes -ne $lastReportedBytes) {
      Write-DownloadProgress -Bytes $bytes
      $lastReportedBytes = $bytes
    }

    if (((Get-Date) - $lastChangeAt).TotalSeconds -gt $DownloadStallTimeoutSeconds) {
      Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
      throw "curl.exe download stalled for more than $DownloadStallTimeoutSeconds seconds"
    }
    if (((Get-Date) - $startedAt).TotalSeconds -gt ($DownloadTimeoutSeconds + 30)) {
      Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
      throw "curl.exe download exceeded timeout of $DownloadTimeoutSeconds seconds"
    }
  }

  $process.WaitForExit()
  if ($process.ExitCode -ne 0) {
    throw "curl.exe exited with code $($process.ExitCode)"
  }
}

function Save-ArchiveFromUrl {
  param(
    [Parameter(Mandatory = $true)][string]$Url,
    [Parameter(Mandatory = $true)][string]$Destination
  )

  Write-InstallerLog "Downloading required FFmpeg archive"
  $curl = Get-Command curl.exe -ErrorAction SilentlyContinue
  if ($null -ne $curl) {
    Write-InstallerLog "Using curl.exe with timeout and retry limits"
    try {
      Save-ArchiveWithCurl -CurlPath $curl.Source -Url $Url -Destination $Destination
      return
    } catch {
      if (Test-Path -LiteralPath $Destination) {
        Remove-Item -LiteralPath $Destination -Force
      }
      Write-InstallerLog "curl.exe download failed: $($_.Exception.Message); falling back to PowerShell"
    }
  }

  Invoke-WebRequest `
    -Uri $Url `
    -OutFile $Destination `
    -UseBasicParsing `
    -TimeoutSec $DownloadTimeoutSeconds `
    -Headers @{ "User-Agent" = "video-downloader-installer" }
}

if (Test-RequiredMediaTools -Root $InstallRoot) {
  Write-InstallerLog "Required FFmpeg media tools already installed"
  exit 0
}

$downloadRoot = $null
$extractRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("video-downloader-ffmpeg-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $extractRoot -Force | Out-Null

try {
  if ($ArchivePath -and -not (Test-Path -LiteralPath $ArchivePath -PathType Leaf)) {
    if ($ArchiveUrl) {
      Write-InstallerLog "Bundled FFmpeg archive not found; downloading release asset"
      $ArchivePath = ""
    } else {
      throw "FFmpeg archive not found: $ArchivePath"
    }
  }

  if (-not $ArchivePath) {
    if (-not $ArchiveUrl) {
      throw "Either ArchivePath or ArchiveUrl is required"
    }

    $downloadRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("video-downloader-ffmpeg-download-" + [System.Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $downloadRoot -Force | Out-Null
    $ArchivePath = Join-Path $downloadRoot "ffmpeg-win64-lgpl.zip"

    Save-ArchiveFromUrl -Url $ArchiveUrl -Destination $ArchivePath
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
