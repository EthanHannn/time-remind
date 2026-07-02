use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
};

use chrono::{Duration, Local, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::db::Database;
use crate::models::*;
use crate::scheduler::Scheduler;

pub(crate) const ALL_REMINDERS_PAUSED_KEY: &str = "all_reminders_paused";
const ALL_REMINDERS_PAUSED_AT_KEY: &str = "all_reminders_paused_at";
const MIN_INTERVAL_MINUTES: i64 = 1;
const MAX_INTERVAL_MINUTES: i64 = 1440;
const MIN_BREAK_DURATION_MINUTES: i64 = 1;
const MAX_BREAK_DURATION_MINUTES: i64 = 120;
const MIN_ACTION_DURATION_SECONDS: i64 = 1;
const MAX_ACTION_DURATION_SECONDS: i64 = 7200;
const MAX_BACKUP_FILE_BYTES: u64 = 2 * 1024 * 1024;

fn now_utc_string() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn validate_backup_file_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return Err("备份文件必须使用 .json 后缀".to_string());
    };

    if !extension.eq_ignore_ascii_case("json") {
        return Err("备份文件必须使用 .json 后缀".to_string());
    }

    Ok(path)
}

fn validate_backup_content_size(content: &str) -> Result<(), String> {
    if content.len() as u64 > MAX_BACKUP_FILE_BYTES {
        return Err("备份文件大小不能超过 2MB".to_string());
    }

    Ok(())
}

fn validate_existing_backup_file(path: &Path) -> Result<(), String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("备份路径不是有效文件".to_string());
    }

    if metadata.len() > MAX_BACKUP_FILE_BYTES {
        return Err("备份文件大小不能超过 2MB".to_string());
    }

    Ok(())
}

fn build_next_trigger(minutes: i64) -> String {
    (Utc::now() + Duration::minutes(minutes))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

fn build_next_trigger_from(base: chrono::DateTime<Utc>, minutes: i64) -> String {
    (base + Duration::minutes(minutes))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

pub(crate) fn all_reminders_paused(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [ALL_REMINDERS_PAUSED_KEY],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .and_then(|value| value.parse().ok())
    .unwrap_or(false)
}

pub(crate) fn pause_all_reminders(conn: &Connection) -> Result<(), String> {
    let now = now_utc_string();
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, 'true')",
        [ALL_REMINDERS_PAUSED_KEY],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        (ALL_REMINDERS_PAUSED_AT_KEY, &now),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn shift_enabled_reminder_schedule(
    conn: &Connection,
    paused_seconds: i64,
    now: chrono::DateTime<Utc>,
) -> Result<(), String> {
    let now_str = now.format("%Y-%m-%dT%H:%M:%S").to_string();
    let mut stmt = conn
        .prepare("SELECT id, interval_minutes, next_trigger FROM reminders WHERE enabled = 1")
        .map_err(|e| e.to_string())?;

    let reminders: Vec<(String, i64, Option<String>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);

    for (id, interval_minutes, next_trigger) in reminders {
        let new_next = next_trigger
            .and_then(|value| NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S").ok())
            .map(|timestamp| timestamp.and_utc() + Duration::seconds(paused_seconds))
            .unwrap_or_else(|| now + Duration::minutes(interval_minutes))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

        conn.execute(
            "UPDATE reminders SET next_trigger = ?1, updated_at = ?2 WHERE id = ?3",
            (&new_next, &now_str, &id),
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn restore_legacy_all_paused_reminders(
    conn: &Connection,
    now: chrono::DateTime<Utc>,
) -> Result<(), String> {
    let enabled_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM reminders WHERE enabled = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if enabled_count > 0 {
        return Ok(());
    }

    let now_str = now.format("%Y-%m-%dT%H:%M:%S").to_string();
    let mut stmt = conn
        .prepare("SELECT id, interval_minutes FROM reminders WHERE enabled = 0")
        .map_err(|e| e.to_string())?;
    let reminders: Vec<(String, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);

    for (id, interval_minutes) in reminders {
        let next = build_next_trigger_from(now, interval_minutes);
        conn.execute(
            "UPDATE reminders SET enabled = 1, next_trigger = ?1, updated_at = ?2 WHERE id = ?3",
            (&next, &now_str, &id),
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub(crate) fn resume_all_reminders(conn: &Connection) -> Result<(), String> {
    let now = Utc::now();
    let paused_at = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [ALL_REMINDERS_PAUSED_AT_KEY],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|value| NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S").ok())
        .map(|timestamp| timestamp.and_utc());

    if let Some(paused_at) = paused_at {
        let paused_seconds = (now - paused_at).num_seconds().max(0);
        if paused_seconds > 0 {
            shift_enabled_reminder_schedule(conn, paused_seconds, now)?;
        }
    }

    restore_legacy_all_paused_reminders(conn, now)?;

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, 'false')",
        [ALL_REMINDERS_PAUSED_KEY],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM settings WHERE key = ?1",
        [ALL_REMINDERS_PAUSED_AT_KEY],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn build_next_trigger_from_parts(
    base: chrono::DateTime<Utc>,
    minutes: i64,
    seconds: i64,
) -> String {
    (base + Duration::minutes(minutes) + Duration::seconds(seconds))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

fn validate_interval_minutes(minutes: i64, field_name: &str) -> Result<(), String> {
    if !(MIN_INTERVAL_MINUTES..=MAX_INTERVAL_MINUTES).contains(&minutes) {
        return Err(format!(
            "{}必须在 {} 到 {} 分钟之间",
            field_name, MIN_INTERVAL_MINUTES, MAX_INTERVAL_MINUTES
        ));
    }

    Ok(())
}

fn validate_required_text(value: &str, field_name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field_name}不能为空"));
    }

    Ok(())
}

fn validate_break_duration_minutes(minutes: i64) -> Result<(), String> {
    if !(MIN_BREAK_DURATION_MINUTES..=MAX_BREAK_DURATION_MINUTES).contains(&minutes) {
        return Err(format!(
            "休息时长必须在 {} 到 {} 分钟之间",
            MIN_BREAK_DURATION_MINUTES, MAX_BREAK_DURATION_MINUTES
        ));
    }

    Ok(())
}

fn validate_action_duration_seconds(seconds: i64) -> Result<(), String> {
    if seconds == 0 {
        return Ok(());
    }

    if !(MIN_ACTION_DURATION_SECONDS..=MAX_ACTION_DURATION_SECONDS).contains(&seconds) {
        return Err(format!(
            "动作时长必须在 {} 到 {} 秒之间",
            MIN_ACTION_DURATION_SECONDS, MAX_ACTION_DURATION_SECONDS
        ));
    }

    Ok(())
}

fn normalize_break_settings(
    reminder_type: &str,
    break_duration_minutes: i64,
    break_notification_enabled: bool,
    action_enabled: bool,
    action_title: String,
    action_message: String,
    action_duration_seconds: i64,
) -> Result<(i64, bool, bool, String, String, i64, String), String> {
    let default_title = match reminder_type {
        "rest" => "休息中",
        "eye_care" => "护眼中",
        _ => "行动中",
    };
    let default_message = match reminder_type {
        "rest" => "倒计时结束后再进入下一轮。",
        "eye_care" => "看向远处，让眼睛放松一下。",
        _ => "倒计时结束后再进入下一轮。",
    };

    if reminder_type == "rest" {
        validate_break_duration_minutes(break_duration_minutes)?;
        let normalized_seconds = if action_duration_seconds > 0 {
            action_duration_seconds
        } else if break_notification_enabled || action_enabled {
            break_duration_minutes * 60
        } else {
            0
        };
        let normalized_enabled = action_enabled || break_notification_enabled;
        if normalized_enabled {
            validate_action_duration_seconds(normalized_seconds)?;
        }
        return Ok((
            break_duration_minutes,
            break_notification_enabled,
            normalized_enabled,
            normalize_action_text(action_title, default_title),
            normalize_action_text(action_message, default_message),
            normalized_seconds,
            "auto".to_string(),
        ));
    }

    let normalized_seconds = if reminder_type == "eye_care" {
        if action_duration_seconds > 0 {
            action_duration_seconds
        } else {
            20
        }
    } else {
        action_duration_seconds
    };

    let normalized_enabled =
        action_enabled || (reminder_type == "eye_care" && normalized_seconds > 0);
    if normalized_enabled {
        validate_action_duration_seconds(normalized_seconds)?;
    }
    Ok((
        if reminder_type == "rest" {
            break_duration_minutes
        } else {
            0
        },
        false,
        normalized_enabled,
        normalize_action_text(action_title, default_title),
        normalize_action_text(action_message, default_message),
        if normalized_enabled {
            normalized_seconds
        } else {
            0
        },
        "auto".to_string(),
    ))
}

fn normalize_action_text(value: String, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn calculate_next_trigger(reminder: &Reminder, action: &str, now: chrono::DateTime<Utc>) -> String {
    let action_seconds =
        if action == "completed" && reminder.action_enabled && reminder.action_duration_seconds > 0
        {
            reminder.action_duration_seconds
        } else {
            0
        };

    build_next_trigger_from_parts(now, reminder.interval_minutes, action_seconds)
}

fn validate_response_action(action: &str) -> Result<(), String> {
    match action {
        "completed" | "skipped" | "timeout" => Ok(()),
        _ => Err("无效的提醒响应动作".to_string()),
    }
}

fn get_reminder_by_id(conn: &Connection, id: &str) -> Result<Reminder, String> {
    conn.query_row(
        "SELECT id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at FROM reminders WHERE id = ?1",
        [id],
        |row| {
            Ok(Reminder {
                id: row.get(0)?,
                name: row.get(1)?,
                reminder_type: row.get(2)?,
                icon: row.get(3)?,
                message: row.get(4)?,
                interval_minutes: row.get(5)?,
                break_duration_minutes: row.get(6)?,
                break_notification_enabled: row.get::<_, i32>(7)? != 0,
                action_enabled: row.get::<_, i32>(8)? != 0,
                action_title: row.get(9)?,
                action_message: row.get(10)?,
                action_duration_seconds: row.get(11)?,
                action_completion_mode: row.get(12)?,
                enabled: row.get::<_, i32>(13)? != 0,
                next_trigger: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

fn resolve_local_datetime(date: NaiveDate) -> Result<chrono::DateTime<Local>, String> {
    let naive = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| "无法构造本地日期边界".to_string())?;

    match Local.from_local_datetime(&naive) {
        LocalResult::Single(value) => Ok(value),
        LocalResult::Ambiguous(value, _) => Ok(value),
        LocalResult::None => Err("无法解析本地日期边界".to_string()),
    }
}

fn local_day_bounds_utc(date: NaiveDate) -> Result<(String, String), String> {
    let next_date = date
        .checked_add_signed(Duration::days(1))
        .ok_or_else(|| "无法计算下一天日期".to_string())?;
    let start = resolve_local_datetime(date)?
        .with_timezone(&Utc)
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    let end = resolve_local_datetime(next_date)?
        .with_timezone(&Utc)
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    Ok((start, end))
}

fn parse_utc_timestamp_to_local_date(value: &str) -> Option<NaiveDate> {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|timestamp| timestamp.and_utc().with_timezone(&Local).date_naive())
}

fn collect_export_data(conn: &Connection) -> Result<ExportData, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at FROM reminders ORDER BY created_at")
        .map_err(|e| e.to_string())?;

    let reminders = stmt
        .query_map([], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                name: row.get(1)?,
                reminder_type: row.get(2)?,
                icon: row.get(3)?,
                message: row.get(4)?,
                interval_minutes: row.get(5)?,
                break_duration_minutes: row.get(6)?,
                break_notification_enabled: row.get::<_, i32>(7)? != 0,
                action_enabled: row.get::<_, i32>(8)? != 0,
                action_title: row.get(9)?,
                action_message: row.get(10)?,
                action_duration_seconds: row.get(11)?,
                action_completion_mode: row.get(12)?,
                enabled: row.get::<_, i32>(13)? != 0,
                next_trigger: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut logs_stmt = conn
        .prepare(
            "SELECT id, reminder_id, action, triggered_at, responded_at
             FROM reminder_logs
             ORDER BY triggered_at",
        )
        .map_err(|e| e.to_string())?;

    let logs = logs_stmt
        .query_map([], |row| {
            Ok(ReminderLog {
                id: row.get(0)?,
                reminder_id: row.get(1)?,
                action: row.get(2)?,
                triggered_at: row.get(3)?,
                responded_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut settings_stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;

    let settings = settings_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<HashMap<String, String>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(ExportData {
        version: "1.2".to_string(),
        exported_at: now_utc_string(),
        reminders,
        logs,
        settings,
    })
}

fn validate_import_data(data: &ExportData) -> Result<(), String> {
    if data.version.trim().is_empty() {
        return Err("备份文件缺少版本号".to_string());
    }

    let mut reminder_ids = HashSet::new();
    for reminder in &data.reminders {
        if reminder.id.trim().is_empty() {
            return Err("备份文件中存在缺少 id 的提醒".to_string());
        }
        if reminder.name.trim().is_empty() {
            return Err("备份文件中存在名称为空的提醒".to_string());
        }
        if reminder.message.trim().is_empty() {
            return Err(format!("备份文件中存在内容为空的提醒: {}", reminder.id));
        }
        validate_interval_minutes(reminder.interval_minutes, "提醒间隔")?;
        let _ = normalize_break_settings(
            &reminder.reminder_type,
            reminder.break_duration_minutes,
            reminder.break_notification_enabled,
            reminder.action_enabled,
            reminder.action_title.clone(),
            reminder.action_message.clone(),
            reminder.action_duration_seconds,
        )?;
        if !reminder_ids.insert(reminder.id.as_str()) {
            return Err(format!("备份文件中存在重复提醒 id: {}", reminder.id));
        }
    }

    let mut log_ids = HashSet::new();
    for log in &data.logs {
        if log.id.trim().is_empty() {
            return Err("备份文件中存在缺少 id 的日志".to_string());
        }
        if !log_ids.insert(log.id.as_str()) {
            return Err(format!("备份文件中存在重复日志 id: {}", log.id));
        }
    }

    Ok(())
}

fn repair_imported_reminder_schedule(conn: &Connection) -> Result<(), String> {
    let now = Utc::now();
    let now_str = now.format("%Y-%m-%dT%H:%M:%S").to_string();

    let mut stmt = conn
        .prepare(
            "SELECT id, interval_minutes, next_trigger
             FROM reminders
             WHERE enabled = 1",
        )
        .map_err(|e| e.to_string())?;

    let reminders: Vec<(String, i64, Option<String>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

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

        let new_next = (now + Duration::minutes(interval_minutes))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

        conn.execute(
            "UPDATE reminders SET next_trigger = ?1, updated_at = ?2 WHERE id = ?3",
            (&new_next, &now_str, &id),
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn get_backup_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_dir = app_data_dir.join("backups");
    std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    Ok(backup_dir)
}

fn prune_backup_files(backup_dir: &PathBuf, keep_count: usize) -> Result<(), String> {
    let mut backup_files = std::fs::read_dir(backup_dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();

    backup_files.sort_by_key(|entry| entry.metadata().and_then(|value| value.modified()).ok());

    let remove_count = backup_files.len().saturating_sub(keep_count);
    for entry in backup_files.into_iter().take(remove_count) {
        std::fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn create_import_backup(app: &AppHandle, conn: &Connection) -> Result<String, String> {
    let backup_dir = get_backup_dir(app)?;
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_path = backup_dir.join(format!("before-import-{}.json", timestamp));
    let snapshot = collect_export_data(conn)?;
    let snapshot_json = serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?;

    std::fs::write(&backup_path, snapshot_json).map_err(|e| e.to_string())?;
    prune_backup_files(&backup_dir, 10)?;

    Ok(backup_path.to_string_lossy().into_owned())
}

/// 获取所有提醒
#[tauri::command]
pub fn get_reminders(db: State<'_, Database>) -> Result<Vec<Reminder>, String> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at FROM reminders ORDER BY created_at")
        .map_err(|e| e.to_string())?;

    let reminders = stmt
        .query_map([], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                name: row.get(1)?,
                reminder_type: row.get(2)?,
                icon: row.get(3)?,
                message: row.get(4)?,
                interval_minutes: row.get(5)?,
                break_duration_minutes: row.get(6)?,
                break_notification_enabled: row.get::<_, i32>(7)? != 0,
                action_enabled: row.get::<_, i32>(8)? != 0,
                action_title: row.get(9)?,
                action_message: row.get(10)?,
                action_duration_seconds: row.get(11)?,
                action_completion_mode: row.get(12)?,
                enabled: row.get::<_, i32>(13)? != 0,
                next_trigger: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(reminders)
}

/// 创建提醒
#[tauri::command]
pub fn create_reminder(
    db: State<'_, Database>,
    request: CreateReminderRequest,
) -> Result<Reminder, String> {
    validate_interval_minutes(request.interval_minutes, "提醒间隔")?;
    validate_required_text(&request.name, "提醒名称")?;
    validate_required_text(&request.message, "提醒内容")?;
    let requested_break_duration = request.break_duration_minutes.unwrap_or_else(|| {
        if request.reminder_type == "rest" {
            5
        } else {
            0
        }
    });
    let requested_break_notification = request
        .break_notification_enabled
        .unwrap_or(request.reminder_type == "rest");
    let requested_action_duration_seconds =
        request
            .action_duration_seconds
            .unwrap_or_else(|| match request.reminder_type.as_str() {
                "rest" => requested_break_duration * 60,
                "eye_care" => 20,
                _ => 0,
            });
    let requested_action_enabled = request
        .action_enabled
        .unwrap_or(requested_break_notification || request.reminder_type == "eye_care");
    let requested_action_title = request.action_title.unwrap_or_default();
    let requested_action_message = request.action_message.unwrap_or_default();
    let (
        break_duration_minutes,
        break_notification_enabled,
        action_enabled,
        action_title,
        action_message,
        action_duration_seconds,
        action_completion_mode,
    ) = normalize_break_settings(
        &request.reminder_type,
        requested_break_duration,
        requested_break_notification,
        requested_action_enabled,
        requested_action_title,
        requested_action_message,
        requested_action_duration_seconds,
    )?;

    let conn = db.conn.lock().unwrap();
    let now = now_utc_string();
    let id = Uuid::new_v4().to_string();
    let enabled = request.enabled.unwrap_or(true);
    let next_trigger = if enabled {
        Some(build_next_trigger(request.interval_minutes))
    } else {
        None
    };

    conn.execute(
        "INSERT INTO reminders (id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![&id, &request.name, &request.reminder_type, &request.icon, &request.message, request.interval_minutes, break_duration_minutes, break_notification_enabled as i32, action_enabled as i32, &action_title, &action_message, action_duration_seconds, &action_completion_mode, enabled as i32, &next_trigger, &now, &now],
    ).map_err(|e| e.to_string())?;

    Ok(Reminder {
        id,
        name: request.name,
        reminder_type: request.reminder_type,
        icon: request.icon,
        message: request.message,
        interval_minutes: request.interval_minutes,
        break_duration_minutes,
        break_notification_enabled,
        action_enabled,
        action_title,
        action_message,
        action_duration_seconds,
        action_completion_mode,
        enabled,
        next_trigger,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 更新提醒
#[tauri::command]
pub fn update_reminder(
    app: AppHandle,
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    id: String,
    request: UpdateReminderRequest,
) -> Result<Reminder, String> {
    if let Some(interval_minutes) = request.interval_minutes {
        validate_interval_minutes(interval_minutes, "提醒间隔")?;
    }

    let conn = db.conn.lock().unwrap();
    let now = now_utc_string();
    let current = get_reminder_by_id(&conn, &id)?;

    let name = request.name.unwrap_or(current.name);
    let reminder_type = request.reminder_type.unwrap_or(current.reminder_type);
    let icon = request.icon.unwrap_or(current.icon);
    let message = request.message.unwrap_or(current.message);
    validate_required_text(&name, "提醒名称")?;
    validate_required_text(&message, "提醒内容")?;
    let interval_minutes = request.interval_minutes.unwrap_or(current.interval_minutes);
    let requested_break_duration = request
        .break_duration_minutes
        .unwrap_or(current.break_duration_minutes);
    let requested_break_notification = request
        .break_notification_enabled
        .unwrap_or(current.break_notification_enabled);
    let requested_action_duration_seconds = request
        .action_duration_seconds
        .unwrap_or(current.action_duration_seconds);
    let requested_action_enabled = request.action_enabled.unwrap_or(current.action_enabled);
    let requested_action_title = request.action_title.unwrap_or(current.action_title);
    let requested_action_message = request.action_message.unwrap_or(current.action_message);
    let (
        break_duration_minutes,
        break_notification_enabled,
        action_enabled,
        action_title,
        action_message,
        action_duration_seconds,
        action_completion_mode,
    ) = normalize_break_settings(
        &reminder_type,
        requested_break_duration,
        requested_break_notification,
        requested_action_enabled,
        requested_action_title,
        requested_action_message,
        requested_action_duration_seconds,
    )?;
    let enabled = request.enabled.unwrap_or(current.enabled);

    // 如果间隔改变或重新启用，重算 next_trigger
    let next_trigger = if enabled {
        if request.interval_minutes.is_some() || (request.enabled == Some(true) && !current.enabled)
        {
            Some(build_next_trigger(interval_minutes))
        } else {
            current.next_trigger
        }
    } else {
        None
    };

    conn.execute(
        "UPDATE reminders SET name = ?1, reminder_type = ?2, icon = ?3, message = ?4, interval_minutes = ?5, break_duration_minutes = ?6, break_notification_enabled = ?7, action_enabled = ?8, action_title = ?9, action_message = ?10, action_duration_seconds = ?11, action_completion_mode = ?12, enabled = ?13, next_trigger = ?14, updated_at = ?15 WHERE id = ?16",
        (&name, &reminder_type, &icon, &message, interval_minutes, break_duration_minutes, break_notification_enabled as i32, action_enabled as i32, &action_title, &action_message, action_duration_seconds, &action_completion_mode, enabled as i32, &next_trigger, &now, &id),
    ).map_err(|e| e.to_string())?;
    drop(conn);

    if !enabled {
        let removed_current = scheduler.release_notification(&id);
        if removed_current {
            hide_notification_window(&app);
        }
    }

    Ok(Reminder {
        id,
        name,
        reminder_type,
        icon,
        message,
        interval_minutes,
        break_duration_minutes,
        break_notification_enabled,
        action_enabled,
        action_title,
        action_message,
        action_duration_seconds,
        action_completion_mode,
        enabled,
        next_trigger,
        created_at: current.created_at,
        updated_at: now,
    })
}

/// 删除提醒
#[tauri::command]
pub fn delete_reminder(
    app: AppHandle,
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    id: String,
) -> Result<(), String> {
    let conn = db.conn.lock().unwrap();
    conn.execute("DELETE FROM reminder_logs WHERE reminder_id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM reminders WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    drop(conn);

    let removed_current = scheduler.release_notification(&id);
    if removed_current {
        hide_notification_window(&app);
    }

    Ok(())
}

/// 切换提醒启用/禁用
#[tauri::command]
pub fn toggle_reminder(
    app: AppHandle,
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    id: String,
) -> Result<Reminder, String> {
    let conn = db.conn.lock().unwrap();
    let now = now_utc_string();
    let current = get_reminder_by_id(&conn, &id)?;

    let enabled = !current.enabled;
    let next_trigger = if enabled {
        Some(build_next_trigger(current.interval_minutes))
    } else {
        None
    };

    conn.execute(
        "UPDATE reminders SET enabled = ?1, next_trigger = ?2, updated_at = ?3 WHERE id = ?4",
        (enabled as i32, &next_trigger, &now, &id),
    )
    .map_err(|e| e.to_string())?;
    drop(conn);

    if !enabled {
        let removed_current = scheduler.release_notification(&id);
        if removed_current {
            hide_notification_window(&app);
        }
    }

    Ok(Reminder {
        id: current.id,
        name: current.name,
        reminder_type: current.reminder_type,
        icon: current.icon,
        message: current.message,
        interval_minutes: current.interval_minutes,
        break_duration_minutes: current.break_duration_minutes,
        break_notification_enabled: current.break_notification_enabled,
        action_enabled: current.action_enabled,
        action_title: current.action_title,
        action_message: current.action_message,
        action_duration_seconds: current.action_duration_seconds,
        action_completion_mode: current.action_completion_mode,
        enabled,
        next_trigger,
        created_at: current.created_at,
        updated_at: now,
    })
}

/// 记录提醒响应
#[tauri::command]
pub fn respond_reminder(
    app: AppHandle,
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    reminder_id: String,
    action: String,
    hold_notification: Option<bool>,
) -> Result<(), String> {
    validate_response_action(&action)?;

    let conn = db.conn.lock().unwrap();
    let current = get_reminder_by_id(&conn, &reminder_id)?;
    let now = Utc::now();
    let now_str = now.format("%Y-%m-%dT%H:%M:%S").to_string();
    let id = Uuid::new_v4().to_string();
    let next_trigger = if current.enabled {
        Some(calculate_next_trigger(&current, &action, now))
    } else {
        None
    };

    conn.execute(
        "INSERT INTO reminder_logs (id, reminder_id, action, triggered_at, responded_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        (&id, &reminder_id, &action, &now_str, &now_str),
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE reminders SET next_trigger = ?1, updated_at = ?2 WHERE id = ?3",
        (&next_trigger, &now_str, &reminder_id),
    )
    .map_err(|e| e.to_string())?;

    app.emit("reminders:changed", ())
        .map_err(|e| e.to_string())?;

    if !hold_notification.unwrap_or(false) {
        scheduler.release_notification(&reminder_id);
    }

    Ok(())
}

/// 获取今日统计
#[tauri::command]
pub fn get_daily_stats(db: State<'_, Database>) -> Result<Vec<ReminderStat>, String> {
    let conn = db.conn.lock().unwrap();
    let today = Local::now().date_naive();
    let (start_utc, end_utc) = local_day_bounds_utc(today)?;

    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.name, r.icon, r.reminder_type,
                    COUNT(CASE WHEN l.action = 'completed' THEN 1 END) as completed_count,
                    COUNT(CASE WHEN l.action IS NOT NULL THEN 1 END) as total_count
              FROM reminders r
             LEFT JOIN reminder_logs l ON r.id = l.reminder_id AND l.triggered_at >= ?1 AND l.triggered_at < ?2
             WHERE r.enabled = 1
              GROUP BY r.id",
        )
        .map_err(|e| e.to_string())?;

    let stats = stmt
        .query_map([&start_utc, &end_utc], |row| {
            Ok(ReminderStat {
                reminder_id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                reminder_type: row.get(3)?,
                completed_count: row.get(4)?,
                total_count: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(stats)
}

/// 统计数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderStat {
    pub reminder_id: String,
    pub name: String,
    pub icon: String,
    pub reminder_type: String,
    pub completed_count: i64,
    pub total_count: i64,
}

/// 延后提醒
#[tauri::command]
pub fn postpone_reminder(
    app: AppHandle,
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    reminder_id: String,
    minutes: i64,
) -> Result<(), String> {
    validate_interval_minutes(minutes, "延后时间")?;

    let conn = db.conn.lock().unwrap();
    let current = get_reminder_by_id(&conn, &reminder_id)?;
    if !current.enabled {
        return Err("提醒已禁用，无法延后".to_string());
    }

    let now = now_utc_string();
    let new_next = build_next_trigger(minutes);

    // 记录延后操作
    let log_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO reminder_logs (id, reminder_id, action, triggered_at, responded_at) VALUES (?1, ?2, 'postponed', ?3, ?3)",
        (&log_id, &reminder_id, &now),
    )
    .map_err(|e| e.to_string())?;

    // 更新下次触发时间
    conn.execute(
        "UPDATE reminders SET next_trigger = ?1, updated_at = ?2 WHERE id = ?3",
        (&new_next, &now, &reminder_id),
    )
    .map_err(|e| e.to_string())?;

    app.emit("reminders:changed", ())
        .map_err(|e| e.to_string())?;
    scheduler.release_notification(&reminder_id);

    Ok(())
}

/// 释放当前通知并展示下一条
#[tauri::command]
pub fn release_notification(
    app: AppHandle,
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    reminder_id: String,
    finish_break_now: Option<bool>,
) -> Result<(), String> {
    if finish_break_now.unwrap_or(false) {
        let conn = db.conn.lock().unwrap();
        let current = get_reminder_by_id(&conn, &reminder_id)?;

        if current.enabled && current.action_enabled && current.action_duration_seconds > 0 {
            let now = Utc::now();
            let now_str = now.format("%Y-%m-%dT%H:%M:%S").to_string();
            let next_trigger = build_next_trigger_from(now, current.interval_minutes);

            conn.execute(
                "UPDATE reminders SET next_trigger = ?1, updated_at = ?2 WHERE id = ?3",
                (&next_trigger, &now_str, &reminder_id),
            )
            .map_err(|e| e.to_string())?;

            app.emit("reminders:changed", ())
                .map_err(|e| e.to_string())?;
        }
    }

    scheduler.release_notification(&reminder_id);
    Ok(())
}

fn hide_notification_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("notification") {
        let _ = window.hide();
    }
}

/// 保存设置
#[tauri::command]
pub fn save_setting(
    app: AppHandle,
    db: State<'_, Database>,
    key: String,
    value: String,
) -> Result<(), String> {
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        (&key, &value),
    )
    .map_err(|e| e.to_string())?;
    drop(conn);

    if key == "language" {
        crate::refresh_tray_menu_text(&app, &value);
    }

    Ok(())
}

/// 获取设置
#[tauri::command]
pub fn get_setting(db: State<'_, Database>, key: String) -> Result<Option<String>, String> {
    let conn = db.conn.lock().unwrap();
    let result = conn.query_row("SELECT value FROM settings WHERE key = ?1", [&key], |row| {
        row.get::<_, String>(0)
    });

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// 获取所有设置
#[tauri::command]
pub fn get_all_settings(
    db: State<'_, Database>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;

    let settings = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<std::collections::HashMap<String, String>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(settings)
}

/// 趋势统计数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendStat {
    pub date: String,
    pub completed_count: i64,
    pub total_count: i64,
}

/// 连续打卡天数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreakInfo {
    pub reminder_id: String,
    pub name: String,
    pub icon: String,
    pub reminder_type: String,
    pub current_streak: i64,
    pub max_streak: i64,
}

/// 获取趋势统计（过去 N 天）
#[tauri::command]
pub fn get_trend_stats(
    db: State<'_, Database>,
    days: Option<i64>,
) -> Result<Vec<TrendStat>, String> {
    let conn = db.conn.lock().unwrap();
    let days = days.unwrap_or(7).clamp(1, 365);
    let end_date = Local::now().date_naive();
    let start_date = end_date
        .checked_sub_signed(Duration::days(days - 1))
        .ok_or_else(|| "无法计算趋势开始日期".to_string())?;
    let (start_utc, _) = local_day_bounds_utc(start_date)?;
    let (_, end_utc) = local_day_bounds_utc(end_date)?;

    let mut stmt = conn
        .prepare(
            "SELECT triggered_at, action
              FROM reminder_logs
              WHERE triggered_at >= ?1 AND triggered_at < ?2
              ORDER BY triggered_at",
        )
        .map_err(|e| e.to_string())?;

    let logs = stmt
        .query_map([&start_utc, &end_utc], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut stats_by_date: HashMap<NaiveDate, (i64, i64)> = HashMap::new();
    for (triggered_at, action) in logs {
        if let Some(date) = parse_utc_timestamp_to_local_date(&triggered_at) {
            let entry = stats_by_date.entry(date).or_insert((0, 0));
            if action == "completed" {
                entry.0 += 1;
            }
            entry.1 += 1;
        }
    }

    let mut stats = Vec::new();
    let mut current = start_date;
    while current <= end_date {
        let (completed_count, total_count) = stats_by_date.get(&current).copied().unwrap_or((0, 0));
        stats.push(TrendStat {
            date: current.format("%Y-%m-%d").to_string(),
            completed_count,
            total_count,
        });
        current = current
            .checked_add_signed(Duration::days(1))
            .ok_or_else(|| "无法推进趋势日期".to_string())?;
    }

    Ok(stats)
}

/// 获取连续打卡天数
#[tauri::command]
pub fn get_streak_stats(db: State<'_, Database>) -> Result<Vec<StreakInfo>, String> {
    let conn = db.conn.lock().unwrap();
    let today = Local::now().date_naive();

    // 获取所有启用的提醒
    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.name, r.icon, r.reminder_type
             FROM reminders r
             WHERE r.enabled = 1",
        )
        .map_err(|e| e.to_string())?;

    let reminders: Vec<(String, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    for (id, name, icon, reminder_type) in reminders {
        // 查询每个提醒的完成记录
        let mut log_stmt = conn
            .prepare(
                "SELECT triggered_at, action
                 FROM reminder_logs
                  WHERE reminder_id = ?1
                  ORDER BY triggered_at",
            )
            .map_err(|e| e.to_string())?;

        let logs: Vec<(String, String)> = log_stmt
            .query_map([&id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut completed_dates = BTreeSet::new();
        for (triggered_at, action) in logs {
            if action != "completed" {
                continue;
            }

            if let Some(date) = parse_utc_timestamp_to_local_date(&triggered_at) {
                completed_dates.insert(date);
            }
        }

        let mut current_streak = 0i64;
        let mut current_date = today;
        while completed_dates.contains(&current_date) {
            current_streak += 1;
            let Some(previous_date) = current_date.checked_sub_signed(Duration::days(1)) else {
                break;
            };
            current_date = previous_date;
        }

        let mut max_streak = 0i64;
        let mut temp_streak = 0i64;
        let mut previous_date: Option<NaiveDate> = None;
        for date in &completed_dates {
            if let Some(last_date) = previous_date {
                let next_expected = last_date
                    .checked_add_signed(Duration::days(1))
                    .ok_or_else(|| "无法计算连续打卡日期".to_string())?;
                if *date == next_expected {
                    temp_streak += 1;
                } else {
                    max_streak = max_streak.max(temp_streak);
                    temp_streak = 1;
                }
            } else {
                temp_streak = 1;
            }
            previous_date = Some(*date);
        }
        max_streak = max_streak.max(temp_streak);

        results.push(StreakInfo {
            reminder_id: id,
            name,
            icon,
            reminder_type,
            current_streak,
            max_streak,
        });
    }

    Ok(results)
}

/// 清理旧日志（保留指定天数）
#[tauri::command]
pub fn cleanup_old_logs(db: State<'_, Database>, keep_days: Option<i64>) -> Result<i64, String> {
    let conn = db.conn.lock().unwrap();
    let keep_days = keep_days.unwrap_or(90);
    let cutoff_date = (Utc::now() - chrono::Duration::days(keep_days))
        .format("%Y-%m-%d")
        .to_string();

    let deleted = conn
        .execute(
            "DELETE FROM reminder_logs WHERE triggered_at < ?1",
            [&cutoff_date],
        )
        .map_err(|e| e.to_string())?;

    Ok(deleted as i64)
}

/// 导出数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub version: String,
    pub exported_at: String,
    pub reminders: Vec<Reminder>,
    #[serde(default)]
    pub logs: Vec<ReminderLog>,
    pub settings: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    #[default]
    Replace,
    Merge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub mode: ImportMode,
    pub backup_path: String,
    pub reminders_imported: usize,
    pub logs_imported: usize,
    pub settings_imported: usize,
    pub skipped_logs: usize,
    pub message: String,
}

/// 导出所有数据
#[tauri::command]
pub fn export_data(db: State<'_, Database>) -> Result<ExportData, String> {
    let conn = db.conn.lock().unwrap();
    collect_export_data(&conn)
}

/// 导入数据
#[tauri::command]
pub fn import_data(
    app: AppHandle,
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    data: ExportData,
    mode: Option<ImportMode>,
) -> Result<ImportResult, String> {
    validate_import_data(&data)?;

    let mode = mode.unwrap_or_default();
    let mut conn = db.conn.lock().unwrap();
    let backup_path = create_import_backup(&app, &conn)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    if matches!(mode, ImportMode::Replace) {
        tx.execute("DELETE FROM reminder_logs", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM reminders", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM settings", [])
            .map_err(|e| e.to_string())?;
    }

    let reminder_insert_sql = if matches!(mode, ImportMode::Merge) {
        "INSERT OR REPLACE INTO reminders (id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)"
    } else {
        "INSERT INTO reminders (id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)"
    };

    for reminder in &data.reminders {
        let (
            break_duration_minutes,
            break_notification_enabled,
            action_enabled,
            action_title,
            action_message,
            action_duration_seconds,
            action_completion_mode,
        ) = normalize_break_settings(
            &reminder.reminder_type,
            reminder.break_duration_minutes,
            reminder.break_notification_enabled,
            reminder.action_enabled,
            reminder.action_title.clone(),
            reminder.action_message.clone(),
            reminder.action_duration_seconds,
        )?;
        tx.execute(
            reminder_insert_sql,
            params![
                &reminder.id,
                &reminder.name,
                &reminder.reminder_type,
                &reminder.icon,
                &reminder.message,
                reminder.interval_minutes,
                break_duration_minutes,
                break_notification_enabled as i32,
                action_enabled as i32,
                &action_title,
                &action_message,
                action_duration_seconds,
                &action_completion_mode,
                reminder.enabled as i32,
                &reminder.next_trigger,
                &reminder.created_at,
                &reminder.updated_at,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    let mut reminder_ids_stmt = tx
        .prepare("SELECT id FROM reminders")
        .map_err(|e| e.to_string())?;
    let reminder_ids = reminder_ids_stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<HashSet<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(reminder_ids_stmt);

    let mut logs_imported = 0usize;
    let mut skipped_logs = 0usize;
    let log_insert_sql = if matches!(mode, ImportMode::Merge) {
        "INSERT OR IGNORE INTO reminder_logs (id, reminder_id, action, triggered_at, responded_at) VALUES (?1, ?2, ?3, ?4, ?5)"
    } else {
        "INSERT INTO reminder_logs (id, reminder_id, action, triggered_at, responded_at) VALUES (?1, ?2, ?3, ?4, ?5)"
    };

    for log in &data.logs {
        if !reminder_ids.contains(&log.reminder_id) {
            skipped_logs += 1;
            continue;
        }

        let affected = tx
            .execute(
                log_insert_sql,
                (
                    &log.id,
                    &log.reminder_id,
                    &log.action,
                    &log.triggered_at,
                    &log.responded_at,
                ),
            )
            .map_err(|e| e.to_string())?;

        if affected > 0 {
            logs_imported += 1;
        } else {
            skipped_logs += 1;
        }
    }

    let settings_insert_sql = if matches!(mode, ImportMode::Merge) {
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)"
    } else {
        "INSERT INTO settings (key, value) VALUES (?1, ?2)"
    };

    for (key, value) in &data.settings {
        tx.execute(settings_insert_sql, (key, value))
            .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    repair_imported_reminder_schedule(&conn)?;

    scheduler.clear_all_active();
    if all_reminders_paused(&conn) {
        crate::set_tray_visual_state(&app, crate::TrayVisualState::Muted);
    }
    app.emit("reminders:changed", ())
        .map_err(|e| e.to_string())?;

    let mode_label = match mode {
        ImportMode::Replace => "覆盖导入",
        ImportMode::Merge => "合并导入",
    };

    Ok(ImportResult {
        mode,
        backup_path: backup_path.clone(),
        reminders_imported: data.reminders.len(),
        logs_imported,
        settings_imported: data.settings.len(),
        skipped_logs,
        message: format!(
            "{}完成：{} 条提醒，{} 条日志，{} 项设置。已自动备份到 {}",
            mode_label,
            data.reminders.len(),
            logs_imported,
            data.settings.len(),
            backup_path
        ),
    })
}

/// 写入文件
#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    let path = validate_backup_file_path(&path)?;
    validate_backup_content_size(&content)?;
    std::fs::write(&path, &content).map_err(|e| e.to_string())
}

/// 读取文件
#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    let path = validate_backup_file_path(&path)?;
    validate_existing_backup_file(&path)?;
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    validate_backup_content_size(&content)?;
    Ok(content)
}

/// 批量切换所有提醒的启用状态
#[tauri::command]
pub fn toggle_all_reminders(
    app: AppHandle,
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    enabled: bool,
) -> Result<(), String> {
    let conn = db.conn.lock().unwrap();

    if enabled {
        resume_all_reminders(&conn)?;
        crate::set_tray_visual_state(&app, crate::TrayVisualState::Idle);
    } else {
        pause_all_reminders(&conn)?;
        scheduler.clear_all_active();
        if let Some(window) = app.get_webview_window("notification") {
            let _ = window.hide();
        }
        crate::set_tray_visual_state(&app, crate::TrayVisualState::Muted);
    }
    drop(conn);

    app.emit("reminders:changed", ())
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 设置临时免打扰
#[tauri::command]
pub fn set_temp_dnd(db: State<'_, Database>, minutes: i64) -> Result<(), String> {
    if !(1..=480).contains(&minutes) {
        return Err("免打扰时间必须在 1 到 480 分钟之间".to_string());
    }

    let conn = db.conn.lock().unwrap();
    let until = (Utc::now() + Duration::minutes(minutes))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('temp_dnd_until', ?1)",
        [&until],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// 获取最近一次触发的提醒信息
#[tauri::command]
pub fn get_last_triggered(
    db: State<'_, Database>,
) -> Result<Option<HashMap<String, String>>, String> {
    let conn = db.conn.lock().unwrap();
    let result = conn.query_row(
        "SELECT r.name, r.icon, l.triggered_at
         FROM reminder_logs l
         JOIN reminders r ON r.id = l.reminder_id
         ORDER BY l.triggered_at DESC
         LIMIT 1",
        [],
        |row| {
            let mut map = HashMap::new();
            map.insert("name".to_string(), row.get::<_, String>(0)?);
            map.insert("icon".to_string(), row.get::<_, String>(1)?);
            map.insert("triggered_at".to_string(), row.get::<_, String>(2)?);
            Ok(map)
        },
    );

    match result {
        Ok(map) => Ok(Some(map)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_json_backup_file_path() {
        let path = validate_backup_file_path("backup.JSON").expect("json path should be accepted");

        assert_eq!(path, PathBuf::from("backup.JSON"));
    }

    #[test]
    fn rejects_non_json_backup_file_path() {
        let error =
            validate_backup_file_path("backup.txt").expect_err("non-json path should be rejected");

        assert!(error.contains(".json"));
    }

    #[test]
    fn rejects_oversized_backup_content() {
        let content = "x".repeat(MAX_BACKUP_FILE_BYTES as usize + 1);
        let error =
            validate_backup_content_size(&content).expect_err("oversized content should fail");

        assert!(error.contains("2MB"));
    }

    #[test]
    fn rejects_import_with_empty_reminder_message() {
        let reminder = Reminder {
            id: "broken".to_string(),
            name: "Broken reminder".to_string(),
            reminder_type: "custom".to_string(),
            icon: "custom".to_string(),
            message: " ".to_string(),
            interval_minutes: 30,
            break_duration_minutes: 0,
            break_notification_enabled: false,
            action_enabled: false,
            action_title: String::new(),
            action_message: String::new(),
            action_duration_seconds: 0,
            action_completion_mode: "auto".to_string(),
            enabled: true,
            next_trigger: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let data = ExportData {
            version: "1.2".to_string(),
            exported_at: String::new(),
            reminders: vec![reminder],
            logs: Vec::new(),
            settings: HashMap::new(),
        };

        let error = validate_import_data(&data).expect_err("empty message should fail");

        assert!(error.contains("内容为空"));
    }

    #[test]
    fn completed_action_countdown_keeps_second_precision() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 11, 10, 0, 0)
            .single()
            .expect("fixed test time should be valid");
        let reminder = Reminder {
            id: "default-eye-care".to_string(),
            name: "Eye care".to_string(),
            reminder_type: "eye_care".to_string(),
            icon: "eye_care".to_string(),
            message: "Look away".to_string(),
            interval_minutes: 20,
            break_duration_minutes: 0,
            break_notification_enabled: false,
            action_enabled: true,
            action_title: "Eye care".to_string(),
            action_message: "Look away".to_string(),
            action_duration_seconds: 20,
            action_completion_mode: "auto".to_string(),
            enabled: true,
            next_trigger: None,
            created_at: String::new(),
            updated_at: String::new(),
        };

        let next_trigger = calculate_next_trigger(&reminder, "completed", now);

        assert_eq!(next_trigger, "2026-06-11T10:20:20");
    }

    fn prepare_pause_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("test database should open");
        conn.execute_batch(
            "
            CREATE TABLE reminders (
                id TEXT PRIMARY KEY,
                interval_minutes INTEGER NOT NULL,
                enabled INTEGER NOT NULL,
                next_trigger TEXT,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )
        .expect("test schema should be created");
        conn
    }

    #[test]
    fn pause_all_preserves_individual_enabled_state() {
        let conn = prepare_pause_test_db();
        conn.execute(
            "INSERT INTO reminders (id, interval_minutes, enabled, next_trigger, updated_at) VALUES ('on', 20, 1, '2026-07-01T10:20:00', ''), ('off', 30, 0, NULL, '')",
            [],
        )
        .expect("test reminders should be inserted");

        pause_all_reminders(&conn).expect("pause should succeed");

        let states = conn
            .prepare("SELECT id, enabled, next_trigger FROM reminders ORDER BY id")
            .expect("query should prepare")
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .expect("query should run")
            .collect::<Result<Vec<_>, _>>()
            .expect("rows should collect");

        assert_eq!(
            states,
            vec![
                ("off".to_string(), 0, None),
                ("on".to_string(), 1, Some("2026-07-01T10:20:00".to_string())),
            ]
        );
        assert!(all_reminders_paused(&conn));
    }

    #[test]
    fn resume_all_recovers_legacy_disabled_all_state() {
        let conn = prepare_pause_test_db();
        conn.execute(
            "INSERT INTO reminders (id, interval_minutes, enabled, next_trigger, updated_at) VALUES ('drink', 90, 0, NULL, '')",
            [],
        )
        .expect("test reminder should be inserted");

        resume_all_reminders(&conn).expect("resume should succeed");

        let (enabled, next_trigger): (i32, Option<String>) = conn
            .query_row(
                "SELECT enabled, next_trigger FROM reminders WHERE id = 'drink'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("reminder should exist");

        assert_eq!(enabled, 1);
        assert!(next_trigger.is_some());
        assert!(!all_reminders_paused(&conn));
    }
}
