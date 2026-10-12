!include nsDialogs.nsh

Var StartupDialog
Var StartupKeepControl
Var StartupAutoControl
Var StartupSilentControl
Var StartupExistingData
Var StartupKeep
Var StartupAuto
Var StartupSilent
Var StartupChoiceProvided
Var StartupPreviousRun

!define STARTUP_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"

!macro TIME_REMIND_STARTUP_INIT
  StrCpy $StartupKeep 0
  StrCpy $StartupAuto 0
  StrCpy $StartupSilent 0
  StrCpy $StartupChoiceProvided 0
  StrCpy $StartupExistingData 0
  ; Capture before the maintenance page can uninstall the previous version.
  ReadRegStr $StartupPreviousRun HKCU "${STARTUP_RUN_KEY}" "${PRODUCTNAME}"
  ; A previous installation may not have launched the app yet.
  IfFileExists "$APPDATA\${BUNDLEID}\time-remind.db" startup_keep_existing
  IfFileExists "$APPDATA\${BUNDLEID}\installer-startup.json" startup_keep_existing startup_init_done
  startup_keep_existing:
    StrCpy $StartupExistingData 1
    StrCpy $StartupKeep 1
  startup_init_done:
!macroend

!macro TIME_REMIND_STARTUP_PAGE
  Page custom TimeRemindStartupPage TimeRemindStartupLeave

  Function TimeRemindStartupPage
    ${If} $PassiveMode = 1
    ${OrIf} $UpdateMode = 1
      Abort
    ${EndIf}
    IfSilent 0 +2
      Abort

    ; An explicit uninstall with data removal turns this into a fresh install.
    IfFileExists "$APPDATA\${BUNDLEID}\time-remind.db" startup_page_create
    IfFileExists "$APPDATA\${BUNDLEID}\installer-startup.json" startup_page_create
      StrCpy $StartupExistingData 0
      StrCpy $StartupKeep 0

    startup_page_create:
    !insertmacro MUI_HEADER_TEXT "$(startupTitle)" "$(startupSubtitle)"
    nsDialogs::Create 1018
    Pop $StartupDialog
    ${If} $StartupDialog == error
      Abort
    ${EndIf}

    ${NSD_CreateLabel} 0 0 100% 18u "$(startupIntro)"
    Pop $0
    ${NSD_CreateCheckbox} 0 22u 100% 14u "$(startupKeep)"
    Pop $StartupKeepControl
    ${NSD_SetState} $StartupKeepControl $StartupKeep
    ${NSD_OnClick} $StartupKeepControl TimeRemindStartupControls
    ${If} $StartupExistingData = 0
      ShowWindow $StartupKeepControl ${SW_HIDE}
    ${EndIf}

    ${NSD_CreateCheckbox} 0 42u 100% 14u "$(startupAuto)"
    Pop $StartupAutoControl
    ${NSD_SetState} $StartupAutoControl $StartupAuto
    ${NSD_OnClick} $StartupAutoControl TimeRemindStartupControls
    ${NSD_CreateLabel} 12u 58u 94% 18u "$(startupAutoHint)"
    Pop $0

    ${NSD_CreateCheckbox} 12u 80u 94% 14u "$(startupSilent)"
    Pop $StartupSilentControl
    ${NSD_SetState} $StartupSilentControl $StartupSilent
    ${NSD_OnClick} $StartupSilentControl TimeRemindStartupControls
    ${NSD_CreateLabel} 24u 96u 90% 18u "$(startupSilentHint)"
    Pop $0

    ${NSD_CreateLabel} 0 120u 100% 18u "$(startupFooter)"
    Pop $0
    Call TimeRemindUpdateStartupControls
    nsDialogs::Show
  FunctionEnd

  Function TimeRemindStartupControls
    Pop $0 ; nsDialogs click callbacks receive the control handle on the stack.
    Call TimeRemindUpdateStartupControls
  FunctionEnd

  Function TimeRemindUpdateStartupControls
    ${NSD_GetState} $StartupKeepControl $StartupKeep
    ${NSD_GetState} $StartupAutoControl $StartupAuto
    ${NSD_GetState} $StartupSilentControl $StartupSilent
    ${If} $StartupKeep = ${BST_CHECKED}
      EnableWindow $StartupAutoControl 0
      EnableWindow $StartupSilentControl 0
    ${Else}
      EnableWindow $StartupAutoControl 1
      ${If} $StartupAuto = ${BST_CHECKED}
        EnableWindow $StartupSilentControl 1
      ${Else}
        StrCpy $StartupSilent 0
        ${NSD_SetState} $StartupSilentControl ${BST_UNCHECKED}
        EnableWindow $StartupSilentControl 0
      ${EndIf}
    ${EndIf}
  FunctionEnd

  Function TimeRemindStartupLeave
    ${NSD_GetState} $StartupKeepControl $StartupKeep
    ${NSD_GetState} $StartupAutoControl $StartupAuto
    ${NSD_GetState} $StartupSilentControl $StartupSilent
    StrCpy $StartupChoiceProvided 0
    ${If} $StartupKeep <> ${BST_CHECKED}
      StrCpy $StartupChoiceProvided 1
    ${EndIf}
    ${If} $StartupAuto <> ${BST_CHECKED}
      StrCpy $StartupSilent 0
    ${EndIf}
  FunctionEnd

  Function TimeRemindApplyStartupOptions
    ${If} $StartupChoiceProvided = 0
      ; Silent/passive/update installs and "keep" never overwrite preferences.
      ; Restore an existing Run entry if maintenance removed it or the path changed.
      ${If} $StartupPreviousRun != ""
        WriteRegStr HKCU "${STARTUP_RUN_KEY}" "${PRODUCTNAME}" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --autostart'
      ${EndIf}
      Return
    ${EndIf}

    StrCpy $R3 "false"
    StrCpy $R4 "false"
    ${If} $StartupAuto = ${BST_CHECKED}
      StrCpy $R3 "true"
      ${If} $StartupSilent = ${BST_CHECKED}
        StrCpy $R4 "true"
      ${EndIf}
    ${EndIf}

    ; A unique request ID makes a leftover marker safe if deletion later fails.
    System::Call 'ole32::CoCreateGuid(g .r0) i.r1'
    ${If} $1 <> 0
      Goto startup_save_failed
    ${EndIf}
    ClearErrors
    CreateDirectory "$APPDATA\${BUNDLEID}"
    FileOpen $R1 "$APPDATA\${BUNDLEID}\installer-startup.json.tmp" w
    IfErrors startup_save_failed
    FileWrite $R1 '{$\"schema$\":1,$\"request_id$\":$\"$0$\",$\"auto_start$\":$R3,$\"silent_start$\":$R4}'
    FileClose $R1
    IfErrors startup_save_failed
    CopyFiles /SILENT "$APPDATA\${BUNDLEID}\installer-startup.json.tmp" "$APPDATA\${BUNDLEID}\installer-startup.json"
    IfErrors startup_save_failed
    Delete "$APPDATA\${BUNDLEID}\installer-startup.json.tmp"

    ClearErrors
    ${If} $StartupAuto = ${BST_CHECKED}
      WriteRegStr HKCU "${STARTUP_RUN_KEY}" "${PRODUCTNAME}" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --autostart'
      IfErrors startup_save_failed
      ; Match the autostart plugin when the user explicitly enables startup.
      WriteRegBin HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "${PRODUCTNAME}" 020000000000000000000000
      IfErrors startup_save_failed
    ${Else}
      ; Deleting a missing entry is already the requested disabled state.
      ReadRegStr $R2 HKCU "${STARTUP_RUN_KEY}" "${PRODUCTNAME}"
      ${IfNot} ${Errors}
        DeleteRegValue HKCU "${STARTUP_RUN_KEY}" "${PRODUCTNAME}"
        IfErrors startup_save_failed
      ${EndIf}
    ${EndIf}
    ClearErrors ; A missing Run entry above is a successful disabled state.
    Return

    startup_save_failed:
      MessageBox MB_OK|MB_ICONEXCLAMATION "$(startupSaveFailed)"
      Abort
  FunctionEnd
!macroend
