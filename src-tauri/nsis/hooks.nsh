!define TIME_REMIND_NSIS_DIR "${__FILEDIR__}"
!include "${__FILEDIR__}\startup.nsh"
!include "${__FILEDIR__}\startup-languages.nsh"

!macro NSIS_HOOK_POSTINSTALL
  CreateDirectory "$APPDATA\${BUNDLEID}"

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
  ${ElseIf} $LANGUAGE = ${LANG_KHMER}
    StrCpy $R0 "km-KH"
  ${ElseIf} $LANGUAGE = ${LANG_SPANISH}
    StrCpy $R0 "es-ES"
  ${ElseIf} $LANGUAGE = ${LANG_PORTUGUESEBR}
    StrCpy $R0 "pt-BR"
  ${ElseIf} $LANGUAGE = ${LANG_INDONESIAN}
    StrCpy $R0 "id-ID"
  ${ElseIf} $LANGUAGE = ${LANG_ITALIAN}
    StrCpy $R0 "it-IT"
  ${ElseIf} $LANGUAGE = ${LANG_RUSSIAN}
    StrCpy $R0 "ru-RU"
  ${ElseIf} $LANGUAGE = ${LANG_TURKISH}
    StrCpy $R0 "tr-TR"
  ${ElseIf} $LANGUAGE = ${LANG_ARABIC}
    StrCpy $R0 "ar"
  ${ElseIf} $LANGUAGE = ${LANG_HINDI}
    StrCpy $R0 "hi-IN"
  ${EndIf}

  FileOpen $R1 "$APPDATA\${BUNDLEID}\installer-language.txt" w
  FileWrite $R1 "$R0"
  FileClose $R1

  Call TimeRemindApplyStartupOptions
!macroend
