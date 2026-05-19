!include LogicLib.nsh

!define FFMPEG_ARCHIVE_RESOURCE "resources\vendor\ffmpeg\ffmpeg-win64-lgpl.zip"
!define FFMPEG_ARCHIVE_SHA256 "d3c0d41c26b64bb42abbf9051a9494bc67185b6d9fa57798f20efb0e0213caf7"

!macro NSIS_HOOK_POSTINSTALL
  SetDetailsView show
  DetailPrint "Preparing FFmpeg media tools"

  CreateDirectory "$INSTDIR\resources\media-tools"
  StrCpy $0 "$INSTDIR\${FFMPEG_ARCHIVE_RESOURCE}"

  DetailPrint "Installing bundled FFmpeg media tools"
  nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\resources\install-media-tools.ps1" -ArchivePath "$0" -InstallRoot "$INSTDIR\resources\media-tools\ffmpeg" -ExpectedSha256 "${FFMPEG_ARCHIVE_SHA256}"'
  Pop $1
  ${If} $1 != 0
    MessageBox MB_ICONSTOP "Failed to install FFmpeg media tools. Check installer details for more information."
    Abort
  ${EndIf}

  Delete "$0"
  DetailPrint "FFmpeg media tools installed"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  RMDir /r "$INSTDIR\resources\media-tools"
  RMDir /r "$INSTDIR\resources\vendor"
  DeleteRegKey SHCTX "${MANUPRODUCTKEY}"
  DeleteRegKey /ifempty SHCTX "${MANUKEY}"
!macroend
