!include "WinMessages.nsh"

!macro customInit
  FindWindow $0 "" "Pirate Cinema"
  ${If} $0 != 0
    SendMessage $0 ${WM_CLOSE} 0 0
    Sleep 2000
  ${EndIf}
  nsExec::ExecToStack '"$SYSDIR\taskkill.exe" /F /T /IM "Pirate Cinema.exe"'
  Pop $0
  Pop $1
!macroend
