Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

!ifndef APP_SOURCE
  !error "Pass /DAPP_SOURCE=... with the verified application folder"
!endif
!ifndef APP_OUTPUT
  !error "Pass /DAPP_OUTPUT=... with a new installer path"
!endif
!ifndef APP_ICON
  !error "Pass /DAPP_ICON=... with the application icon"
!endif
!ifndef APP_VERSION
  !error "Pass /DAPP_VERSION=... with the application version"
!endif

Name "Pirate Cinema ${APP_VERSION}"
OutFile "${APP_OUTPUT}"
InstallDir "$PROGRAMFILES64\Pirate Cinema"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
SetCompressorDictSize 64
ShowInstDetails nevershow
ShowUninstDetails nevershow
Icon "${APP_ICON}"
UninstallIcon "${APP_ICON}"
VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey "ProductName" "Pirate Cinema"
VIAddVersionKey "FileDescription" "Pirate Cinema installer"
VIAddVersionKey "FileVersion" "${APP_VERSION}.0"
VIAddVersionKey "LegalCopyright" "Pirate Cinema contributors"

!define MUI_WELCOMEPAGE_TEXT "Pirate Cinema ${APP_VERSION}. Existing media history remains in place during the update."
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "English"

Function .onInit
  SetRegView 64
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "Pirate Cinema requires 64-bit Windows."
    Abort
  ${EndIf}
FunctionEnd

Section "Pirate Cinema" SEC_APP
  nsExec::Exec '"$SYSDIR\taskkill.exe" /IM pirate-cinema.exe /T /F'
  nsExec::Exec '"$SYSDIR\taskkill.exe" /IM "Pirate Cinema.exe" /T /F'
  Sleep 700
  IfFileExists "$INSTDIR\Uninstall Pirate Cinema.exe" uninstall_old continue_install
  uninstall_old:
    ExecWait '"$INSTDIR\Uninstall Pirate Cinema.exe" /S _?=$INSTDIR'
  continue_install:
  SetOutPath "$INSTDIR"
  File "${APP_SOURCE}\pirate-cinema.exe"
  File "${APP_SOURCE}\README.md"
  File "${APP_SOURCE}\LICENSE"
  File "${APP_SOURCE}\THIRD_PARTY_NOTICES.md"
  File /oname=pirate-cinema.ico "${APP_ICON}"
  ; The release workflow always stages the official VC++ runtime.
  ; NSIS has no !ifexist directive, so include it unconditionally.
  File "${APP_SOURCE}\vc_redist.x64.exe"
  SetOutPath "$INSTDIR\ffmpeg"
  File "${APP_SOURCE}\ffmpeg\ffmpeg.exe"

  IfFileExists "$INSTDIR\vc_redist.x64.exe" 0 +2
    ExecWait '"$INSTDIR\vc_redist.x64.exe" /install /quiet /norestart' $0

  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\Uninstall Pirate Cinema.exe"
  CreateShortcut "$DESKTOP\Pirate Cinema.lnk" "$INSTDIR\pirate-cinema.exe" "" "$INSTDIR\pirate-cinema.ico"
  CreateShortcut "$SMPROGRAMS\Pirate Cinema.lnk" "$INSTDIR\pirate-cinema.exe" "" "$INSTDIR\pirate-cinema.ico"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PirateCinema" "DisplayName" "Pirate Cinema"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PirateCinema" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PirateCinema" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PirateCinema" "UninstallString" "$\"$INSTDIR\Uninstall Pirate Cinema.exe$\""
  WriteRegStr HKCU "Software\Classes\magnet" "" "URL:Magnet link"
  WriteRegStr HKCU "Software\Classes\magnet" "URL Protocol" ""
  WriteRegStr HKCU "Software\Classes\magnet\DefaultIcon" "" "$INSTDIR\pirate-cinema.ico"
  WriteRegStr HKCU "Software\Classes\magnet\shell\open\command" "" "$\"$INSTDIR\pirate-cinema.exe$\" $\"%1$\""
SectionEnd

Section "Uninstall"
  nsExec::Exec '"$SYSDIR\taskkill.exe" /IM pirate-cinema.exe /T /F'
  Sleep 700
  Delete "$DESKTOP\Pirate Cinema.lnk"
  Delete "$SMPROGRAMS\Pirate Cinema.lnk"
  Delete "$INSTDIR\pirate-cinema.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD_PARTY_NOTICES.md"
  Delete "$INSTDIR\pirate-cinema.ico"
  Delete "$INSTDIR\vc_redist.x64.exe"
  Delete "$INSTDIR\ffmpeg\ffmpeg.exe"
  Delete "$INSTDIR\Uninstall Pirate Cinema.exe"
  RMDir "$INSTDIR\ffmpeg"
  RMDir "$INSTDIR"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PirateCinema"
  ReadRegStr $0 HKCU "Software\Classes\magnet\shell\open\command" ""
  StrCmp $0 "$\"$INSTDIR\pirate-cinema.exe$\" $\"%1$\"" 0 +2
  DeleteRegKey HKCU "Software\Classes\magnet"
SectionEnd
