!include LogicLib.nsh

!define FFMPEG_ARCHIVE_URL "https://github.com/shenhaofang/video_downloader/releases/latest/download/ffmpeg-win64-lgpl.zip"
!define FFMPEG_ARCHIVE_SHA256 "D3C0D41C26B64BB42ABBF9051A9494BC67185B6D9FA57798F20EFB0E0213CAF7"

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

!macro ENSURE_REQUIRED_MEDIA_TOOLS
  IfFileExists "$INSTDIR\dependencies\ffmpeg\bin\ffmpeg.exe" 0 install_required_media_tools
  IfFileExists "$INSTDIR\dependencies\ffmpeg\bin\ffprobe.exe" media_tools_installed install_required_media_tools

media_tools_installed:
  DetailPrint "FFmpeg media tools already installed"
  Goto media_tools_done

install_required_media_tools:
  DetailPrint "Installing required FFmpeg media tools"
  IfFileExists "$INSTDIR\resources\install-media-tools.ps1" 0 media_tools_script_missing
  nsExec::ExecToLog 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\resources\install-media-tools.ps1" -ArchiveUrl "${FFMPEG_ARCHIVE_URL}" -InstallRoot "$INSTDIR\dependencies\ffmpeg" -ExpectedSha256 "${FFMPEG_ARCHIVE_SHA256}"'
  Pop $0
  ${If} $0 != 0
    MessageBox MB_ICONSTOP "Required FFmpeg installation failed with exit code $0."
    Abort
  ${EndIf}
  Goto media_tools_done

media_tools_script_missing:
  MessageBox MB_ICONSTOP "Required installer script is missing."
  Abort

media_tools_done:
!macroend

!macro NSIS_HOOK_POSTINSTALL
  SetDetailsView show
  !insertmacro ENSURE_REQUIRED_MEDIA_TOOLS
  !insertmacro REFRESH_INSTALLED_SHORTCUT_ICONS
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  RMDir /r "$INSTDIR\dependencies"
  DeleteRegKey SHCTX "${MANUPRODUCTKEY}"
  DeleteRegKey /ifempty SHCTX "${MANUKEY}"
!macroend
