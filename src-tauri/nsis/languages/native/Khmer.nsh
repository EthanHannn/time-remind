;Language: Khmer (1107)
;Time Remind translation, based on the NSIS English language structure.

!insertmacro LANGFILE "Khmer" "Khmer" "ភាសាខ្មែរ" "Khmer"

!ifdef MUI_WELCOMEPAGE
  ${LangFileString} MUI_TEXT_WELCOME_INFO_TITLE "ស្វាគមន៍មកកាន់ការដំឡើង $(^NameDA)"
  ${LangFileString} MUI_TEXT_WELCOME_INFO_TEXT "កម្មវិធីនេះនឹងណែនាំអ្នកក្នុងការដំឡើង $(^NameDA)។$\r$\n$\r$\nសូមបិទកម្មវិធីផ្សេងទៀតមុនចាប់ផ្ដើម។$\r$\n$\r$\n$_CLICK"
!endif

!ifdef MUI_UNWELCOMEPAGE
  ${LangFileString} MUI_UNTEXT_WELCOME_INFO_TITLE "ស្វាគមន៍មកកាន់ការលុប $(^NameDA)"
  ${LangFileString} MUI_UNTEXT_WELCOME_INFO_TEXT "កម្មវិធីនេះនឹងណែនាំអ្នកក្នុងការលុប $(^NameDA)។$\r$\n$\r$\nសូមប្រាកដថា $(^NameDA) មិនកំពុងដំណើរការ។$\r$\n$\r$\n$_CLICK"
!endif

!ifdef MUI_LICENSEPAGE
  ${LangFileString} MUI_TEXT_LICENSE_TITLE "កិច្ចព្រមព្រៀងអាជ្ញាបណ្ណ"
  ${LangFileString} MUI_TEXT_LICENSE_SUBTITLE "សូមអានលក្ខខណ្ឌអាជ្ញាបណ្ណមុនដំឡើង $(^NameDA)។"
  ${LangFileString} MUI_INNERTEXT_LICENSE_BOTTOM "បើអ្នកយល់ព្រម សូមចុច ខ្ញុំយល់ព្រម ដើម្បីបន្ត។ អ្នកត្រូវយល់ព្រមដើម្បីដំឡើង $(^NameDA)។"
  ${LangFileString} MUI_INNERTEXT_LICENSE_BOTTOM_CHECKBOX "បើអ្នកយល់ព្រម សូមធីកប្រអប់ខាងក្រោម។ អ្នកត្រូវយល់ព្រមដើម្បីដំឡើង $(^NameDA)។ $_CLICK"
  ${LangFileString} MUI_INNERTEXT_LICENSE_BOTTOM_RADIOBUTTONS "បើអ្នកយល់ព្រម សូមជ្រើសជម្រើសទីមួយខាងក្រោម។ អ្នកត្រូវយល់ព្រមដើម្បីដំឡើង $(^NameDA)។ $_CLICK"
!endif

!ifdef MUI_UNLICENSEPAGE
  ${LangFileString} MUI_UNTEXT_LICENSE_TITLE "កិច្ចព្រមព្រៀងអាជ្ញាបណ្ណ"
  ${LangFileString} MUI_UNTEXT_LICENSE_SUBTITLE "សូមអានលក្ខខណ្ឌអាជ្ញាបណ្ណមុនលុប $(^NameDA)។"
  ${LangFileString} MUI_UNINNERTEXT_LICENSE_BOTTOM "បើអ្នកយល់ព្រម សូមចុច ខ្ញុំយល់ព្រម ដើម្បីបន្ត។ អ្នកត្រូវយល់ព្រមដើម្បីលុប $(^NameDA)។"
  ${LangFileString} MUI_UNINNERTEXT_LICENSE_BOTTOM_CHECKBOX "បើអ្នកយល់ព្រម សូមធីកប្រអប់ខាងក្រោម។ អ្នកត្រូវយល់ព្រមដើម្បីលុប $(^NameDA)។ $_CLICK"
  ${LangFileString} MUI_UNINNERTEXT_LICENSE_BOTTOM_RADIOBUTTONS "បើអ្នកយល់ព្រម សូមជ្រើសជម្រើសទីមួយខាងក្រោម។ អ្នកត្រូវយល់ព្រមដើម្បីលុប $(^NameDA)។ $_CLICK"
!endif

!ifdef MUI_LICENSEPAGE | MUI_UNLICENSEPAGE
  ${LangFileString} MUI_INNERTEXT_LICENSE_TOP "ចុច Page Down ដើម្បីអានផ្នែកបន្ទាប់នៃកិច្ចព្រមព្រៀង។"
!endif

!ifdef MUI_COMPONENTSPAGE
  ${LangFileString} MUI_TEXT_COMPONENTS_TITLE "ជ្រើសសមាសភាគ"
  ${LangFileString} MUI_TEXT_COMPONENTS_SUBTITLE "ជ្រើសមុខងាររបស់ $(^NameDA) ដែលចង់ដំឡើង។"
!endif

!ifdef MUI_UNCOMPONENTSPAGE
  ${LangFileString} MUI_UNTEXT_COMPONENTS_TITLE "ជ្រើសសមាសភាគ"
  ${LangFileString} MUI_UNTEXT_COMPONENTS_SUBTITLE "ជ្រើសមុខងាររបស់ $(^NameDA) ដែលចង់លុប។"
!endif

!ifdef MUI_COMPONENTSPAGE | MUI_UNCOMPONENTSPAGE
  ${LangFileString} MUI_INNERTEXT_COMPONENTS_DESCRIPTION_TITLE "ការពិពណ៌នា"
  !ifndef NSIS_CONFIG_COMPONENTPAGE_ALTERNATIVE
    ${LangFileString} MUI_INNERTEXT_COMPONENTS_DESCRIPTION_INFO "ជ្រើសសមាសភាគដើម្បីមើលការពិពណ៌នា។"
  !else
    ${LangFileString} MUI_INNERTEXT_COMPONENTS_DESCRIPTION_INFO "ជ្រើសសមាសភាគដើម្បីមើលការពិពណ៌នា។"
  !endif
!endif

!ifdef MUI_DIRECTORYPAGE
  ${LangFileString} MUI_TEXT_DIRECTORY_TITLE "ជ្រើសទីតាំងដំឡើង"
  ${LangFileString} MUI_TEXT_DIRECTORY_SUBTITLE "ជ្រើសថតសម្រាប់ដំឡើង $(^NameDA)។"
!endif

!ifdef MUI_UNDIRECTORYPAGE
  ${LangFileString} MUI_UNTEXT_DIRECTORY_TITLE "ជ្រើសទីតាំងលុប"
  ${LangFileString} MUI_UNTEXT_DIRECTORY_SUBTITLE "ជ្រើសថតដែលត្រូវលុប $(^NameDA) ចេញពី។"
!endif

!ifdef MUI_INSTFILESPAGE
  ${LangFileString} MUI_TEXT_INSTALLING_TITLE "កំពុងដំឡើង"
  ${LangFileString} MUI_TEXT_INSTALLING_SUBTITLE "សូមរង់ចាំពេលដំឡើង $(^NameDA)។"
  ${LangFileString} MUI_TEXT_FINISH_TITLE "ការដំឡើងបានបញ្ចប់"
  ${LangFileString} MUI_TEXT_FINISH_SUBTITLE "ការដំឡើងបានជោគជ័យ។"
  ${LangFileString} MUI_TEXT_ABORT_TITLE "ការដំឡើងត្រូវបានបោះបង់"
  ${LangFileString} MUI_TEXT_ABORT_SUBTITLE "ការដំឡើងមិនបានបញ្ចប់ទេ។"
!endif

!ifdef MUI_UNINSTFILESPAGE
  ${LangFileString} MUI_UNTEXT_UNINSTALLING_TITLE "កំពុងលុប"
  ${LangFileString} MUI_UNTEXT_UNINSTALLING_SUBTITLE "សូមរង់ចាំពេលលុប $(^NameDA)។"
  ${LangFileString} MUI_UNTEXT_FINISH_TITLE "ការលុបបានបញ្ចប់"
  ${LangFileString} MUI_UNTEXT_FINISH_SUBTITLE "ការលុបបានជោគជ័យ។"
  ${LangFileString} MUI_UNTEXT_ABORT_TITLE "ការលុបត្រូវបានបោះបង់"
  ${LangFileString} MUI_UNTEXT_ABORT_SUBTITLE "ការលុបមិនបានបញ្ចប់ទេ។"
!endif

!ifdef MUI_FINISHPAGE
  ${LangFileString} MUI_TEXT_FINISH_INFO_TITLE "បញ្ចប់ការដំឡើង $(^NameDA)"
  ${LangFileString} MUI_TEXT_FINISH_INFO_TEXT "$(^NameDA) បានដំឡើងលើកុំព្យូទ័ររបស់អ្នក។$\r$\n$\r$\nចុច បញ្ចប់ ដើម្បីបិទកម្មវិធីដំឡើង។"
  ${LangFileString} MUI_TEXT_FINISH_INFO_REBOOT "ត្រូវចាប់ផ្ដើមកុំព្យូទ័រឡើងវិញដើម្បីបញ្ចប់ការដំឡើង $(^NameDA)។ តើចង់ចាប់ផ្ដើមឡើងវិញឥឡូវនេះទេ?"
!endif

!ifdef MUI_UNFINISHPAGE
  ${LangFileString} MUI_UNTEXT_FINISH_INFO_TITLE "បញ្ចប់ការលុប $(^NameDA)"
  ${LangFileString} MUI_UNTEXT_FINISH_INFO_TEXT "$(^NameDA) បានលុបចេញពីកុំព្យូទ័ររបស់អ្នក។$\r$\n$\r$\nចុច បញ្ចប់ ដើម្បីបិទ។"
  ${LangFileString} MUI_UNTEXT_FINISH_INFO_REBOOT "ត្រូវចាប់ផ្ដើមកុំព្យូទ័រឡើងវិញដើម្បីបញ្ចប់ការលុប $(^NameDA)។ តើចង់ចាប់ផ្ដើមឡើងវិញឥឡូវនេះទេ?"
!endif

!ifdef MUI_FINISHPAGE | MUI_UNFINISHPAGE
  ${LangFileString} MUI_TEXT_FINISH_REBOOTNOW "ចាប់ផ្ដើមឡើងវិញឥឡូវនេះ"
  ${LangFileString} MUI_TEXT_FINISH_REBOOTLATER "ខ្ញុំនឹងចាប់ផ្ដើមឡើងវិញពេលក្រោយ"
  ${LangFileString} MUI_TEXT_FINISH_RUN "ដំណើរការ $(^NameDA) (&R)"
  ${LangFileString} MUI_TEXT_FINISH_SHOWREADME "បង្ហាញឯកសារណែនាំ (&S)"
  ${LangFileString} MUI_BUTTONTEXT_FINISH "បញ្ចប់ (&F)"
!endif

!ifdef MUI_STARTMENUPAGE
  ${LangFileString} MUI_TEXT_STARTMENU_TITLE "ជ្រើសថតម៉ឺនុយចាប់ផ្ដើម"
  ${LangFileString} MUI_TEXT_STARTMENU_SUBTITLE "ជ្រើសថតម៉ឺនុយចាប់ផ្ដើមសម្រាប់ផ្លូវកាត់ $(^NameDA)។"
  ${LangFileString} MUI_INNERTEXT_STARTMENU_TOP "ជ្រើសថតសម្រាប់ផ្លូវកាត់កម្មវិធី។ អ្នកអាចបញ្ចូលឈ្មោះដើម្បីបង្កើតថតថ្មី។"
  ${LangFileString} MUI_INNERTEXT_STARTMENU_CHECKBOX "កុំបង្កើតផ្លូវកាត់"
!endif

!ifdef MUI_UNCONFIRMPAGE
  ${LangFileString} MUI_UNTEXT_CONFIRM_TITLE "លុប $(^NameDA)"
  ${LangFileString} MUI_UNTEXT_CONFIRM_SUBTITLE "លុប $(^NameDA) ចេញពីកុំព្យូទ័រ។"
!endif

!ifdef MUI_ABORTWARNING
  ${LangFileString} MUI_TEXT_ABORTWARNING "តើអ្នកចង់បិទការដំឡើង $(^Name) មែនទេ?"
!endif

!ifdef MUI_UNABORTWARNING
  ${LangFileString} MUI_UNTEXT_ABORTWARNING "តើអ្នកចង់បិទការលុប $(^Name) មែនទេ?"
!endif

!ifdef MULTIUSER_INSTALLMODEPAGE
  ${LangFileString} MULTIUSER_TEXT_INSTALLMODE_TITLE "ជ្រើសអ្នកប្រើ"
  ${LangFileString} MULTIUSER_TEXT_INSTALLMODE_SUBTITLE "ជ្រើសអ្នកប្រើដែលអាចប្រើ $(^NameDA)។"
  ${LangFileString} MULTIUSER_INNERTEXT_INSTALLMODE_TOP "ជ្រើសដំឡើង $(^NameDA) សម្រាប់ខ្លួនឯង ឬអ្នកប្រើទាំងអស់។ $(^ClickNext)"
  ${LangFileString} MULTIUSER_INNERTEXT_INSTALLMODE_ALLUSERS "ដំឡើងសម្រាប់អ្នកប្រើទាំងអស់"
  ${LangFileString} MULTIUSER_INNERTEXT_INSTALLMODE_CURRENTUSER "ដំឡើងសម្រាប់ខ្ញុំតែប៉ុណ្ណោះ"
!endif
