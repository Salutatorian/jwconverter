!macro NSIS_HOOK_POSTINSTALL
  ; Place a stable README next to the app for the finish-page checkbox.
  ${If} ${FileExists} "$INSTDIR\resources\README.txt"
    CopyFiles /SILENT "$INSTDIR\resources\README.txt" "$INSTDIR\README.txt"
  ${ElseIf} ${FileExists} "$INSTDIR\resources\resources\README.txt"
    CopyFiles /SILENT "$INSTDIR\resources\resources\README.txt" "$INSTDIR\README.txt"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_PREUNINSTALL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
!macroend
