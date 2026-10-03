Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"

!ifndef WEB_OUTPUT
  !error "Pass /DWEB_OUTPUT=... with the web installer path"
!endif
!ifndef APP_ICON
  !error "Pass /DAPP_ICON=... with the application icon"
!endif

Name "Pirate Cinema 0.6.4 Web Installer"
OutFile "${WEB_OUTPUT}"
RequestExecutionLevel user
Icon "${APP_ICON}"
VIProductVersion "0.6.4.0"
VIAddVersionKey "ProductName" "Pirate Cinema Web Installer"
VIAddVersionKey "FileDescription" "Pirate Cinema web installer"
VIAddVersionKey "FileVersion" "0.6.4.0"
VIAddVersionKey "LegalCopyright" "Pirate Cinema contributors"
ShowInstDetails nevershow

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "English"

Section "Pirate Cinema"
  StrCpy $0 "$TEMP\Pirate-Cinema-Setup-0.6.4-win-x64.exe"
  Delete "$0"
  nsExec::ExecToStack '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$ProgressPreference=''SilentlyContinue''; Invoke-WebRequest -UseBasicParsing -Uri ''https://github.com/cyberboy1999/pirate-cinema/releases/download/v0.6.4/Pirate-Cinema-Setup-0.6.4-win-x64.exe'' -OutFile ''$0''"'
  Pop $1
  Pop $2
  ${If} $1 != 0
    MessageBox MB_OK|MB_ICONSTOP "Download failed. Check the Internet connection and try again."
    Abort
  ${EndIf}
  IfFileExists "$0" +3 0
    MessageBox MB_OK|MB_ICONSTOP "The application package was not downloaded."
    Abort
  ExecWait '"$0"' $1
  ${If} $1 != 0
    MessageBox MB_OK|MB_ICONSTOP "The application installer returned error $1."
    Abort
  ${EndIf}
  Delete "$0"
SectionEnd
