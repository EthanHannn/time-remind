mod app_log;
mod commands;
mod db;
mod fullscreen;
mod models;
pub mod power;
mod scheduler;

use chrono::{NaiveDateTime, Utc};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime, WebviewWindow,
};

#[cfg(target_os = "windows")]
use window_vibrancy::apply_mica;
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
#[cfg(target_os = "windows")]
use windows::Win32::Globalization::GetUserDefaultLocaleName;

/// 应用窗口材质
fn apply_window_material<R: Runtime>(window: &WebviewWindow<R>) {
    #[cfg(target_os = "windows")]
    {
        let _ = apply_mica(window, None);
    }
    #[cfg(target_os = "macos")]
    {
        let _ = apply_vibrancy(
            window,
            NSVisualEffectMaterial::UnderWindowBackground,
            None,
            None,
        );
    }
}

use db::Database;
use power::PowerMonitor;
use scheduler::Scheduler;

const TRAY_ID: &str = "main-tray";
const LAST_BOOT_MARKER_KEY: &str = "last_boot_marker";

struct ReminderTemplateText {
    drink_name: &'static str,
    drink_message: &'static str,
    rest_name: &'static str,
    rest_message: &'static str,
    rest_action_title: &'static str,
    rest_action_message: &'static str,
    eye_name: &'static str,
    eye_message: &'static str,
    eye_action_title: &'static str,
    eye_action_message: &'static str,
}

struct TrayMenuText {
    pause_all: &'static str,
    resume_all: &'static str,
    dnd_30: &'static str,
    dnd_60: &'static str,
    settings: &'static str,
    show: &'static str,
    quit: &'static str,
}

struct TrayMenuItems<R: Runtime> {
    pause_all: MenuItem<R>,
    resume_all: MenuItem<R>,
    dnd_30: MenuItem<R>,
    dnd_60: MenuItem<R>,
    settings: MenuItem<R>,
    show: MenuItem<R>,
    quit: MenuItem<R>,
}

#[cfg(target_os = "windows")]
fn system_locale_name() -> Option<String> {
    let mut buffer = [0u16; 85];
    let length = unsafe { GetUserDefaultLocaleName(&mut buffer) };
    if length <= 0 {
        return None;
    }

    let end = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16(&buffer[..end]).ok()
}

#[cfg(not(target_os = "windows"))]
fn system_locale_name() -> Option<String> {
    None
}

fn match_template_language(value: &str) -> &'static str {
    let normalized = value.replace('_', "-").to_ascii_lowercase();

    if normalized.starts_with("zh-hant")
        || normalized == "zh-tw"
        || normalized == "zh-hk"
        || normalized == "zh-mo"
    {
        return "zh-TW";
    }
    if normalized.starts_with("zh") {
        return "zh-CN";
    }
    if normalized.starts_with("ja") {
        return "ja-JP";
    }
    if normalized.starts_with("ko") {
        return "ko-KR";
    }
    if normalized.starts_with("fr") {
        return "fr-FR";
    }
    if normalized.starts_with("de") {
        return "de-DE";
    }
    if normalized.starts_with("vi") {
        return "vi-VN";
    }
    if normalized.starts_with("th") {
        return "th-TH";
    }
    if normalized.starts_with("ms") {
        return "ms-MY";
    }
    if normalized.starts_with("km") {
        return "km-KH";
    }

    "en-US"
}

fn current_ui_language(db: &Database) -> String {
    let conn = db.conn.lock().unwrap();
    conn.query_row(
        "SELECT value FROM settings WHERE key = 'language'",
        [],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_else(|_| "en-US".to_string())
}

fn tray_menu_text(language: &str) -> TrayMenuText {
    match match_template_language(language) {
        "zh-CN" => TrayMenuText {
            pause_all: "全部暂停",
            resume_all: "全部恢复",
            dnd_30: "免打扰 30 分钟",
            dnd_60: "免打扰 60 分钟",
            settings: "设置",
            show: "显示窗口",
            quit: "退出",
        },
        "zh-TW" => TrayMenuText {
            pause_all: "全部暫停",
            resume_all: "全部恢復",
            dnd_30: "勿擾 30 分鐘",
            dnd_60: "勿擾 60 分鐘",
            settings: "設定",
            show: "顯示視窗",
            quit: "結束",
        },
        "ja-JP" => TrayMenuText {
            pause_all: "すべて一時停止",
            resume_all: "すべて再開",
            dnd_30: "30分通知しない",
            dnd_60: "60分通知しない",
            settings: "設定",
            show: "ウィンドウを表示",
            quit: "終了",
        },
        "ko-KR" => TrayMenuText {
            pause_all: "모두 일시 중지",
            resume_all: "모두 다시 시작",
            dnd_30: "30분 방해 금지",
            dnd_60: "60분 방해 금지",
            settings: "설정",
            show: "창 표시",
            quit: "종료",
        },
        "fr-FR" => TrayMenuText {
            pause_all: "Tout suspendre",
            resume_all: "Tout reprendre",
            dnd_30: "Ne pas déranger 30 min",
            dnd_60: "Ne pas déranger 60 min",
            settings: "Paramètres",
            show: "Afficher la fenêtre",
            quit: "Quitter",
        },
        "de-DE" => TrayMenuText {
            pause_all: "Alle pausieren",
            resume_all: "Alle fortsetzen",
            dnd_30: "30 Min. nicht stören",
            dnd_60: "60 Min. nicht stören",
            settings: "Einstellungen",
            show: "Fenster anzeigen",
            quit: "Beenden",
        },
        "vi-VN" => TrayMenuText {
            pause_all: "Tạm dừng tất cả",
            resume_all: "Tiếp tục tất cả",
            dnd_30: "Không làm phiền 30 phút",
            dnd_60: "Không làm phiền 60 phút",
            settings: "Cài đặt",
            show: "Hiện cửa sổ",
            quit: "Thoát",
        },
        "th-TH" => TrayMenuText {
            pause_all: "หยุดทั้งหมดชั่วคราว",
            resume_all: "ทำงานต่อทั้งหมด",
            dnd_30: "ห้ามรบกวน 30 นาที",
            dnd_60: "ห้ามรบกวน 60 นาที",
            settings: "ตั้งค่า",
            show: "แสดงหน้าต่าง",
            quit: "ออก",
        },
        "ms-MY" => TrayMenuText {
            pause_all: "Jeda semua",
            resume_all: "Sambung semua",
            dnd_30: "Jangan Ganggu 30 minit",
            dnd_60: "Jangan Ganggu 60 minit",
            settings: "Tetapan",
            show: "Tunjukkan tetingkap",
            quit: "Keluar",
        },
        "km-KH" => TrayMenuText {
            pause_all: "ផ្អាកទាំងអស់",
            resume_all: "បន្តទាំងអស់",
            dnd_30: "កុំរំខាន 30 នាទី",
            dnd_60: "កុំរំខាន 60 នាទី",
            settings: "ការកំណត់",
            show: "បង្ហាញបង្អួច",
            quit: "ចាកចេញ",
        },
        _ => TrayMenuText {
            pause_all: "Pause all",
            resume_all: "Resume all",
            dnd_30: "Do not disturb 30 min",
            dnd_60: "Do not disturb 60 min",
            settings: "Settings",
            show: "Show window",
            quit: "Quit",
        },
    }
}

fn installer_language_marker(app: &AppHandle) -> Option<String> {
    let marker_path = app
        .path()
        .app_data_dir()
        .ok()?
        .join("installer-language.txt");

    std::fs::read_to_string(marker_path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn default_template_language(app: &AppHandle) -> (&'static str, bool) {
    if let Some(language) = installer_language_marker(app) {
        return (match_template_language(&language), true);
    }

    let language = system_locale_name()
        .as_deref()
        .map(match_template_language)
        .unwrap_or("en-US");

    (language, false)
}

fn reminder_template_text(language: &str) -> ReminderTemplateText {
    match language {
        "zh-CN" => ReminderTemplateText {
            drink_name: "喝水提醒",
            drink_message: "喝点水，放松一下。",
            rest_name: "休息提醒",
            rest_message: "起来活动两分钟。",
            rest_action_title: "开始休息",
            rest_action_message: "倒计时结束后再进入下一轮。",
            eye_name: "护眼提醒",
            eye_message: "看向远处 20 秒，让眼睛放松一下。",
            eye_action_title: "护眼提醒",
            eye_action_message: "看向远处 20 秒，让眼睛放松一下。",
        },
        "zh-TW" => ReminderTemplateText {
            drink_name: "喝水提醒",
            drink_message: "喝點水，放鬆一下。",
            rest_name: "休息提醒",
            rest_message: "起來活動兩分鐘。",
            rest_action_title: "開始休息",
            rest_action_message: "倒數結束後再進入下一輪。",
            eye_name: "護眼提醒",
            eye_message: "看向遠處 20 秒，讓眼睛放鬆一下。",
            eye_action_title: "護眼提醒",
            eye_action_message: "看向遠處 20 秒，讓眼睛放鬆一下。",
        },
        "ja-JP" => ReminderTemplateText {
            drink_name: "水分補給リマインダー",
            drink_message: "水を飲んで少し休みましょう。",
            rest_name: "休憩リマインダー",
            rest_message: "立って2分ほど動きましょう。",
            rest_action_title: "休憩開始",
            rest_action_message: "カウントダウン後に次のラウンドへ進みます。",
            eye_name: "目の休憩リマインダー",
            eye_message: "20秒間遠くを見て目を休めましょう。",
            eye_action_title: "目の休憩リマインダー",
            eye_action_message: "20秒間遠くを見て目を休めましょう。",
        },
        "ko-KR" => ReminderTemplateText {
            drink_name: "물 마시기 알림",
            drink_message: "물을 마시고 잠시 쉬세요.",
            rest_name: "휴식 알림",
            rest_message: "일어나서 2분 정도 움직이세요.",
            rest_action_title: "휴식 시작",
            rest_action_message: "카운트다운 후 다음 라운드로 이동합니다.",
            eye_name: "눈 휴식 알림",
            eye_message: "20초 동안 멀리 바라보며 눈을 쉬게 해 주세요.",
            eye_action_title: "눈 휴식 알림",
            eye_action_message: "20초 동안 멀리 바라보며 눈을 쉬게 해 주세요.",
        },
        "fr-FR" => ReminderTemplateText {
            drink_name: "Rappel hydratation",
            drink_message: "Buvez un peu d’eau et détendez-vous.",
            rest_name: "Rappel de pause",
            rest_message: "Levez-vous et bougez deux minutes.",
            rest_action_title: "Commencer la pause",
            rest_action_message: "Le prochain cycle commence après le compte à rebours.",
            eye_name: "Rappel pour les yeux",
            eye_message: "Regardez au loin pour reposer vos yeux.",
            eye_action_title: "Rappel pour les yeux",
            eye_action_message: "Regardez au loin pour reposer vos yeux.",
        },
        "de-DE" => ReminderTemplateText {
            drink_name: "Trinkerinnerung",
            drink_message: "Trinken Sie etwas Wasser und entspannen Sie sich.",
            rest_name: "Pausenerinnerung",
            rest_message: "Stehen Sie auf und bewegen Sie sich zwei Minuten.",
            rest_action_title: "Pause starten",
            rest_action_message: "Die nächste Runde beginnt nach dem Countdown.",
            eye_name: "Augenpause",
            eye_message: "Lassen Sie Ihre Augen in die Ferne schauen.",
            eye_action_title: "Augenpause",
            eye_action_message: "Lassen Sie Ihre Augen in die Ferne schauen.",
        },
        "vi-VN" => ReminderTemplateText {
            drink_name: "Nhắc uống nước",
            drink_message: "Uống chút nước và thư giãn.",
            rest_name: "Nhắc nghỉ ngơi",
            rest_message: "Đứng dậy vận động hai phút.",
            rest_action_title: "Bắt đầu nghỉ",
            rest_action_message: "Vòng tiếp theo bắt đầu sau đếm ngược.",
            eye_name: "Nhắc nghỉ mắt",
            eye_message: "Hãy để mắt nhìn xa một chút.",
            eye_action_title: "Nhắc nghỉ mắt",
            eye_action_message: "Hãy để mắt nhìn xa một chút.",
        },
        "th-TH" => ReminderTemplateText {
            drink_name: "เตือนดื่มน้ำ",
            drink_message: "ดื่มน้ำสักหน่อยและผ่อนคลาย",
            rest_name: "เตือนพัก",
            rest_message: "ลุกขึ้นขยับร่างกายสองนาที",
            rest_action_title: "เริ่มพัก",
            rest_action_message: "รอบถัดไปจะเริ่มหลังนับถอยหลัง",
            eye_name: "เตือนพักสายตา",
            eye_message: "พักสายตาด้วยการมองไกล",
            eye_action_title: "เตือนพักสายตา",
            eye_action_message: "พักสายตาด้วยการมองไกล",
        },
        "ms-MY" => ReminderTemplateText {
            drink_name: "Peringatan minum",
            drink_message: "Minum sedikit air dan berehat.",
            rest_name: "Peringatan rehat",
            rest_message: "Bangun dan bergerak selama dua minit.",
            rest_action_title: "Mula Rehat",
            rest_action_message: "Pusingan seterusnya bermula selepas kiraan.",
            eye_name: "Peringatan mata",
            eye_message: "Biarkan mata memandang jauh.",
            eye_action_title: "Peringatan mata",
            eye_action_message: "Biarkan mata memandang jauh.",
        },
        "km-KH" => ReminderTemplateText {
            drink_name: "រំលឹកផឹកទឹក",
            drink_message: "ផឹកទឹកបន្តិច ហើយសម្រាក។",
            rest_name: "រំលឹកសម្រាក",
            rest_message: "ក្រោកឡើង ហើយធ្វើចលនាពីរនាទី។",
            rest_action_title: "ចាប់ផ្តើមសម្រាក",
            rest_action_message: "វគ្គបន្ទាប់ចាប់ផ្តើមបន្ទាប់ពីរាប់ថយក្រោយ។",
            eye_name: "រំលឹកសម្រាកភ្នែក",
            eye_message: "ឱ្យភ្នែកមើលទៅឆ្ងាយបន្តិច។",
            eye_action_title: "រំលឹកសម្រាកភ្នែក",
            eye_action_message: "ឱ្យភ្នែកមើលទៅឆ្ងាយបន្តិច។",
        },
        _ => ReminderTemplateText {
            drink_name: "Drink reminder",
            drink_message: "Have some water and relax.",
            rest_name: "Rest reminder",
            rest_message: "Stand up and move for two minutes.",
            rest_action_title: "Start Rest",
            rest_action_message: "The next round starts after the countdown.",
            eye_name: "Eye care reminder",
            eye_message: "Look 20 feet away for 20 seconds.",
            eye_action_title: "Eye care reminder",
            eye_action_message: "Look 20 feet away for 20 seconds.",
        },
    }
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    let window = app.get_webview_window("main")?;
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    Some(window)
}

pub(crate) enum TrayVisualState {
    Idle,
    Alert,
    Muted,
}

pub(crate) fn set_tray_visual_state<R: Runtime>(app: &AppHandle<R>, state: TrayVisualState) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let icon_bytes = match state {
        TrayVisualState::Idle => include_bytes!("../icons/tray-idle.png").as_slice(),
        TrayVisualState::Alert => include_bytes!("../icons/tray-alert.png").as_slice(),
        TrayVisualState::Muted => include_bytes!("../icons/tray-muted.png").as_slice(),
    };

    if let Ok(icon) = Image::from_bytes(icon_bytes) {
        let _ = tray.set_icon(Some(icon));
    }
}

pub(crate) fn refresh_tray_menu_text<R: Runtime>(app: &AppHandle<R>, language: &str) {
    let Some(items) = app.try_state::<TrayMenuItems<R>>() else {
        return;
    };
    let text = tray_menu_text(language);

    let _ = items.pause_all.set_text(text.pause_all);
    let _ = items.resume_all.set_text(text.resume_all);
    let _ = items.dnd_30.set_text(text.dnd_30);
    let _ = items.dnd_60.set_text(text.dnd_60);
    let _ = items.settings.set_text(text.settings);
    let _ = items.show.set_text(text.show);
    let _ = items.quit.set_text(text.quit);
}

/// 清理旧日志（保留90天）
fn cleanup_old_logs(db: &Database) {
    let conn = db.conn.lock().unwrap();
    let cutoff_date = (chrono::Utc::now() - chrono::Duration::days(90))
        .format("%Y-%m-%d")
        .to_string();

    let deleted = conn
        .execute(
            "DELETE FROM reminder_logs WHERE triggered_at < ?1",
            [&cutoff_date],
        )
        .unwrap_or(0);

    if deleted > 0 {
        println!("Cleaned up {} old log entries", deleted);
    }
}

/// 插入默认提醒数据
fn insert_default_reminders(app: &AppHandle, db: &Database) {
    let conn = db.conn.lock().unwrap();

    // 检查是否已有数据
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM reminders", [], |row| row.get(0))
        .unwrap_or(0);

    if count > 0 {
        return;
    }

    let now = chrono::Utc::now();
    let (language, from_installer) = default_template_language(app);
    let text = reminder_template_text(language);
    let defaults = vec![
        (
            "default-drink",
            text.drink_name,
            "drink",
            "drink",
            text.drink_message,
            90i64,
            0i64,
            0i32,
            0i32,
            "",
            "",
            0i64,
            "auto",
        ),
        (
            "default-rest",
            text.rest_name,
            "rest",
            "rest",
            text.rest_message,
            60i64,
            5i64,
            1i32,
            1i32,
            text.rest_action_title,
            text.rest_action_message,
            300i64,
            "auto",
        ),
        (
            "default-eye-care",
            text.eye_name,
            "eye_care",
            "eye_care",
            text.eye_message,
            20i64,
            0i64,
            0i32,
            1i32,
            text.eye_action_title,
            text.eye_action_message,
            20i64,
            "auto",
        ),
    ];

    for (
        id,
        name,
        rtype,
        icon,
        message,
        interval,
        break_duration,
        break_notification_enabled,
        action_enabled,
        action_title,
        action_message,
        action_duration_seconds,
        action_completion_mode,
    ) in defaults
    {
        let next_trigger = (now + chrono::Duration::minutes(interval))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        let now_str = now.format("%Y-%m-%dT%H:%M:%S").to_string();

        let _ = conn.execute(
            "INSERT INTO reminders (id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 1, ?14, ?15, ?16)",
            (id, name, rtype, icon, message, interval, break_duration, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, next_trigger, now_str.clone(), now_str),
        );
    }

    if from_installer {
        let _ = conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('language', ?1)",
            [language],
        );
    }
}

/// 安全升级仍保持旧默认值的预设提醒，不覆盖用户已调整过的设置
fn repair_default_reminder_templates(db: &Database) {
    let conn = db.conn.lock().unwrap();
    let now = Utc::now();
    let now_str = now.format("%Y-%m-%dT%H:%M:%S").to_string();

    let _ = conn.execute(
        "UPDATE reminders
         SET interval_minutes = 90,
             next_trigger = CASE WHEN enabled = 1 THEN ?1 ELSE next_trigger END,
             updated_at = ?2
         WHERE id = 'default-drink' AND interval_minutes = 45",
        (
            (now + chrono::Duration::minutes(90))
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            &now_str,
        ),
    );

    let _ = conn.execute(
        "UPDATE reminders
         SET break_duration_minutes = CASE WHEN break_duration_minutes IN (0, 10) THEN 5 ELSE break_duration_minutes END,
             break_notification_enabled = CASE WHEN break_duration_minutes IN (0, 10) THEN 1 ELSE break_notification_enabled END,
             action_enabled = 1,
             action_title = CASE WHEN action_title = '' THEN '休息中' ELSE action_title END,
             action_message = CASE WHEN action_message = '' THEN '倒计时结束后再进入下一轮。' ELSE action_message END,
             action_duration_seconds = CASE WHEN action_duration_seconds IN (0, 600) THEN 300 ELSE action_duration_seconds END,
             action_completion_mode = CASE WHEN action_completion_mode = '' THEN 'auto' ELSE action_completion_mode END,
             updated_at = ?1
         WHERE id = 'default-rest'
           AND (
             break_duration_minutes IN (0, 10)
             OR action_duration_seconds IN (0, 600)
           )",
        [&now_str],
    );

    let _ = conn.execute(
        "UPDATE reminders
         SET action_duration_seconds = 20,
             action_enabled = 1,
             action_title = CASE WHEN action_title = '' THEN '护眼中' ELSE action_title END,
             action_message = CASE WHEN action_message = '' THEN '看向远处，让眼睛放松一下。' ELSE action_message END,
             action_completion_mode = CASE WHEN action_completion_mode = '' THEN 'auto' ELSE action_completion_mode END,
             message = CASE WHEN message = '让眼睛看远一点。' THEN '看向远处 20 秒，让眼睛放松一下。' ELSE message END,
             updated_at = ?1
         WHERE id = 'default-eye-care' AND action_duration_seconds = 0",
        [&now_str],
    );
}

/// 修复已过期或缺失的下次提醒时间，避免应用启动时连续触发旧提醒
fn repair_reminder_schedule(db: &Database) {
    let conn = db.conn.lock().unwrap();
    let now = Utc::now();
    let now_str = now.format("%Y-%m-%dT%H:%M:%S").to_string();

    let mut stmt = match conn.prepare(
        "SELECT id, interval_minutes, next_trigger
         FROM reminders
         WHERE enabled = 1",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return,
    };

    let reminders: Vec<(String, i64, Option<String>)> =
        match stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))) {
            Ok(rows) => rows.filter_map(Result::ok).collect(),
            Err(_) => return,
        };

    for (id, interval_minutes, next_trigger) in reminders {
        let should_reset = match next_trigger {
            Some(value) => NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S")
                .map(|timestamp| timestamp.and_utc() <= now)
                .unwrap_or(true),
            None => true,
        };

        if !should_reset {
            continue;
        }

        let new_next = (now + chrono::Duration::minutes(interval_minutes))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

        let _ = conn.execute(
            "UPDATE reminders SET next_trigger = ?1, updated_at = ?2 WHERE id = ?3",
            (&new_next, &now_str, &id),
        );
    }
}

fn reset_enabled_reminder_schedule(db: &Database) {
    let conn = db.conn.lock().unwrap();
    let now = Utc::now();
    let now_str = now.format("%Y-%m-%dT%H:%M:%S").to_string();

    let mut stmt = match conn.prepare(
        "SELECT id, interval_minutes
         FROM reminders
         WHERE enabled = 1",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return,
    };

    let reminders: Vec<(String, i64)> =
        match stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?))) {
            Ok(rows) => rows.filter_map(Result::ok).collect(),
            Err(_) => return,
        };

    for (id, interval_minutes) in reminders {
        let new_next = (now + chrono::Duration::minutes(interval_minutes))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

        let _ = conn.execute(
            "UPDATE reminders SET next_trigger = ?1, updated_at = ?2 WHERE id = ?3",
            (&new_next, &now_str, &id),
        );
    }
}

#[cfg(target_os = "windows")]
fn current_boot_marker() -> Option<String> {
    use windows::Win32::System::SystemInformation::GetTickCount64;

    let uptime_ms = unsafe { GetTickCount64() };
    let uptime_ms = i64::try_from(uptime_ms).ok()?;
    let boot_time = Utc::now() - chrono::Duration::milliseconds(uptime_ms);
    Some(boot_time.format("%Y-%m-%dT%H:%M:%S").to_string())
}

#[cfg(not(target_os = "windows"))]
fn current_boot_marker() -> Option<String> {
    None
}

fn reconcile_schedule_on_startup(db: &Database) {
    let current_marker = current_boot_marker();

    {
        let conn = db.conn.lock().unwrap();
        let previous_marker = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [LAST_BOOT_MARKER_KEY],
                |row| row.get::<_, String>(0),
            )
            .ok();

        let system_rebooted = match (&previous_marker, &current_marker) {
            (Some(previous), Some(current)) => previous != current,
            _ => false,
        };

        drop(conn);

        if system_rebooted {
            reset_enabled_reminder_schedule(db);
        } else {
            repair_reminder_schedule(db);
        }
    }

    if let Some(marker) = current_marker {
        let conn = db.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            (LAST_BOOT_MARKER_KEY, &marker),
        );
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app_log::install_panic_hook();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let is_autostart = args.iter().any(|arg| arg == "--autostart");
            if !is_autostart {
                show_main_window(app);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart".into()]),
        ))
        .setup(|app| {
            app_log::init(app.handle());
            app_log::info("应用启动");

            // 应用窗口材质
            if let Some(main_window) = app.get_webview_window("main") {
                apply_window_material(&main_window);
            }
            if let Some(notification_window) = app.get_webview_window("notification") {
                apply_window_material(&notification_window);
            }

            // 初始化数据库
            let db = Database::init(app.handle()).expect("failed to init database");
            cleanup_old_logs(&db);
            insert_default_reminders(app.handle(), &db);
            repair_default_reminder_templates(&db);
            reconcile_schedule_on_startup(&db);
            let tray_language = current_ui_language(&db);
            app.manage(db);

            // 启动定时器
            let scheduler = Scheduler::new(app.handle().clone());
            app.manage(scheduler);
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<Scheduler>();
                state.start().await;
            });

            // 启动电源监听器
            let power_monitor = PowerMonitor::new(app.handle().clone());
            app.manage(power_monitor);
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<PowerMonitor>();
                state.start().await;
            });

            // 创建托盘菜单
            let tray_text = tray_menu_text(&tray_language);
            let pause_item =
                MenuItem::with_id(app, "pause_all", tray_text.pause_all, true, None::<&str>)?;
            let resume_item =
                MenuItem::with_id(app, "resume_all", tray_text.resume_all, true, None::<&str>)?;
            let dnd_30 = MenuItem::with_id(app, "dnd_30", tray_text.dnd_30, true, None::<&str>)?;
            let dnd_60 = MenuItem::with_id(app, "dnd_60", tray_text.dnd_60, true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let settings_item =
                MenuItem::with_id(app, "settings", tray_text.settings, true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", tray_text.show, true, None::<&str>)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", tray_text.quit, true, None::<&str>)?;

            let menu = Menu::with_items(app, &[
                &pause_item,
                &resume_item,
                &dnd_30,
                &dnd_60,
                &sep1,
                &settings_item,
                &show_item,
                &sep2,
                &quit_item,
            ])?;
            app.manage(TrayMenuItems {
                pause_all: pause_item.clone(),
                resume_all: resume_item.clone(),
                dnd_30: dnd_30.clone(),
                dnd_60: dnd_60.clone(),
                settings: settings_item.clone(),
                show: show_item.clone(),
                quit: quit_item.clone(),
            });

            // 创建系统托盘
            let _tray = TrayIconBuilder::with_id(TRAY_ID)
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" | "settings" => {
                        if let Some(window) = show_main_window(app) {
                            if event.id.as_ref() == "settings" {
                                let _ = window.emit("navigate", "settings");
                            }
                        }
                    }
                    "pause_all" => {
                        let db = app.state::<Database>();
                        let conn = db.conn.lock().unwrap();
                        let _ = commands::pause_all_reminders(&conn);
                        drop(conn);
                        let scheduler = app.state::<Scheduler>();
                        scheduler.clear_all_active();
                        if let Some(notification_window) = app.get_webview_window("notification") {
                            let _ = notification_window.hide();
                        }
                        let _ = app.emit("reminders:changed", ());
                        set_tray_visual_state(app, TrayVisualState::Muted);
                    }
                    "resume_all" => {
                        let db = app.state::<Database>();
                        let conn = db.conn.lock().unwrap();
                        let _ = commands::resume_all_reminders(&conn);
                        drop(conn);
                        let _ = app.emit("reminders:changed", ());
                        set_tray_visual_state(app, TrayVisualState::Idle);
                    }
                    "dnd_30" | "dnd_60" => {
                        let minutes: i64 = if event.id.as_ref() == "dnd_30" { 30 } else { 60 };
                        let db = app.state::<Database>();
                        let conn = db.conn.lock().unwrap();
                        let until = (Utc::now() + chrono::Duration::minutes(minutes))
                            .format("%Y-%m-%dT%H:%M:%S")
                            .to_string();
                        let _ = conn.execute(
                            "INSERT OR REPLACE INTO settings (key, value) VALUES ('temp_dnd_until', ?1)",
                            [&until],
                        );
                        set_tray_visual_state(app, TrayVisualState::Muted);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        show_main_window(app);
                    }
                })
                .build(app)?;

            // 静默启动：仅开机自启且用户开启静默启动时，不显示主窗口
            let is_autostart = std::env::args().any(|a| a == "--autostart");
            let should_show = if is_autostart {
                let db = app.state::<Database>();
                let silent = db
                    .conn.lock().unwrap()
                    .query_row(
                        "SELECT value FROM settings WHERE key = 'silent_start'",
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .map(|v| v == "true")
                    .unwrap_or(false);
                !silent
            } else {
                true
            };

            if should_show {
                show_main_window(app.handle());
            }

            let all_paused = {
                let db = app.state::<Database>();
                let conn = db.conn.lock().unwrap();
                commands::all_reminders_paused(&conn)
            };
            set_tray_visual_state(
                &app.handle().clone(),
                if all_paused {
                    TrayVisualState::Muted
                } else {
                    TrayVisualState::Idle
                },
            );

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_reminders,
            commands::create_reminder,
            commands::update_reminder,
            commands::delete_reminder,
            commands::toggle_reminder,
            commands::respond_reminder,
            commands::postpone_reminder,
            commands::release_notification,
            commands::get_daily_stats,
            commands::get_trend_stats,
            commands::get_streak_stats,
            commands::cleanup_old_logs,
            commands::save_setting,
            commands::get_setting,
            commands::get_all_settings,
            commands::export_data,
            commands::import_data,
            commands::write_file,
            commands::read_file,
            commands::toggle_all_reminders,
            commands::set_temp_dnd,
            commands::get_last_triggered,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
