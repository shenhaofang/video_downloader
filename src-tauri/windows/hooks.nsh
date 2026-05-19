!include LogicLib.nsh

!define FFMPEG_ARCHIVE_URL "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-05-18-18-09/ffmpeg-n7.1.4-5-ged860ef7d9-win64-lgpl-shared-7.1.zip"
!define FFMPEG_ARCHIVE_SHA256 "d3c0d41c26b64bb42abbf9051a9494bc67185b6d9fa57798f20efb0e0213caf7"

!macro NSIS_HOOK_POSTINSTALL
  SetDetailsView show
  DetailPrint "Preparing FFmpeg media tools"

  CreateDirectory "$INSTDIR\resources\media-tools"
  StrCpy $0 "$TEMP\video-downloader-ffmpeg.zip"

  DetailPrint "Downloading FFmpeg media tools"
  NSISdl::download /TIMEOUT=300000 "${FFMPEG_ARCHIVE_URL}" "$0"
  Pop $1
  ${If} $1 != "success"
    MessageBox MB_ICONSTOP "Failed to download FFmpeg media tools: $1"
    Abort
  ${EndIf}

  DetailPrint "Verifying and extracting FFmpeg media tools"
  nsExec::ExecToLog 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\resources\install-media-tools.ps1" -ArchivePath "$0" -InstallRoot "$INSTDIR\resources\media-tools\ffmpeg" -ExpectedSha256 "${FFMPEG_ARCHIVE_SHA256}"'
  Pop $1
  ${If} $1 != 0
    MessageBox MB_ICONSTOP "Failed to install FFmpeg media tools. Check installer details for more information."
    Abort
  ${EndIf}

  Delete "$0"
  DetailPrint "FFmpeg media tools installed"
!macroend
