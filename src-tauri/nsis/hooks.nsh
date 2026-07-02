!macro NSIS_HOOK_POSTINSTALL
  CreateDirectory "$APPDATA\com.hpk.time-remind"

  StrCpy $R0 "en-US"
  ${If} $LANGUAGE = ${LANG_SIMPCHINESE}
    StrCpy $R0 "zh-CN"
  ${ElseIf} $LANGUAGE = ${LANG_TRADCHINESE}
    StrCpy $R0 "zh-TW"
  ${ElseIf} $LANGUAGE = ${LANG_JAPANESE}
    StrCpy $R0 "ja-JP"
  ${ElseIf} $LANGUAGE = ${LANG_KOREAN}
    StrCpy $R0 "ko-KR"
  ${ElseIf} $LANGUAGE = ${LANG_FRENCH}
    StrCpy $R0 "fr-FR"
  ${ElseIf} $LANGUAGE = ${LANG_GERMAN}
    StrCpy $R0 "de-DE"
  ${ElseIf} $LANGUAGE = ${LANG_VIETNAMESE}
    StrCpy $R0 "vi-VN"
  ${ElseIf} $LANGUAGE = ${LANG_THAI}
    StrCpy $R0 "th-TH"
  ${ElseIf} $LANGUAGE = ${LANG_MALAY}
    StrCpy $R0 "ms-MY"
  ${EndIf}

  FileOpen $R1 "$APPDATA\com.hpk.time-remind\installer-language.txt" w
  FileWrite $R1 "$R0"
  FileClose $R1
!macroend
