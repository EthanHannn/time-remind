!macro StartupLanguage ID TITLE SUBTITLE INTRO KEEP AUTO AUTOHINT SILENT SILENTHINT FOOTER FAILED
  LangString startupTitle ${ID} "${TITLE}"
  LangString startupSubtitle ${ID} "${SUBTITLE}"
  LangString startupIntro ${ID} "${INTRO}"
  LangString startupKeep ${ID} "${KEEP}"
  LangString startupAuto ${ID} "${AUTO}"
  LangString startupAutoHint ${ID} "${AUTOHINT}"
  LangString startupSilent ${ID} "${SILENT}"
  LangString startupSilentHint ${ID} "${SILENTHINT}"
  LangString startupFooter ${ID} "${FOOTER}"
  LangString startupSaveFailed ${ID} "${FAILED}"
!macroend

!macro TIME_REMIND_STARTUP_LANGUAGES
  !insertmacro StartupLanguage ${LANG_ENGLISH} \
    "Startup settings" "Choose how Time Remind starts with Windows." \
    "You can change these options later in the app's settings." \
    "Keep my current startup settings (uncheck to change)" \
    "Start with Windows" "Start reminders automatically when you sign in to Windows." \
    "Start silently in the system tray" "At sign-in, keep reminders running without opening the main window." \
    "Opening the app yourself, including after setup, still shows its window." \
    "Could not save startup settings. Please retry setup or change them in the app."
  !insertmacro StartupLanguage ${LANG_SIMPCHINESE} \
    "启动设置" "选择 Time Remind 随 Windows 启动的方式。" \
    "这些选项以后也可以在应用的设置中修改。" \
    "保留当前启动设置（取消勾选后可修改）" \
    "开机启动" "登录 Windows 后自动运行，及时开始提醒。" \
    "静默启动，仅驻留系统托盘" "开机启动时不弹出主窗口，提醒功能照常运行。" \
    "安装后立即运行或手动打开应用时，仍会显示主窗口。" \
    "无法保存启动设置。请重试安装，或在应用设置中修改。"
  !insertmacro StartupLanguage ${LANG_TRADCHINESE} \
    "啟動設定" "選擇 Time Remind 隨 Windows 啟動的方式。" \
    "這些選項之後也可以在應用程式設定中修改。" \
    "保留目前的啟動設定（取消勾選後可修改）" \
    "開機啟動" "登入 Windows 後自動執行，即時開始提醒。" \
    "靜默啟動，僅駐留系統匣" "開機啟動時不開啟主視窗，提醒功能照常執行。" \
    "安裝後立即執行或手動開啟應用程式時，仍會顯示主視窗。" \
    "無法儲存啟動設定。請重試安裝，或在應用程式設定中修改。"
  !insertmacro StartupLanguage ${LANG_JAPANESE} \
    "起動設定" "Windows へのサインイン時の動作を選択します。" \
    "これらの項目は、後からアプリの設定で変更できます。" \
    "現在の起動設定を維持する（変更するにはチェックを外す）" \
    "Windows と同時に起動する" "サインイン時に自動で起動し、リマインダーを開始します。" \
    "ウィンドウを開かず、トレイに常駐する" "自動起動時はトレイで動作し、通常どおり通知します。" \
    "インストール後や手動で起動した場合は、ウィンドウを表示します。" \
    "起動設定を保存できませんでした。再インストールするか、アプリの設定で変更してください。"
  !insertmacro StartupLanguage ${LANG_KOREAN} \
    "시작 설정" "Windows 로그인 시 실행 방식을 선택하세요." \
    "나중에 앱 설정에서 변경할 수 있습니다." \
    "현재 시작 설정 유지 (변경하려면 선택 해제)" \
    "Windows 시작 시 실행" "Windows 로그인 시 자동으로 실행하고 알림을 시작합니다." \
    "창을 열지 않고 시스템 트레이에서 시작" "자동 시작 시 기본 창을 열지 않으며 알림은 정상 작동합니다." \
    "설치 후 실행하거나 직접 앱을 열면 기본 창이 표시됩니다." \
    "시작 설정을 저장하지 못했습니다. 설치를 다시 시도하거나 앱 설정에서 변경하세요."
  !insertmacro StartupLanguage ${LANG_FRENCH} \
    "Options de démarrage" "Choisissez le comportement au démarrage de Windows." \
    "Vous pourrez modifier ces choix dans les paramètres de l’application." \
    "Conserver mes réglages actuels (décocher pour modifier)" \
    "Démarrer avec Windows" "Lancer les rappels automatiquement à l’ouverture de session." \
    "Démarrer discrètement dans la zone de notification" "Ne pas ouvrir la fenêtre au démarrage. Les rappels restent actifs." \
    "Un lancement manuel, y compris après l’installation, ouvre la fenêtre." \
    "Impossible d’enregistrer ces choix. Réessayez ou utilisez les paramètres de l’application."
  !insertmacro StartupLanguage ${LANG_GERMAN} \
    "Starteinstellungen" "Wählen Sie das Verhalten beim Windows-Start." \
    "Diese Optionen lassen sich später in der App ändern." \
    "Bisherige Einstellungen behalten (zum Ändern abwählen)" \
    "Mit Windows starten" "Erinnerungen nach der Windows-Anmeldung automatisch starten." \
    "Ohne Fenster im Infobereich starten" "Beim Autostart bleibt das Fenster geschlossen. Erinnerungen laufen weiter." \
    "Beim manuellen Öffnen, auch nach der Installation, erscheint das Fenster." \
    "Die Einstellungen konnten nicht gespeichert werden. Wiederholen Sie die Installation oder ändern Sie sie in der App."
  !insertmacro StartupLanguage ${LANG_VIETNAMESE} \
    "Cài đặt khởi động" "Chọn cách ứng dụng khởi động cùng Windows." \
    "Bạn có thể thay đổi các tùy chọn này trong phần cài đặt của ứng dụng." \
    "Giữ cài đặt hiện tại (bỏ chọn để thay đổi)" \
    "Khởi động cùng Windows" "Tự chạy và bắt đầu nhắc nhở khi đăng nhập Windows." \
    "Khởi động ẩn trong khay hệ thống" "Không mở cửa sổ khi tự khởi động. Lời nhắc vẫn hoạt động." \
    "Mở ứng dụng thủ công hoặc ngay sau khi cài đặt vẫn hiển thị cửa sổ." \
    "Không thể lưu cài đặt. Hãy thử cài đặt lại hoặc thay đổi trong ứng dụng."
  !insertmacro StartupLanguage ${LANG_THAI} \
    "การตั้งค่าเริ่มต้น" "เลือกวิธีเริ่มแอปพร้อม Windows" \
    "คุณสามารถเปลี่ยนตัวเลือกเหล่านี้ได้ในการตั้งค่าแอป" \
    "คงการตั้งค่าปัจจุบัน (ยกเลิกเพื่อเปลี่ยน)" \
    "เริ่มพร้อม Windows" "เริ่มแอปและการแจ้งเตือนอัตโนมัติเมื่อเข้าสู่ระบบ" \
    "เริ่มแบบเงียบในถาดระบบ" "ไม่เปิดหน้าต่างเมื่อเริ่มอัตโนมัติ แต่ยังแจ้งเตือนตามปกติ" \
    "การเปิดแอปเองหรือหลังติดตั้งยังคงแสดงหน้าต่างหลัก" \
    "บันทึกการตั้งค่าไม่ได้ โปรดลองติดตั้งอีกครั้งหรือแก้ไขในการตั้งค่าแอป"
  !insertmacro StartupLanguage ${LANG_MALAY} \
    "Tetapan permulaan" "Pilih cara aplikasi bermula bersama Windows." \
    "Pilihan ini boleh diubah kemudian dalam tetapan aplikasi." \
    "Kekalkan tetapan semasa (nyahpilih untuk mengubah)" \
    "Mulakan bersama Windows" "Jalankan aplikasi dan peringatan semasa log masuk Windows." \
    "Mulakan secara senyap dalam dulang sistem" "Jangan buka tetingkap ketika mula automatik. Peringatan terus berjalan." \
    "Membuka aplikasi sendiri, termasuk selepas pemasangan, memaparkan tetingkap." \
    "Tetapan tidak dapat disimpan. Cuba pasang semula atau ubah dalam aplikasi."
!macroend
