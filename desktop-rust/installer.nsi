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

Name "Pirate Cinema 0.6.4"
OutFile "${APP_OUTPUT}"
InstallDir "$PROGRAMFILES64\Pirate Cinema"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
SetCompressorDictSize 64
ShowInstDetails nevershow
ShowUninstDetails nevershow
Icon "${APP_ICON}"
UninstallIcon "${APP_ICON}"
VIProductVersion "0.6.4.0"
VIAddVersionKey "ProductName" "Pirate Cinema"
VIAddVersionKey "FileDescription" "Pirate Cinema installer"
VIAddVersionKey "FileVersion" "0.6.4.0"
VIAddVersionKey "LegalCopyright" "Pirate Cinema contributors"

!define MUI_WELCOMEPAGE_TEXT "Pirate Cinema 0.6.4 is the Rust/Dioxus desktop application. Existing media history and TorrServer data remain in place during the update."
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
  ; Stop only the bundled server from this installation directory. An external
  ; TorrServer with the same process name must remain untouched.
  nsExec::Exec '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -Command "$$target=[IO.Path]::GetFullPath(''$INSTDIR\torrserver\TorrServer-windows-amd64.exe''); Get-Process -Name ''TorrServer-windows-amd64'' -ErrorAction SilentlyContinue | Where-Object { $$_.Path -and [IO.Path]::GetFullPath($$_.Path) -eq $$target } | Stop-Process -Force"'
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

  SetOutPath "$INSTDIR\torrserver"
  File "${APP_SOURCE}\torrserver\TorrServer-windows-amd64.exe"

  SetOutPath "$INSTDIR\mpv"
  File "${APP_SOURCE}\mpv\mpv.exe"
  File "${APP_SOURCE}\mpv\d3dcompiler_43.dll"
  SetOutPath "$INSTDIR\mpv\fonts"
  File "${APP_SOURCE}\mpv\fonts\NotoEmoji-Regular.ttf"
  File "${APP_SOURCE}\mpv\fonts\NotoNaskhArabic-Regular.ttf"
  File "${APP_SOURCE}\mpv\fonts\NotoSans-Regular.ttf"
  File "${APP_SOURCE}\mpv\fonts\NotoSansHans-Regular.otf"
  File "${APP_SOURCE}\mpv\fonts\NotoSansJP-Regular.otf"
  File "${APP_SOURCE}\mpv\fonts\NotoSansKR-Regular.otf"
  SetOutPath "$INSTDIR\mpv\mpv"
  File "${APP_SOURCE}\mpv\mpv\fonts.conf"
  File "${APP_SOURCE}\mpv\mpv\mpv.conf"

  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\Uninstall Pirate Cinema.exe"
  CreateShortcut "$DESKTOP\Pirate Cinema.lnk" "$INSTDIR\pirate-cinema.exe" "" "$INSTDIR\pirate-cinema.ico"
  CreateShortcut "$SMPROGRAMS\Pirate Cinema.lnk" "$INSTDIR\pirate-cinema.exe" "" "$INSTDIR\pirate-cinema.ico"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PirateCinema" "DisplayName" "Pirate Cinema"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PirateCinema" "DisplayVersion" "0.6.4"
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
  nsExec::Exec '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -Command "$$target=[IO.Path]::GetFullPath(''$INSTDIR\torrserver\TorrServer-windows-amd64.exe''); Get-Process -Name ''TorrServer-windows-amd64'' -ErrorAction SilentlyContinue | Where-Object { $$_.Path -and [IO.Path]::GetFullPath($$_.Path) -eq $$target } | Stop-Process -Force"'
  Sleep 700
  Delete "$DESKTOP\Pirate Cinema.lnk"
  Delete "$SMPROGRAMS\Pirate Cinema.lnk"
  Delete "$INSTDIR\pirate-cinema.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD_PARTY_NOTICES.md"
  Delete "$INSTDIR\pirate-cinema.ico"
  Delete "$INSTDIR\torrserver\TorrServer-windows-amd64.exe"
  Delete "$INSTDIR\mpv\mpv.exe"
  Delete "$INSTDIR\mpv\d3dcompiler_43.dll"
  Delete "$INSTDIR\mpv\fonts\NotoEmoji-Regular.ttf"
  Delete "$INSTDIR\mpv\fonts\NotoNaskhArabic-Regular.ttf"
  Delete "$INSTDIR\mpv\fonts\NotoSans-Regular.ttf"
  Delete "$INSTDIR\mpv\fonts\NotoSansHans-Regular.otf"
  Delete "$INSTDIR\mpv\fonts\NotoSansJP-Regular.otf"
  Delete "$INSTDIR\mpv\fonts\NotoSansKR-Regular.otf"
  Delete "$INSTDIR\mpv\mpv\fonts.conf"
  Delete "$INSTDIR\mpv\mpv\mpv.conf"
  Delete "$INSTDIR\Uninstall Pirate Cinema.exe"
  RMDir "$INSTDIR\mpv\fonts"
  RMDir "$INSTDIR\mpv\mpv"
  RMDir "$INSTDIR\mpv"
  RMDir "$INSTDIR\torrserver"
  RMDir "$INSTDIR"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PirateCinema"
  ReadRegStr $0 HKCU "Software\Classes\magnet\shell\open\command" ""
  StrCmp $0 "$\"$INSTDIR\pirate-cinema.exe$\" $\"%1$\"" 0 +2
  DeleteRegKey HKCU "Software\Classes\magnet"
SectionEnd
