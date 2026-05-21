!include LogicLib.nsh

!define FFMPEG_ARCHIVE_RESOURCE "resources\vendor\ffmpeg\ffmpeg-win64-lgpl.zip"
!define FFMPEG_ARCHIVE_SHA256 "d3c0d41c26b64bb42abbf9051a9494bc67185b6d9fa57798f20efb0e0213caf7"

!macro REFRESH_SHORTCUT_ICON LINK_PATH
  IfFileExists "${LINK_PATH}" 0 +2
    CreateShortcut "${LINK_PATH}" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0
!macroend

!macro REFRESH_INSTALLED_SHORTCUT_ICONS
  DetailPrint "Refreshing shortcut icons"
  !if "${STARTMENUFOLDER}" != ""
    !insertmacro REFRESH_SHORTCUT_ICON "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
  !else
    !insertmacro REFRESH_SHORTCUT_ICON "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  !endif
  !insertmacro REFRESH_SHORTCUT_ICON "$DESKTOP\${PRODUCTNAME}.lnk"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  SetDetailsView show
  DetailPrint "Preparing FFmpeg media tools"

  CreateDirectory "$INSTDIR\dependencies"
  StrCpy $0 "$INSTDIR\${FFMPEG_ARCHIVE_RESOURCE}"

  DetailPrint "Installing bundled FFmpeg media tools"
  nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\resources\install-media-tools.ps1" -ArchivePath "$0" -InstallRoot "$INSTDIR\dependencies\ffmpeg" -ExpectedSha256 "${FFMPEG_ARCHIVE_SHA256}"'
  Pop $1
  ${If} $1 != 0
    MessageBox MB_ICONSTOP "Failed to install FFmpeg media tools. Check installer details for more information."
    Abort
  ${EndIf}

  Delete "$0"
  DetailPrint "FFmpeg media tools installed"
  !insertmacro REFRESH_INSTALLED_SHORTCUT_ICONS
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  RMDir /r "$INSTDIR\dependencies"
  RMDir /r "$INSTDIR\resources\vendor"
  DeleteRegKey SHCTX "${MANUPRODUCTKEY}"
  DeleteRegKey /ifempty SHCTX "${MANUKEY}"
!macroend
