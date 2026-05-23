!include LogicLib.nsh

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
  DetailPrint "Application dependencies are managed from Settings"
  !insertmacro REFRESH_INSTALLED_SHORTCUT_ICONS
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  RMDir /r "$INSTDIR\dependencies"
  DeleteRegKey SHCTX "${MANUPRODUCTKEY}"
  DeleteRegKey /ifempty SHCTX "${MANUKEY}"
!macroend
