use std::{
    collections::{BTreeSet, HashMap, HashSet},
    io::Read,
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
pub(crate) const SYSTEM_PAUSED_AT_KEY: &str = "system_paused_at";
const TEMP_DND_UNTIL_KEY: &str = "temp_dnd_until";
const TEMP_DND_PERIODS_KEY: &str = "temp_dnd_compensated_periods";
const MIN_INTERVAL_MINUTES: i64 = 1;
const MAX_INTERVAL_MINUTES: i64 = 1440;
const MIN_BREAK_DURATION_MINUTES: i64 = 1;
const MAX_BREAK_DURATION_MINUTES: i64 = 120;
const MIN_ACTION_DURATION_SECONDS: i64 = 1;
const MAX_ACTION_DURATION_SECONDS: i64 = 7200;
const MAX_BACKUP_FILE_BYTES: u64 = 64 * 1024 * 1024;

fn now_utc_string() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn validate_backup_file_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return Err(crate::errors::code("backupExtension"));
    };

    if !extension.eq_ignore_ascii_case("json") {
        return Err(crate::errors::code("backupExtension"));
    }

    Ok(path)
}

fn validate_backup_content_size(content: &str) -> Result<(), String> {
    if content.len() as u64 > MAX_BACKUP_FILE_BYTES {
        return Err(crate::errors::params(
            "backupTooLarge",
            serde_json::json!({"max": 64}),
        ));
    }

    Ok(())
}

fn validate_existing_backup_file(path: &Path) -> Result<(), String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err(crate::errors::code("invalidBackup"));
    }

    if metadata.len() > MAX_BACKUP_FILE_BYTES {
        return Err(crate::errors::params(
            "backupTooLarge",
            serde_json::json!({"max": 64}),
        ));
    }

    Ok(())
}

#[cfg(test)]
fn build_next_trigger_from(base: chrono::DateTime<Utc>, minutes: i64) -> String {
    (base + Duration::minutes(minutes))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

// New schedules share the frozen clock of the currently active pauses. Resume
// can then shift every reminder once, including ones created or re-enabled
// halfway through a pause, without counting time before their creation.
pub(crate) fn schedule_base(
    conn: &Connection,
    now: chrono::DateTime<Utc>,
) -> chrono::DateTime<Utc> {
    let manual = all_reminders_paused(conn)
        .then(|| setting_timestamp(conn, ALL_REMINDERS_PAUSED_AT_KEY))
        .flatten();
    [manual, setting_timestamp(conn, SYSTEM_PAUSED_AT_KEY)]
        .into_iter()
        .flatten()
        .fold(now, |base, start| base.min(start))
}

fn build_next_trigger_for_state(conn: &Connection, minutes: i64) -> String {
    build_next_trigger_for_state_at(conn, minutes, Utc::now())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CompensatedPeriod {
    start: i64,
    end: i64,
}

fn temp_dnd_periods(conn: &Connection) -> Vec<CompensatedPeriod> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [TEMP_DND_PERIODS_KEY],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .and_then(|value| serde_json::from_str(&value).ok())
    .unwrap_or_default()
}

fn covered_seconds(periods: &[CompensatedPeriod], start: i64, end: i64) -> i64 {
    let mut spans: Vec<_> = periods
        .iter()
        .map(|p| (p.start.max(start), p.end.min(end)))
        .filter(|(left, right)| left < right)
        .collect();
    spans.sort_unstable();
    let mut cursor = start;
    let mut covered = 0;
    for (left, right) in spans {
        covered += right.saturating_sub(left.max(cursor)).max(0);
        cursor = cursor.max(right);
    }
    covered
}

pub(crate) fn build_next_trigger_for_state_at(
    conn: &Connection,
    minutes: i64,
    now: chrono::DateTime<Utc>,
) -> String {
    let base = schedule_base(conn, now);
    let periods = temp_dnd_periods(conn);
    let end = periods
        .iter()
        .map(|period| period.end)
        .max()
        .unwrap_or(now.timestamp());
    build_next_trigger_from_parts(
        base,
        minutes,
        covered_seconds(&periods, base.timestamp(), end),
    )
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
    if all_reminders_paused(conn) {
        return Ok(());
    }
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
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
    now: chrono::DateTime<Utc>,
) -> Result<(), String> {
    let paused_seconds = (end - start).num_seconds().max(0)
        - covered_seconds(&temp_dnd_periods(conn), start.timestamp(), end.timestamp());
    if paused_seconds == 0 {
        return Ok(());
    }
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

fn shift_enabled_reminder_schedule_for_temp_dnd(
    conn: &Connection,
    paused_seconds: i64,
    now: chrono::DateTime<Utc>,
    resume_at: chrono::DateTime<Utc>,
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
            .map(|timestamp| timestamp.and_utc())
            .filter(|timestamp| *timestamp > schedule_base(conn, now))
            .map(|timestamp| timestamp + Duration::seconds(paused_seconds))
            .unwrap_or_else(|| {
                if schedule_base(conn, now) < now {
                    NaiveDateTime::parse_from_str(
                        &build_next_trigger_for_state_at(conn, interval_minutes, now),
                        "%Y-%m-%dT%H:%M:%S",
                    )
                    .unwrap()
                    .and_utc()
                } else {
                    resume_at + Duration::minutes(interval_minutes)
                }
            })
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

pub(crate) fn start_temp_dnd(conn: &Connection, minutes: i64) -> Result<(), String> {
    start_temp_dnd_at(conn, minutes, Utc::now())
}

fn start_temp_dnd_at(
    conn: &Connection,
    minutes: i64,
    now: chrono::DateTime<Utc>,
) -> Result<(), String> {
    if !(1..=480).contains(&minutes) {
        return Err(crate::errors::params(
            "dndRange",
            serde_json::json!({"min": 1, "max": 480}),
        ));
    }

    let transaction = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let conn = &transaction;
    let requested_until = now + Duration::minutes(minutes);
    let previous_until = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [TEMP_DND_UNTIL_KEY],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|value| NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S").ok())
        .map(|timestamp| timestamp.and_utc())
        .filter(|until| *until > now);
    let resume_at = previous_until
        .map(|until| until.max(requested_until))
        .unwrap_or(requested_until);
    let paused_seconds = (resume_at - previous_until.unwrap_or(now))
        .num_seconds()
        .max(0);

    // Retain past DND intervals while an older manual/system pause is still
    // open. Those intervals have already shifted next_trigger and must not be
    // added again when that pause ends, even if DND expired in the meantime.
    let base = schedule_base(conn, now).timestamp();
    let mut periods = temp_dnd_periods(conn);
    periods.retain(|period| period.end > base);
    periods.push(CompensatedPeriod {
        start: previous_until.unwrap_or(now).timestamp(),
        end: resume_at.timestamp(),
    });
    periods.sort_by_key(|period| period.start);
    let mut merged: Vec<CompensatedPeriod> = Vec::new();
    for period in periods {
        if period.end <= period.start {
            continue;
        }
        if let Some(last) = merged.last_mut() {
            if period.start <= last.end {
                last.end = last.end.max(period.end);
                continue;
            }
        }
        merged.push(period);
    }
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        (
            TEMP_DND_PERIODS_KEY,
            serde_json::to_string(&merged).map_err(|e| e.to_string())?,
        ),
    )
    .map_err(|e| e.to_string())?;

    if paused_seconds > 0 {
        shift_enabled_reminder_schedule_for_temp_dnd(conn, paused_seconds, now, resume_at)?;
    }

    let until = resume_at.format("%Y-%m-%dT%H:%M:%S").to_string();
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        (TEMP_DND_UNTIL_KEY, &until),
    )
    .map_err(|e| e.to_string())?;

    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn resume_all_reminders(conn: &Connection) -> Result<(), String> {
    resume_all_reminders_at(conn, Utc::now())
}

pub(crate) fn setting_timestamp(conn: &Connection, key: &str) -> Option<chrono::DateTime<Utc>> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
        row.get::<_, String>(0)
    })
    .ok()
    .and_then(|value| NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S").ok())
    .map(|timestamp| timestamp.and_utc())
}

// An overlapping pause that is still active owns compensation from its start.
// Whichever pause ends first only accounts for its non-overlapping prefix.
fn exclusive_pause_seconds(
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
    other_start: Option<chrono::DateTime<Utc>>,
) -> i64 {
    (other_start.map(|other| other.min(end)).unwrap_or(end) - start)
        .num_seconds()
        .max(0)
}

pub(crate) fn compensate_system_pause(
    conn: &Connection,
    started_at: chrono::DateTime<Utc>,
    now: chrono::DateTime<Utc>,
) -> Result<(), String> {
    let transaction = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let conn = &transaction;
    crate::lifecycle::extend_deadlines(conn, (now - started_at).num_seconds())?;
    let manual_start = if all_reminders_paused(conn) {
        setting_timestamp(conn, ALL_REMINDERS_PAUSED_AT_KEY)
    } else {
        None
    };
    let seconds = exclusive_pause_seconds(started_at, now, manual_start);
    if seconds > 0 {
        shift_enabled_reminder_schedule(
            conn,
            started_at,
            started_at + Duration::seconds(seconds),
            now,
        )?;
    }
    conn.execute(
        "DELETE FROM settings WHERE key = ?1",
        [SYSTEM_PAUSED_AT_KEY],
    )
    .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

fn resume_all_reminders_at(conn: &Connection, now: chrono::DateTime<Utc>) -> Result<(), String> {
    if !all_reminders_paused(conn) {
        return Ok(());
    }
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
        let paused_seconds = exclusive_pause_seconds(
            paused_at,
            now,
            setting_timestamp(conn, SYSTEM_PAUSED_AT_KEY),
        );
        if paused_seconds > 0 {
            shift_enabled_reminder_schedule(
                conn,
                paused_at,
                paused_at + Duration::seconds(paused_seconds),
                now,
            )?;
        }
    }

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

fn validate_interval_minutes(minutes: i64) -> Result<(), String> {
    if !(MIN_INTERVAL_MINUTES..=MAX_INTERVAL_MINUTES).contains(&minutes) {
        return Err(crate::errors::code("interval"));
    }

    Ok(())
}

fn validate_required_text(value: &str, field_name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(crate::errors::code("required"));
    }
    let limit = if field_name == "提醒名称" {
        1024
    } else {
        16_384
    };
    if value.len() > limit {
        return Err(crate::errors::code("invalidReminder"));
    }

    Ok(())
}

fn validate_reminder_metadata(icon: &str, completion_mode: Option<&str>) -> Result<(), String> {
    if icon.len() > 128 || completion_mode.is_some_and(|mode| mode != "auto") {
        return Err(crate::errors::code("invalidReminder"));
    }
    Ok(())
}

fn validate_break_duration_minutes(minutes: i64) -> Result<(), String> {
    if !(MIN_BREAK_DURATION_MINUTES..=MAX_BREAK_DURATION_MINUTES).contains(&minutes) {
        return Err(crate::errors::code("breakDuration"));
    }

    Ok(())
}

fn validate_action_duration_seconds(seconds: i64) -> Result<(), String> {
    if !(MIN_ACTION_DURATION_SECONDS..=MAX_ACTION_DURATION_SECONDS).contains(&seconds) {
        return Err(crate::errors::params(
            "actionDuration",
            serde_json::json!({"min": MIN_ACTION_DURATION_SECONDS, "max": MAX_ACTION_DURATION_SECONDS}),
        ));
    }

    Ok(())
}

fn normalize_break_settings(
    reminder_type: &str,
    break_duration_minutes: i64,
    _break_notification_enabled: bool,
    action_enabled: bool,
    action_title: String,
    action_message: String,
    action_duration_seconds: i64,
) -> Result<(i64, bool, bool, String, String, i64, String), String> {
    if !matches!(reminder_type, "rest" | "eye_care" | "drink" | "custom")
        || action_title.len() > 1024
        || action_message.len() > 16_384
    {
        return Err(crate::errors::code("invalidReminder"));
    }
    // Resolve legacy defaults at the input boundary; an explicit false always wins.
    if action_enabled {
        validate_action_duration_seconds(action_duration_seconds)?;
        if reminder_type == "rest" {
            validate_break_duration_minutes(break_duration_minutes)?;
        }
    }
    Ok((
        if reminder_type == "rest" && action_enabled {
            break_duration_minutes
        } else {
            0
        },
        reminder_type == "rest" && action_enabled,
        action_enabled,
        action_title.trim().to_string(),
        action_message.trim().to_string(),
        if action_enabled {
            action_duration_seconds
        } else {
            0
        },
        "auto".to_string(),
    ))
}

fn validate_response_action(action: &str) -> Result<(), String> {
    match action {
        "completed" | "skipped" | "timeout" => Ok(()),
        _ => Err(crate::errors::code("invalidReminder")),
    }
}

pub(crate) fn get_reminder_by_id(conn: &Connection, id: &str) -> Result<Reminder, String> {
    conn.query_row(
        "SELECT id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at, template_fields FROM reminders WHERE id = ?1",
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
                template_fields: row.get::<_, Option<String>>(17)?.map(|value| serde_json::from_str(&value).unwrap_or_default()),
            })
        },
    )
    .map_err(|e| e.to_string())
}

fn resolve_local_datetime(date: NaiveDate) -> Result<chrono::DateTime<Local>, String> {
    let naive = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| crate::errors::code("invalidDate"))?;

    match Local.from_local_datetime(&naive) {
        LocalResult::Single(value) => Ok(value),
        LocalResult::Ambiguous(value, _) => Ok(value),
        LocalResult::None => Err(crate::errors::code("invalidDate")),
    }
}

fn local_day_bounds_utc(date: NaiveDate) -> Result<(String, String), String> {
    let next_date = date
        .checked_add_signed(Duration::days(1))
        .ok_or_else(|| crate::errors::code("invalidDate"))?;
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
        .prepare("SELECT id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at, template_fields FROM reminders ORDER BY created_at")
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
                template_fields: row
                    .get::<_, Option<String>>(17)?
                    .map(|value| serde_json::from_str(&value).unwrap_or_default()),
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
        frontend_settings: None,
    })
}

fn validate_import_data(data: &ExportData) -> Result<(), String> {
    if !matches!(data.version.as_str(), "1.0" | "1.1" | "1.2") {
        return Err(crate::errors::code("unsupportedBackupVersion"));
    }
    // Use the same byte budget as export and automatic snapshots, rather than
    // rejecting a valid exported history with a separate record-count ceiling.
    validate_backup_content_size(&serde_json::to_string(data).map_err(|e| e.to_string())?)?;

    let mut reminder_ids = HashSet::new();
    for reminder in &data.reminders {
        if reminder.id.trim().is_empty() {
            return Err(crate::errors::code("invalidBackup"));
        }
        if reminder.name.trim().is_empty() {
            return Err(crate::errors::code("invalidBackup"));
        }
        if reminder.message.trim().is_empty() {
            return Err(crate::errors::code("invalidBackup"));
        }
        if reminder.id.len() > 128
            || reminder.name.len() > 1024
            || reminder.message.len() > 16_384
            || reminder.icon.len() > 128
            || reminder.action_title.len() > 1024
            || reminder.action_message.len() > 16_384
        {
            return Err(crate::errors::code("invalidReminder"));
        }
        if !matches!(
            reminder.reminder_type.as_str(),
            "drink" | "rest" | "eye_care" | "custom"
        ) || reminder.action_completion_mode != "auto"
        {
            return Err(crate::errors::code("invalidReminder"));
        }
        crate::settings::timestamp(&reminder.created_at)?;
        crate::settings::timestamp(&reminder.updated_at)?;
        if let Some(next) = &reminder.next_trigger {
            crate::settings::timestamp(next)?;
        }
        validate_interval_minutes(reminder.interval_minutes)?;
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
            return Err(crate::errors::code("invalidBackup"));
        }
    }

    let mut log_ids = HashSet::new();
    for log in &data.logs {
        if log.id.trim().is_empty() {
            return Err(crate::errors::code("invalidBackup"));
        }
        if !log_ids.insert(log.id.as_str()) {
            return Err(crate::errors::code("invalidBackup"));
        }
        if log.id.len() > 128
            || log.reminder_id.trim().is_empty()
            || log.reminder_id.len() > 128
            || !matches!(
                log.action.as_str(),
                "completed" | "postponed" | "skipped" | "timeout" | "interrupted"
            )
        {
            return Err(crate::errors::code("invalidBackup"));
        }
        let triggered = crate::settings::timestamp(&log.triggered_at)?;
        if let Some(responded) = &log.responded_at {
            if crate::settings::timestamp(responded)? < triggered {
                return Err(crate::errors::code("invalidBackup"));
            }
        }
    }

    crate::settings::timestamp(&data.exported_at)?;
    for (key, value) in &data.settings {
        crate::settings::validate(key, value)?;
    }

    Ok(())
}

fn repair_imported_reminder_schedule(conn: &Connection) -> Result<(), String> {
    let now = Utc::now();
    if all_reminders_paused(conn)
        && setting_timestamp(conn, ALL_REMINDERS_PAUSED_AT_KEY).is_none_or(|start| start > now)
    {
        conn.execute(
            "INSERT OR REPLACE INTO settings VALUES (?1, ?2)",
            (
                ALL_REMINDERS_PAUSED_AT_KEY,
                now.format("%Y-%m-%dT%H:%M:%S").to_string(),
            ),
        )
        .map_err(|error| error.to_string())?;
    }
    let reference = schedule_base(conn, now);
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
                .map(|timestamp| timestamp.and_utc() <= reference)
                .unwrap_or(true),
            None => true,
        };

        if !should_reset {
            continue;
        }

        let new_next = build_next_trigger_for_state_at(conn, interval_minutes, now);

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
    let backup_path = backup_dir.join(format!(
        "before-import-{}-{}.json",
        timestamp,
        Uuid::new_v4()
    ));
    let snapshot = collect_export_data(conn)?;
    let snapshot_json = serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?;
    validate_backup_content_size(&snapshot_json)?;

    std::fs::write(&backup_path, snapshot_json).map_err(|e| e.to_string())?;
    prune_backup_files(&backup_dir, 10)?;

    Ok(backup_path.to_string_lossy().into_owned())
}

/// 获取所有提醒
#[tauri::command]
pub fn get_reminders(db: State<'_, Database>) -> Result<Vec<Reminder>, String> {
    let conn = db.conn.lock().unwrap();
    read_reminders(&conn)
}

fn read_reminders(conn: &Connection) -> Result<Vec<Reminder>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at, template_fields FROM reminders ORDER BY created_at")
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
                template_fields: row
                    .get::<_, Option<String>>(17)?
                    .map(|value| serde_json::from_str(&value).unwrap_or_default()),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(reminders)
}

#[derive(Serialize)]
pub struct ReminderOverview {
    reminders: Vec<Reminder>,
    #[serde(flatten)]
    pub runtime: RuntimeSnapshot,
}

#[derive(Debug, Serialize)]
pub struct ReminderRuntime {
    state: &'static str,
    reason: Option<&'static str>,
    until: Option<String>,
}

#[derive(Serialize)]
pub struct RuntimeSnapshot {
    revision: u64,
    all_paused: bool,
    suppression: Option<ReminderRuntime>,
    countdowns: HashMap<String, i64>,
    states: HashMap<String, ReminderRuntime>,
}

static RUNTIME_REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[tauri::command]
pub fn get_reminder_overview(
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
) -> Result<ReminderOverview, String> {
    read_reminder_overview(&db.conn.lock().unwrap(), &scheduler)
}

pub(crate) fn read_reminder_overview(
    conn: &Connection,
    scheduler: &Scheduler,
) -> Result<ReminderOverview, String> {
    let reminders = read_reminders(conn)?;
    let runtime = read_runtime(
        conn,
        &reminders,
        scheduler.overview_queue(),
        Utc::now(),
        crate::fullscreen::is_foreground_window_fullscreen(),
    )?;
    Ok(ReminderOverview { reminders, runtime })
}

fn read_runtime(
    conn: &Connection,
    reminders: &[Reminder],
    queue: crate::scheduler::QueueOverview,
    now: chrono::DateTime<Utc>,
    fullscreen: bool,
) -> Result<RuntimeSnapshot, String> {
    let manual = all_reminders_paused(conn);
    let system = setting_timestamp(conn, SYSTEM_PAUSED_AT_KEY).is_some();
    let paused = manual || system;
    let suppression = if paused {
        Some(ReminderRuntime {
            state: "paused",
            reason: Some(if manual { "manual" } else { "system" }),
            until: None,
        })
    } else {
        crate::scheduler::dnd_status(conn, now, fullscreen).map(|(reason, until)| ReminderRuntime {
            state: "dnd",
            reason: Some(reason),
            until,
        })
    };
    let reference = schedule_base(conn, now);
    let mut deadlines = conn
        .prepare(
            "SELECT reminder_id, deadline FROM active_notifications WHERE started_at IS NOT NULL",
        )
        .map_err(|e| e.to_string())?;
    let deadlines: HashMap<String, String> = deadlines
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    let mut countdowns = HashMap::new();
    let mut states = HashMap::new();
    for reminder in reminders {
        let deadline = deadlines
            .get(&reminder.id)
            .or(reminder.next_trigger.as_ref());
        let remaining = deadline
            .and_then(|value| crate::settings::timestamp(value).ok())
            .map(|time| (((time - reference).num_milliseconds() + 999) / 1000).max(0));
        if reminder.enabled {
            if let Some(seconds) = remaining {
                countdowns.insert(reminder.id.clone(), seconds);
            }
        }
        let state = if !reminder.enabled {
            "disabled"
        } else if paused {
            "paused"
        } else if deadlines.contains_key(&reminder.id) {
            "acting"
        } else if queue.current.as_deref() == Some(reminder.id.as_str()) {
            "notifying"
        } else if suppression.is_some() {
            "dnd"
        } else if queue.pending.contains(&reminder.id) {
            "queued"
        } else if remaining == Some(0) {
            "due"
        } else {
            "scheduled"
        };
        let suppressed = matches!(state, "paused" | "dnd");
        states.insert(
            reminder.id.clone(),
            ReminderRuntime {
                state,
                reason: if suppressed {
                    suppression.as_ref().and_then(|s| s.reason)
                } else {
                    None
                },
                until: if suppressed {
                    suppression.as_ref().and_then(|s| s.until.clone())
                } else {
                    None
                },
            },
        );
    }
    Ok(RuntimeSnapshot {
        revision: RUNTIME_REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1,
        all_paused: paused,
        suppression,
        countdowns,
        states,
    })
}

/// 创建提醒
#[tauri::command]
pub fn create_reminder(
    db: State<'_, Database>,
    request: CreateReminderRequest,
) -> Result<Reminder, String> {
    validate_interval_minutes(request.interval_minutes)?;
    validate_required_text(&request.name, "提醒名称")?;
    validate_required_text(&request.message, "提醒内容")?;
    validate_reminder_metadata(&request.icon, request.action_completion_mode.as_deref())?;
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
                "rest" => requested_break_duration.saturating_mul(60),
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
        Some(build_next_trigger_for_state(
            &conn,
            request.interval_minutes,
        ))
    } else {
        None
    };

    conn.execute(
        "INSERT INTO reminders (id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at, template_fields) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, '[]')",
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
        template_fields: Some(vec![]),
    })
}

/// 更新提醒
#[tauri::command]
pub fn update_reminder(
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    id: String,
    request: UpdateReminderRequest,
) -> Result<Reminder, String> {
    if let Some(interval_minutes) = request.interval_minutes {
        validate_interval_minutes(interval_minutes)?;
    }

    let conn = db.conn.lock().unwrap();
    let now = now_utc_string();
    let current = get_reminder_by_id(&conn, &id)?;
    let template_fields = crate::template::after_edit(&current, &request);

    let name = request.name.unwrap_or(current.name);
    let reminder_type = request.reminder_type.unwrap_or(current.reminder_type);
    let icon = request.icon.unwrap_or(current.icon);
    let message = request.message.unwrap_or(current.message);
    validate_reminder_metadata(&icon, request.action_completion_mode.as_deref())?;
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
            Some(build_next_trigger_for_state(&conn, interval_minutes))
        } else {
            current.next_trigger
        }
    } else {
        None
    };

    conn.execute(
        "UPDATE reminders SET name = ?1, reminder_type = ?2, icon = ?3, message = ?4, interval_minutes = ?5, break_duration_minutes = ?6, break_notification_enabled = ?7, action_enabled = ?8, action_title = ?9, action_message = ?10, action_duration_seconds = ?11, action_completion_mode = ?12, enabled = ?13, next_trigger = ?14, updated_at = ?15, template_fields = ?17 WHERE id = ?16",
        params![&name, &reminder_type, &icon, &message, interval_minutes, break_duration_minutes, break_notification_enabled as i32, action_enabled as i32, &action_title, &action_message, action_duration_seconds, &action_completion_mode, enabled as i32, &next_trigger, &now, &id, serde_json::to_string(&template_fields).unwrap()],
    ).map_err(|e| e.to_string())?;
    if !enabled {
        crate::lifecycle::interrupt(&conn, Some(&id), Utc::now())?;
        scheduler.release_notification(&id);
    }
    drop(conn);

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
        template_fields: Some(template_fields),
    })
}

/// 删除提醒
#[tauri::command]
pub fn delete_reminder(
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    id: String,
) -> Result<(), String> {
    let conn = db.conn.lock().unwrap();
    crate::lifecycle::interrupt(&conn, Some(&id), Utc::now())?;
    conn.execute("DELETE FROM reminder_logs WHERE reminder_id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM reminders WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    drop(conn);

    scheduler.release_notification(&id);

    Ok(())
}

/// 切换提醒启用/禁用
#[tauri::command]
pub fn toggle_reminder(
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    id: String,
) -> Result<Reminder, String> {
    let conn = db.conn.lock().unwrap();
    let now = now_utc_string();
    let current = get_reminder_by_id(&conn, &id)?;

    let enabled = !current.enabled;
    let next_trigger = if enabled {
        Some(build_next_trigger_for_state(
            &conn,
            current.interval_minutes,
        ))
    } else {
        None
    };

    conn.execute(
        "UPDATE reminders SET enabled = ?1, next_trigger = ?2, updated_at = ?3 WHERE id = ?4",
        (enabled as i32, &next_trigger, &now, &id),
    )
    .map_err(|e| e.to_string())?;
    if !enabled {
        crate::lifecycle::interrupt(&conn, Some(&id), Utc::now())?;
        scheduler.release_notification(&id);
    }
    drop(conn);

    Ok(Reminder {
        id: current.id,
        template_fields: current.template_fields,
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
    notification_id: String,
    action: String,
    hold_notification: Option<bool>,
) -> Result<Option<crate::lifecycle::ActionProgress>, String> {
    validate_response_action(&action)?;
    let conn = db.conn.lock().unwrap();
    validate_live_notification(&conn, &scheduler, &reminder_id, &notification_id)?;
    let current = get_reminder_by_id(&conn, &reminder_id)?;
    let result =
        crate::lifecycle::respond(&conn, &current, &notification_id, &action, None, Utc::now())?;
    // The backend decides whether this action has a countdown.
    let _ = hold_notification;
    if result.is_none() {
        scheduler.release_notification(&reminder_id);
    }
    let _ = app.emit("reminders:changed", ());
    Ok(result)
}

fn validate_live_notification(
    conn: &Connection,
    scheduler: &Scheduler,
    reminder_id: &str,
    notification_id: &str,
) -> Result<(), String> {
    if all_reminders_paused(conn)
        || setting_timestamp(conn, SYSTEM_PAUSED_AT_KEY).is_some()
        || !scheduler.is_current_notification(reminder_id, notification_id)
    {
        return Err(crate::errors::code("unavailable"));
    }
    Ok(())
}

#[tauri::command]
pub fn get_action_progress(
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    reminder_id: String,
    notification_id: String,
) -> Result<Option<crate::lifecycle::ActionProgress>, String> {
    let conn = db.conn.lock().unwrap();
    validate_live_notification(&conn, &scheduler, &reminder_id, &notification_id)?;
    crate::lifecycle::progress(&conn, &notification_id, Utc::now())
}

/// 获取今日统计
#[tauri::command]
pub fn get_daily_stats(db: State<'_, Database>) -> Result<Vec<ReminderStat>, String> {
    let conn = db.conn.lock().unwrap();
    read_daily_stats(&conn, Local::now().date_naive())
}

fn read_daily_stats(conn: &Connection, today: NaiveDate) -> Result<Vec<ReminderStat>, String> {
    let (start_utc, end_utc) = local_day_bounds_utc(today)?;

    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.name, r.icon, r.reminder_type,
                    COUNT(CASE WHEN l.action = 'completed' THEN 1 END) as completed_count,
                    COUNT(CASE WHEN l.action IN ('completed', 'skipped', 'timeout', 'interrupted') THEN 1 END) as total_count,
                    COUNT(CASE WHEN l.action = 'postponed' THEN 1 END) as postponed_count
              FROM reminders r
             LEFT JOIN reminder_logs l ON r.id = l.reminder_id AND COALESCE(l.responded_at, l.triggered_at) >= ?1 AND COALESCE(l.responded_at, l.triggered_at) < ?2
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
                postponed_count: row.get(6)?,
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
    pub postponed_count: i64,
}

/// 延后提醒
#[tauri::command]
pub fn postpone_reminder(
    app: AppHandle,
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    reminder_id: String,
    notification_id: String,
    minutes: i64,
) -> Result<(), String> {
    validate_interval_minutes(minutes)?;
    let conn = db.conn.lock().unwrap();
    validate_live_notification(&conn, &scheduler, &reminder_id, &notification_id)?;
    let current = get_reminder_by_id(&conn, &reminder_id)?;
    crate::lifecycle::respond(
        &conn,
        &current,
        &notification_id,
        "postponed",
        Some(minutes),
        Utc::now(),
    )?;
    scheduler.release_notification(&reminder_id);
    let _ = app.emit("reminders:changed", ());
    Ok(())
}

/// 完成当前动作，并短暂保留完成反馈。
#[tauri::command]
pub fn release_notification(
    app: AppHandle,
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    reminder_id: String,
    notification_id: String,
    finish_break_now: Option<bool>,
) -> Result<(), String> {
    let conn = db.conn.lock().unwrap();
    validate_live_notification(&conn, &scheduler, &reminder_id, &notification_id)?;
    if scheduler.is_completed_notification(&reminder_id, &notification_id) {
        return Ok(());
    }
    let current = get_reminder_by_id(&conn, &reminder_id)?;
    crate::lifecycle::finish(
        &conn,
        &current,
        &notification_id,
        finish_break_now.unwrap_or(false),
        Utc::now(),
    )?;
    scheduler.complete_notification(&conn, &reminder_id, &notification_id);
    let _ = app.emit("reminders:changed", ());
    Ok(())
}

/// 保存设置
#[tauri::command]
pub fn save_setting(
    app: AppHandle,
    db: State<'_, Database>,
    key: String,
    value: String,
) -> Result<(), String> {
    save_settings(app, db, HashMap::from([(key, value)]))
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    db: State<'_, Database>,
    settings: HashMap<String, String>,
) -> Result<(), String> {
    crate::settings::save_batch(&mut db.conn.lock().unwrap(), &settings)?;
    if let Some(language) = settings.get("language") {
        crate::refresh_tray_menu_text(&app, language);
    }
    let _ = app.emit("settings:changed", ());
    let _ = app.emit("reminders:changed", ());
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

#[tauri::command]
pub fn get_platform_capabilities() -> PlatformCapabilities {
    PlatformCapabilities::current()
}

/// 趋势统计数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendStat {
    pub date: String,
    pub completed_count: i64,
    pub total_count: i64,
    pub postponed_count: i64,
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

#[derive(Serialize)]
pub struct StatisticsSnapshot {
    daily: Vec<ReminderStat>,
    trend: Vec<TrendStat>,
    streaks: Vec<StreakInfo>,
}

#[tauri::command]
pub fn get_statistics(
    db: State<'_, Database>,
    days: Option<i64>,
) -> Result<StatisticsSnapshot, String> {
    let conn = db.conn.lock().unwrap();
    let today = Local::now().date_naive();
    Ok(StatisticsSnapshot {
        daily: read_daily_stats(&conn, today)?,
        trend: read_trend_stats(&conn, days.unwrap_or(7), today)?,
        streaks: read_streak_stats(&conn, today)?,
    })
}

/// 获取趋势统计（过去 N 天）
#[tauri::command]
pub fn get_trend_stats(
    db: State<'_, Database>,
    days: Option<i64>,
) -> Result<Vec<TrendStat>, String> {
    let conn = db.conn.lock().unwrap();
    read_trend_stats(&conn, days.unwrap_or(7), Local::now().date_naive())
}

fn read_trend_stats(
    conn: &Connection,
    days: i64,
    end_date: NaiveDate,
) -> Result<Vec<TrendStat>, String> {
    let days = days.clamp(1, 365);
    let start_date = end_date
        .checked_sub_signed(Duration::days(days - 1))
        .ok_or_else(|| crate::errors::code("invalidDate"))?;
    let (start_utc, _) = local_day_bounds_utc(start_date)?;
    let (_, end_utc) = local_day_bounds_utc(end_date)?;

    let mut stmt = conn
        .prepare(
            "SELECT COALESCE(responded_at, triggered_at), action
              FROM reminder_logs
              WHERE COALESCE(responded_at, triggered_at) >= ?1 AND COALESCE(responded_at, triggered_at) < ?2
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

    let mut stats_by_date: HashMap<NaiveDate, (i64, i64, i64)> = HashMap::new();
    for (triggered_at, action) in logs {
        if let Some(date) = parse_utc_timestamp_to_local_date(&triggered_at) {
            let entry = stats_by_date.entry(date).or_insert((0, 0, 0));
            if action == "completed" {
                entry.0 += 1;
            }
            if matches!(
                action.as_str(),
                "completed" | "skipped" | "timeout" | "interrupted"
            ) {
                entry.1 += 1;
            }
            if action == "postponed" {
                entry.2 += 1;
            }
        }
    }

    let mut stats = Vec::new();
    let mut current = start_date;
    while current <= end_date {
        let (completed_count, total_count, postponed_count) =
            stats_by_date.get(&current).copied().unwrap_or((0, 0, 0));
        stats.push(TrendStat {
            date: current.format("%Y-%m-%d").to_string(),
            completed_count,
            total_count,
            postponed_count,
        });
        current = current
            .checked_add_signed(Duration::days(1))
            .ok_or_else(|| crate::errors::code("invalidDate"))?;
    }

    Ok(stats)
}

/// 获取连续打卡天数
#[tauri::command]
pub fn get_streak_stats(db: State<'_, Database>) -> Result<Vec<StreakInfo>, String> {
    let conn = db.conn.lock().unwrap();
    read_streak_stats(&conn, Local::now().date_naive())
}

fn read_streak_stats(conn: &Connection, today: NaiveDate) -> Result<Vec<StreakInfo>, String> {
    // Historical achievements are independent of the current enabled switch.
    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.name, r.icon, r.reminder_type
             FROM reminders r",
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
                "SELECT COALESCE(responded_at, triggered_at), action
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
                if date <= today {
                    completed_dates.insert(date);
                }
            }
        }

        let mut current_streak = 0i64;
        let mut current_date = if completed_dates.contains(&today) {
            today
        } else {
            today
                .pred_opt()
                .ok_or_else(|| crate::errors::code("invalidDate"))?
        };
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
                    .ok_or_else(|| crate::errors::code("invalidDate"))?;
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
    #[serde(deserialize_with = "deserialize_backup_reminders")]
    pub reminders: Vec<Reminder>,
    #[serde(default)]
    pub logs: Vec<ReminderLog>,
    pub settings: std::collections::HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontend_settings: Option<HashMap<String, serde_json::Value>>,
}

fn prepare_import_data(mut data: ExportData) -> Result<ExportData, String> {
    if let Some(frontend) = data.frontend_settings.take() {
        for (field, value) in frontend {
            let (key, expected) = match field.as_str() {
                "theme" => ("theme", "string"),
                "mascotStyle" => ("mascot_style", "string"),
                "notificationAppearance" => ("notification_appearance", "string"),
                "language" => ("language", "string"),
                "notificationDuration" => ("notification_duration", "number"),
                "postponeOptions" => ("postpone_options", "array"),
                "soundEnabled" => ("sound_enabled", "boolean"),
                "soundPreset" => ("sound_preset", "string"),
                "soundVolume" => ("sound_volume", "number"),
                _ => {
                    return Err(crate::errors::params(
                        "invalidSetting",
                        serde_json::json!({"key": field}),
                    ))
                }
            };
            let typed = match expected {
                "string" => value.is_string(),
                "number" => value.is_i64(),
                "array" => value.is_array(),
                "boolean" => value.is_boolean(),
                _ => false,
            };
            if !typed {
                return Err(crate::errors::params(
                    "invalidSetting",
                    serde_json::json!({"key": field}),
                ));
            }
            let text = value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string());
            crate::settings::validate(key, &text)?;
            // Database settings are authoritative; old frontend-only backups still work.
            data.settings.entry(key.to_string()).or_insert(text);
        }
    }
    validate_import_data(&data)?;
    let normalize = |value: &str| {
        crate::settings::timestamp(value).map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string())
    };
    for reminder in &mut data.reminders {
        reminder.created_at = normalize(&reminder.created_at)?;
        reminder.updated_at = normalize(&reminder.updated_at)?;
        reminder.next_trigger = reminder
            .next_trigger
            .as_deref()
            .map(normalize)
            .transpose()?;
    }
    for log in &mut data.logs {
        log.triggered_at = normalize(&log.triggered_at)?;
        log.responded_at = log.responded_at.as_deref().map(normalize).transpose()?;
    }
    for key in [
        ALL_REMINDERS_PAUSED_AT_KEY,
        SYSTEM_PAUSED_AT_KEY,
        TEMP_DND_UNTIL_KEY,
    ] {
        if let Some(value) = data.settings.get_mut(key) {
            *value = normalize(value)?;
        }
    }
    Ok(data)
}

fn deserialize_backup_reminders<'de, D>(deserializer: D) -> Result<Vec<Reminder>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<serde_json::Value>::deserialize(deserializer)?;
    values
        .into_iter()
        .map(|mut value| {
            if let Some(fields) = value.as_object_mut() {
                let kind = fields
                    .get("reminder_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                if !fields.contains_key("action_enabled") {
                    let enabled = kind == "eye_care"
                        || (kind == "rest"
                            && fields
                                .get("break_notification_enabled")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false));
                    fields.insert("action_enabled".into(), enabled.into());
                    fields.entry("action_duration_seconds").or_insert_with(|| {
                        (if !enabled {
                            0
                        } else if kind == "eye_care" {
                            20
                        } else {
                            0
                        })
                        .into()
                    });
                    if enabled
                        && kind == "rest"
                        && fields
                            .get("action_duration_seconds")
                            .and_then(|v| v.as_i64())
                            == Some(0)
                    {
                        let seconds = fields
                            .get("break_duration_minutes")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0)
                            .saturating_mul(60);
                        fields.insert("action_duration_seconds".into(), seconds.into());
                    }
                }
            }
            serde_json::from_value(value).map_err(serde::de::Error::custom)
        })
        .collect()
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
    pub autostart_sync_failed: bool,
}

/// 导出所有数据
#[tauri::command]
pub fn export_data(db: State<'_, Database>) -> Result<ExportData, String> {
    let conn = db.conn.lock().unwrap();
    collect_export_data(&conn)
}

fn import_reminders(
    conn: &Connection,
    reminders: &[Reminder],
    mode: &ImportMode,
) -> Result<(), String> {
    let reminder_insert_sql = if matches!(mode, ImportMode::Merge) {
        "INSERT OR REPLACE INTO reminders (id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at, template_fields) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)"
    } else {
        "INSERT INTO reminders (id, name, reminder_type, icon, message, interval_minutes, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, enabled, next_trigger, created_at, updated_at, template_fields) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)"
    };

    for reminder in reminders {
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
        conn.execute(
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
                serde_json::to_string(&crate::template::fields(reminder)).unwrap(),
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 导入数据
#[tauri::command]
pub fn import_data(
    app: AppHandle,
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    data: serde_json::Value,
    mode: Option<ImportMode>,
) -> Result<ImportResult, String> {
    let data = prepare_import_data(serde_json::from_value(data).map_err(|error| {
        crate::app_log::warn(format!("Invalid backup: {error}"));
        crate::errors::code("invalidBackup")
    })?)?;

    let mode = mode.unwrap_or_default();
    let mut conn = db.conn.lock().unwrap();
    let backup_path = create_import_backup(&app, &conn)?;
    let previous_language =
        crate::template::ensure_language(&conn, "en-US").map_err(|e| e.to_string())?;
    let live_system_pause = setting_timestamp(&conn, SYSTEM_PAUSED_AT_KEY);
    let live_boot_marker = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'last_boot_marker'",
            [],
            |r| r.get::<_, String>(0),
        )
        .ok();
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    crate::lifecycle::interrupt_in_transaction(&tx, None, Utc::now())?;
    if matches!(mode, ImportMode::Replace) {
        tx.execute("DELETE FROM reminder_logs", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM reminders", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM settings", [])
            .map_err(|e| e.to_string())?;
    }

    import_reminders(&tx, &data.reminders, &mode)?;

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
        // A backup cannot describe the lock state of this running process.
        if key == SYSTEM_PAUSED_AT_KEY || key == "last_boot_marker" {
            continue;
        }
        tx.execute(settings_insert_sql, (key, value))
            .map_err(|e| e.to_string())?;
    }

    tx.execute(
        "DELETE FROM settings WHERE key = ?1",
        [SYSTEM_PAUSED_AT_KEY],
    )
    .map_err(|e| e.to_string())?;
    if let Some(start) = live_system_pause {
        tx.execute(
            "INSERT INTO settings VALUES (?1, ?2)",
            (
                SYSTEM_PAUSED_AT_KEY,
                start.format("%Y-%m-%dT%H:%M:%S").to_string(),
            ),
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(marker) = live_boot_marker {
        tx.execute(
            "INSERT OR REPLACE INTO settings VALUES ('last_boot_marker', ?1)",
            [marker],
        )
        .map_err(|e| e.to_string())?;
    }
    let language =
        crate::template::ensure_language(&tx, &previous_language).map_err(|e| e.to_string())?;
    crate::template::sync(&tx, &language).map_err(|e| e.to_string())?;
    repair_imported_reminder_schedule(&tx)?;
    tx.commit().map_err(|e| e.to_string())?;

    scheduler.clear_all_active(&conn)?;
    if all_reminders_paused(&conn) {
        crate::set_tray_visual_state(&app, crate::TrayVisualState::Muted);
    }
    let language = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'language'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "en-US".into());
    let auto_start = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'auto_start'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_default()
        == "true";
    drop(conn);
    let autostart_warning = reconcile_imported_autostart(&app, auto_start).err();
    if let Some(error) = &autostart_warning {
        crate::app_log::warn(format!("Imported autostart sync failed: {error}"));
    }
    crate::refresh_tray_menu_text(&app, &language);
    let _ = app.emit("settings:changed", ());
    app.emit("reminders:changed", ())
        .map_err(|e| e.to_string())?;

    Ok(ImportResult {
        mode,
        backup_path: backup_path.clone(),
        reminders_imported: data.reminders.len(),
        logs_imported,
        settings_imported: data.settings.len(),
        skipped_logs,
        autostart_sync_failed: autostart_warning.is_some(),
    })
}

fn reconcile_imported_autostart(app: &AppHandle, desired: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use tauri_plugin_autostart::ManagerExt;
        let autostart = app.autolaunch();
        crate::settings::reconcile_autostart(
            desired,
            || autostart.is_enabled().map_err(|e| e.to_string()),
            |enabled| {
                (if enabled {
                    autostart.enable()
                } else {
                    autostart.disable()
                })
                .map_err(|e| e.to_string())
            },
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, desired);
        Ok(())
    }
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
    let mut content = String::new();
    std::fs::File::open(&path)
        .map_err(|e| e.to_string())?
        .take(MAX_BACKUP_FILE_BYTES + 1)
        .read_to_string(&mut content)
        .map_err(|e| e.to_string())?;
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
        scheduler.clear_all_active(&conn)?;
        crate::set_tray_visual_state(&app, crate::TrayVisualState::Muted);
    }
    drop(conn);

    app.emit("reminders:changed", ())
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 设置临时免打扰
#[tauri::command]
pub fn set_temp_dnd(
    app: AppHandle,
    db: State<'_, Database>,
    scheduler: State<'_, Scheduler>,
    minutes: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().unwrap();
    start_temp_dnd(&conn, minutes)?;

    scheduler.clear_all_active(&conn)?;
    drop(conn);
    app.emit("reminders:changed", ())
        .map_err(|e| e.to_string())?;
    crate::set_tray_visual_state(&app, crate::TrayVisualState::Muted);

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
    fn failed_system_resume_rolls_back_action_and_schedule_compensation_before_retry() {
        let db = Database::in_memory();
        let conn = db.conn.lock().unwrap();
        let start = crate::settings::timestamp("2026-10-10T01:00:00").unwrap();
        conn.execute("INSERT INTO reminders (id,name,next_trigger,created_at,updated_at) VALUES ('one','One','2026-10-10T02:00:00','','')", []).unwrap();
        crate::lifecycle::shown(&conn, "one", "instance", start).unwrap();
        conn.execute("UPDATE active_notifications SET started_at = '2026-10-10T01:00:00', deadline = '2026-10-10T01:00:20'", []).unwrap();
        conn.execute(
            "INSERT INTO settings VALUES ('system_paused_at', '2026-10-10T01:00:00')",
            [],
        )
        .unwrap();
        conn.execute_batch("CREATE TRIGGER fail_schedule BEFORE UPDATE ON reminders BEGIN SELECT RAISE(ABORT, 'disk full'); END;").unwrap();
        assert!(compensate_system_pause(&conn, start, start + Duration::minutes(1)).is_err());
        assert_eq!(
            crate::lifecycle::progress(&conn, "instance", start)
                .unwrap()
                .unwrap()
                .remaining_seconds,
            20
        );
        assert!(setting_timestamp(&conn, SYSTEM_PAUSED_AT_KEY).is_some());
        conn.execute_batch("DROP TRIGGER fail_schedule").unwrap();
        compensate_system_pause(&conn, start, start + Duration::minutes(1)).unwrap();
        assert_eq!(
            crate::lifecycle::progress(&conn, "instance", start)
                .unwrap()
                .unwrap()
                .remaining_seconds,
            80
        );
        assert!(setting_timestamp(&conn, SYSTEM_PAUSED_AT_KEY).is_none());
    }

    #[test]
    fn runtime_snapshot_distinguishes_due_queue_action_pause_and_dnd() {
        let db = Database::in_memory();
        let conn = db.conn.lock().unwrap();
        let now = crate::settings::timestamp("2026-10-10T01:00:00").unwrap();
        conn.execute("INSERT INTO reminders (id,name,next_trigger,created_at,updated_at) VALUES ('one','One','2026-10-10T01:00:00','','')", []).unwrap();
        let reminders = read_reminders(&conn).unwrap();
        let read = |queue| read_runtime(&conn, &reminders, queue, now, false).unwrap();
        let due = read(crate::scheduler::QueueOverview::default());
        assert_eq!(due.countdowns["one"], 0);
        assert_eq!(due.states["one"].state, "due");
        let queued = read(crate::scheduler::QueueOverview {
            current: None,
            pending: HashSet::from(["one".into()]),
        });
        assert_eq!(queued.states["one"].state, "queued");
        let shown = read(crate::scheduler::QueueOverview {
            current: Some("one".into()),
            pending: HashSet::new(),
        });
        assert_eq!(shown.states["one"].state, "notifying");
        crate::lifecycle::shown(&conn, "one", "instance", now).unwrap();
        conn.execute("UPDATE active_notifications SET started_at = '2026-10-10T01:00:00', deadline = '2026-10-10T01:00:20'", []).unwrap();
        let acting = read(crate::scheduler::QueueOverview::default());
        assert_eq!(
            (acting.states["one"].state, acting.countdowns["one"]),
            ("acting", 20)
        );
        assert!(acting.revision > shown.revision);
        conn.execute(
            "INSERT INTO settings VALUES ('system_paused_at','2026-10-10T01:00:00')",
            [],
        )
        .unwrap();
        let paused = read(crate::scheduler::QueueOverview::default());
        assert_eq!(paused.states["one"].state, "paused");
        assert!(paused.all_paused);
        conn.execute("DELETE FROM settings", []).unwrap();
        conn.execute("DELETE FROM active_notifications", [])
            .unwrap();
        conn.execute(
            "INSERT INTO settings VALUES ('temp_dnd_until','2026-10-10T01:30:00')",
            [],
        )
        .unwrap();
        let dnd = read(crate::scheduler::QueueOverview::default());
        assert_eq!(dnd.states["one"].state, "dnd");
        assert_eq!(
            dnd.states["one"].until.as_deref(),
            Some("2026-10-10T01:30:00")
        );
    }

    #[test]
    fn statistics_keep_disabled_history_and_count_final_outcomes_on_response_day() {
        let db = Database::in_memory();
        let conn = db.conn.lock().unwrap();
        conn.execute("INSERT INTO reminders (id, name, created_at, updated_at) VALUES ('one', 'One', '', '')", []).unwrap();
        let today = Local::now().date_naive();
        let (midnight, _) = local_day_bounds_utc(today).unwrap();
        let before = (crate::settings::timestamp(&midnight).unwrap() - Duration::seconds(10))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        for (id, action) in [
            ("one", "postponed"),
            ("two", "postponed"),
            ("three", "completed"),
        ] {
            conn.execute(
                "INSERT INTO reminder_logs VALUES (?1, 'one', ?2, ?3, ?4)",
                (id, action, &before, &midnight),
            )
            .unwrap();
        }
        for enabled in [true, false] {
            conn.execute("UPDATE reminders SET enabled = ?1", [enabled])
                .unwrap();
            let daily = read_daily_stats(&conn, today).unwrap();
            let trend = read_trend_stats(&conn, 1, today).unwrap();
            assert_eq!(
                (
                    daily[0].completed_count,
                    daily[0].total_count,
                    daily[0].postponed_count
                ),
                (1, 1, 2)
            );
            assert_eq!(
                (
                    trend[0].completed_count,
                    trend[0].total_count,
                    trend[0].postponed_count
                ),
                (1, 1, 2)
            );
            assert_eq!(
                read_streak_stats(&conn, today).unwrap()[0].current_streak,
                1
            );
        }
    }

    #[test]
    fn streak_is_retained_until_today_ends_and_includes_history_older_than_ninety_days() {
        let db = Database::in_memory();
        let conn = db.conn.lock().unwrap();
        conn.execute("INSERT INTO reminders (id, name, created_at, updated_at) VALUES ('one', 'One', '', '')", []).unwrap();
        let today = Local::now().date_naive();
        for days in 1..=120 {
            let date = today - Duration::days(days);
            let (timestamp, _) = local_day_bounds_utc(date).unwrap();
            conn.execute(
                "INSERT INTO reminder_logs VALUES (?1, 'one', 'completed', ?1, ?1)",
                [timestamp],
            )
            .unwrap();
        }
        let stats = read_streak_stats(&conn, today).unwrap();
        assert_eq!((stats[0].current_streak, stats[0].max_streak), (120, 120));
        let next = read_streak_stats(&conn, today.succ_opt().unwrap()).unwrap();
        assert_eq!((next[0].current_streak, next[0].max_streak), (0, 120));
    }

    #[test]
    fn import_validates_legacy_frontend_settings_versions_and_log_times() {
        let db = Database::in_memory();
        let base = collect_export_data(&db.conn.lock().unwrap()).unwrap();
        for version in ["1.0", "1.1", "1.2"] {
            let mut data = base.clone();
            data.version = version.into();
            data.frontend_settings = Some(HashMap::from([
                ("soundVolume".into(), 0.into()),
                ("theme".into(), "dark".into()),
            ]));
            let imported = prepare_import_data(data).unwrap();
            assert_eq!(imported.settings["sound_volume"], "0");
            assert_eq!(imported.settings["theme"], "dark");
        }
        let mut future = base.clone();
        future.version = "99".into();
        assert!(prepare_import_data(future).is_err());
        let mut bad = base.clone();
        bad.frontend_settings = Some(HashMap::from([(
            "notificationDuration".into(),
            "NaN".into(),
        )]));
        assert!(prepare_import_data(bad).is_err());
        let mut log = ReminderLog {
            id: "id".into(),
            reminder_id: "reminder".into(),
            action: "completed".into(),
            triggered_at: "2026-10-10T09:00:00+08:00".into(),
            responded_at: Some("2026-10-10T01:01:00Z".into()),
        };
        let mut data = base.clone();
        data.logs.push(log.clone());
        assert_eq!(
            prepare_import_data(data).unwrap().logs[0].triggered_at,
            "2026-10-10T01:00:00"
        );
        for (action, triggered, responded) in [
            ("invented", "2026-10-10T01:00:00", "2026-10-10T01:01:00"),
            ("completed", "invalid", "2026-10-10T01:01:00"),
            ("completed", "2026-10-10T01:01:00", "2026-10-10T01:00:00"),
        ] {
            log.action = action.into();
            log.triggered_at = triggered.into();
            log.responded_at = Some(responded.into());
            let mut data = base.clone();
            data.logs.push(log.clone());
            assert!(prepare_import_data(data).is_err());
        }
    }

    #[test]
    fn explicit_action_switch_is_respected_for_every_reminder_type() {
        for kind in ["rest", "eye_care", "drink", "custom"] {
            for enabled in [false, true] {
                let result = normalize_break_settings(
                    kind,
                    if enabled { 5 } else { 0 },
                    true,
                    enabled,
                    String::new(),
                    String::new(),
                    if enabled { 300 } else { 0 },
                )
                .unwrap();
                assert_eq!(result.2, enabled, "{kind}");
                assert_eq!(result.5, if enabled { 300 } else { 0 });
            }
            assert!(normalize_break_settings(
                kind,
                5,
                false,
                true,
                String::new(),
                String::new(),
                0
            )
            .is_err());
        }
    }

    #[test]
    fn backup_distinguishes_missing_action_switch_from_explicit_false() {
        let base = serde_json::json!({"version":"1.2", "exported_at":"", "settings":{},
            "reminders":[{"id":"eye", "name":"Eye", "reminder_type":"eye_care", "icon":"eye_care",
                "message":"Look away", "interval_minutes":20, "enabled":true, "created_at":"", "updated_at":""}]});
        let legacy: ExportData = serde_json::from_value(base.clone()).unwrap();
        assert!(legacy.reminders[0].action_enabled);
        assert_eq!(legacy.reminders[0].action_duration_seconds, 20);
        let mut explicit = base;
        explicit["reminders"][0]["action_enabled"] = false.into();
        let imported: ExportData = serde_json::from_value(explicit).unwrap();
        assert!(!imported.reminders[0].action_enabled);
        assert_eq!(imported.reminders[0].action_duration_seconds, 0);
    }

    #[test]
    fn reminder_writes_reject_values_that_cannot_round_trip_through_backups() {
        assert!(validate_required_text(&"n".repeat(1025), "提醒名称").is_err());
        assert!(validate_reminder_metadata("custom", Some("manual")).is_err());
        assert!(normalize_break_settings(
            "unknown",
            0,
            false,
            false,
            String::new(),
            String::new(),
            0
        )
        .is_err());
        assert!(normalize_break_settings(
            "rest",
            i64::MAX,
            true,
            true,
            String::new(),
            String::new(),
            i64::MAX.saturating_mul(60)
        )
        .is_err());
    }

    #[test]
    fn accepts_json_backup_file_path() {
        let path = validate_backup_file_path("backup.JSON").expect("json path should be accepted");

        assert_eq!(path, PathBuf::from("backup.JSON"));
    }

    #[test]
    fn rejects_non_json_backup_file_path() {
        let error =
            validate_backup_file_path("backup.txt").expect_err("non-json path should be rejected");

        assert_eq!(error, crate::errors::code("backupExtension"));
    }

    #[test]
    fn rejects_oversized_backup_content() {
        let content = "x".repeat(MAX_BACKUP_FILE_BYTES as usize + 1);
        let error =
            validate_backup_content_size(&content).expect_err("oversized content should fail");

        assert_eq!(
            error,
            crate::errors::params("backupTooLarge", serde_json::json!({"max": 64}))
        );
    }

    #[test]
    fn large_history_backup_round_trips_through_file_commands() {
        let db = Database::in_memory();
        db.conn.lock().unwrap().execute(
            "INSERT INTO reminders (id, name, message, created_at, updated_at) VALUES ('reminder-id', 'Reminder', 'Take a break', '2026-10-10T01:00:00', '2026-10-10T01:00:00')", []
        ).unwrap();
        let mut data = collect_export_data(&db.conn.lock().unwrap()).unwrap();
        for i in 0..210_000 {
            data.logs.push(ReminderLog {
                id: format!("history-log-{i:06}"),
                reminder_id: "reminder-id".into(),
                action: "completed".into(),
                triggered_at: "2026-10-10T01:00:00".into(),
                responded_at: Some("2026-10-10T01:00:20".into()),
            });
        }
        let content = serde_json::to_string_pretty(&data).unwrap();
        assert!(content.len() > 2 * 1024 * 1024);
        let path = std::env::temp_dir()
            .join(format!("time-remind-{}.json", Uuid::new_v4()))
            .to_string_lossy()
            .into_owned();
        write_file(path.clone(), content).unwrap();
        let restored: ExportData = serde_json::from_str(&read_file(path.clone()).unwrap()).unwrap();
        assert_eq!(restored.logs.len(), 210_000);
        validate_import_data(&restored).unwrap();
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_import_with_empty_reminder_message() {
        let reminder = Reminder {
            id: "broken".to_string(),
            template_fields: None,
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
            frontend_settings: None,
        };

        let error = validate_import_data(&data).expect_err("empty message should fail");

        assert_eq!(error, crate::errors::code("invalidBackup"));
    }

    #[test]
    fn backup_import_preserves_custom_fields_and_infers_only_legacy_builtin_fields() {
        let source = Database::in_memory();
        let mut conn = source.conn.lock().unwrap();
        crate::initialize_default_reminders(&mut conn, "zh-CN").unwrap();
        conn.execute(
            "UPDATE reminders SET template_fields = '[]' WHERE id = 'default-drink'",
            [],
        )
        .unwrap();
        conn.execute("UPDATE reminders SET template_fields = NULL, name = 'Custom rest' WHERE id = 'default-rest'", []).unwrap();
        let backup = collect_export_data(&conn).unwrap();
        let backup: ExportData =
            serde_json::from_str(&serde_json::to_string(&backup).unwrap()).unwrap();
        let backup = prepare_import_data(backup).unwrap();
        for mode in [ImportMode::Replace, ImportMode::Merge] {
            let destination = Database::in_memory();
            let conn = destination.conn.lock().unwrap();
            import_reminders(&conn, &backup.reminders, &mode).unwrap();
            crate::template::sync(&conn, "en-US").unwrap();
            let rest = get_reminder_by_id(&conn, "default-rest").unwrap();
            assert_eq!(rest.name, "Custom rest");
            assert_eq!(
                rest.message,
                crate::reminder_template_text("en-US").rest_message
            );
            let drink = get_reminder_by_id(&conn, "default-drink").unwrap();
            assert_eq!(
                drink.name,
                crate::reminder_template_text("zh-CN").drink_name
            );
            assert_eq!(drink.template_fields, Some(vec![]));
            let exported = collect_export_data(&conn).unwrap();
            assert!(exported
                .reminders
                .iter()
                .all(|reminder| reminder.template_fields.is_some()));
        }
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
        conn.execute_batch(crate::lifecycle::SCHEMA).unwrap();
        conn
    }

    #[test]
    fn manual_and_system_pause_overlap_is_compensated_once_in_every_order() {
        let base = NaiveDateTime::parse_from_str("2026-09-20T10:00:00", "%Y-%m-%dT%H:%M:%S")
            .unwrap()
            .and_utc();
        // Event offsets in minutes: manual start/end and system start/end.
        for (manual_start, manual_end, system_start, system_end) in [
            (0, 30, 10, 20),
            (0, 20, 10, 30),
            (10, 30, 0, 20),
            (10, 20, 0, 30),
            (0, 10, 20, 30),
            (20, 30, 0, 10),
            (0, 30, 0, 30),
        ] {
            let conn = prepare_pause_test_db();
            conn.execute(
                "INSERT INTO reminders VALUES ('test', 20, 1, '2026-09-20T10:05:00', '')",
                [],
            )
            .unwrap();
            let mut events = vec![
                (manual_start, 0),
                (manual_end, 1),
                (system_start, 2),
                (system_end, 3),
            ];
            events.sort();
            for (offset, kind) in events {
                let at = base + Duration::minutes(offset);
                match kind {
                    0 => {
                        conn.execute(
                            "INSERT OR REPLACE INTO settings VALUES (?1, 'true')",
                            [ALL_REMINDERS_PAUSED_KEY],
                        )
                        .unwrap();
                        conn.execute(
                            "INSERT OR REPLACE INTO settings VALUES (?1, ?2)",
                            (
                                ALL_REMINDERS_PAUSED_AT_KEY,
                                at.format("%Y-%m-%dT%H:%M:%S").to_string(),
                            ),
                        )
                        .unwrap();
                    }
                    1 => resume_all_reminders_at(&conn, at).unwrap(),
                    2 => {
                        conn.execute(
                            "INSERT OR REPLACE INTO settings VALUES (?1, ?2)",
                            (
                                SYSTEM_PAUSED_AT_KEY,
                                at.format("%Y-%m-%dT%H:%M:%S").to_string(),
                            ),
                        )
                        .unwrap();
                    }
                    3 => compensate_system_pause(&conn, base + Duration::minutes(system_start), at)
                        .unwrap(),
                    _ => unreachable!(),
                }
            }
            let overlap = (manual_end.min(system_end) - manual_start.max(system_start)).max(0);
            let total = manual_end - manual_start + system_end - system_start - overlap;
            let next: String = conn
                .query_row("SELECT next_trigger FROM reminders", [], |row| row.get(0))
                .unwrap();
            assert_eq!(
                next,
                (base + Duration::minutes(5 + total))
                    .format("%Y-%m-%dT%H:%M:%S")
                    .to_string(),
                "manual {manual_start}..{manual_end}, system {system_start}..{system_end}"
            );
        }
    }

    #[test]
    fn system_resume_preserves_remaining_time_and_repairs_missing_schedule() {
        let conn = prepare_pause_test_db();
        conn.execute("INSERT INTO reminders VALUES ('scheduled', 20, 1, '2026-09-20T10:05:00', ''), ('missing', 20, 1, NULL, '')", []).unwrap();
        let start = NaiveDateTime::parse_from_str("2026-09-20T10:00:00", "%Y-%m-%dT%H:%M:%S")
            .unwrap()
            .and_utc();
        compensate_system_pause(&conn, start, start + Duration::minutes(30)).unwrap();
        for (id, expected) in [
            ("scheduled", "2026-09-20T10:35:00"),
            ("missing", "2026-09-20T10:50:00"),
        ] {
            let next: String = conn
                .query_row(
                    "SELECT next_trigger FROM reminders WHERE id = ?1",
                    [id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(next, expected);
        }
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
    fn repeated_pause_preserves_the_original_baseline() {
        let conn = prepare_pause_test_db();
        conn.execute(
            "INSERT INTO reminders VALUES ('test', 20, 1, '2026-09-20T10:05:00', '')",
            [],
        )
        .unwrap();
        pause_all_reminders(&conn).unwrap();
        conn.execute(
            "UPDATE settings SET value = '2026-09-20T10:00:00' WHERE key = ?1",
            [ALL_REMINDERS_PAUSED_AT_KEY],
        )
        .unwrap();
        pause_all_reminders(&conn).unwrap();
        pause_all_reminders(&conn).unwrap();
        let start = setting_timestamp(&conn, ALL_REMINDERS_PAUSED_AT_KEY).unwrap();
        assert_eq!(
            start.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "2026-09-20T10:00:00"
        );
        resume_all_reminders_at(&conn, start + Duration::minutes(30)).unwrap();
        let next: String = conn
            .query_row("SELECT next_trigger FROM reminders", [], |row| row.get(0))
            .unwrap();
        assert_eq!(next, "2026-09-20T10:35:00");
    }

    #[test]
    fn resume_preserves_individually_disabled_reminders_even_when_all_are_off() {
        for paused in [false, true] {
            let conn = prepare_pause_test_db();
            conn.execute("INSERT INTO reminders VALUES ('off', 20, 0, NULL, '')", [])
                .unwrap();
            if paused {
                pause_all_reminders(&conn).unwrap();
            }
            resume_all_reminders(&conn).unwrap();
            resume_all_reminders(&conn).unwrap();
            let (enabled, next): (bool, Option<String>) = conn
                .query_row("SELECT enabled, next_trigger FROM reminders", [], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .unwrap();
            assert!(!enabled);
            assert!(next.is_none());
            assert!(!all_reminders_paused(&conn));
        }
    }

    #[test]
    fn resume_without_a_pause_timestamp_does_not_guess_legacy_enabled_state() {
        let conn = prepare_pause_test_db();
        conn.execute("INSERT INTO reminders VALUES ('off', 20, 0, NULL, '')", [])
            .unwrap();
        conn.execute(
            "INSERT INTO settings VALUES (?1, 'true')",
            [ALL_REMINDERS_PAUSED_KEY],
        )
        .unwrap();
        resume_all_reminders(&conn).unwrap();
        let enabled: bool = conn
            .query_row("SELECT enabled FROM reminders", [], |row| row.get(0))
            .unwrap();
        assert!(!enabled);
        assert!(!all_reminders_paused(&conn));
    }

    #[test]
    fn repeated_resume_does_not_shift_schedule_twice_or_enable_disabled_items() {
        let conn = prepare_pause_test_db();
        conn.execute("INSERT INTO reminders VALUES ('on', 20, 1, '2026-09-20T10:05:00', ''), ('off', 20, 0, NULL, '')", []).unwrap();
        pause_all_reminders(&conn).unwrap();
        conn.execute(
            "UPDATE settings SET value = '2026-09-20T10:00:00' WHERE key = ?1",
            [ALL_REMINDERS_PAUSED_AT_KEY],
        )
        .unwrap();
        let now = NaiveDateTime::parse_from_str("2026-09-20T10:30:00", "%Y-%m-%dT%H:%M:%S")
            .unwrap()
            .and_utc();
        resume_all_reminders_at(&conn, now).unwrap();
        resume_all_reminders_at(&conn, now + Duration::minutes(10)).unwrap();
        let next: String = conn
            .query_row(
                "SELECT next_trigger FROM reminders WHERE id = 'on'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(next, "2026-09-20T10:35:00");
        let enabled: bool = conn
            .query_row(
                "SELECT enabled FROM reminders WHERE id = 'off'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!enabled);
    }

    #[test]
    fn temp_dnd_keeps_enabled_reminders_staggered() {
        let conn = prepare_pause_test_db();
        conn.execute(
            "INSERT INTO reminders (id, interval_minutes, enabled, next_trigger, updated_at) VALUES ('drink', 90, 1, '2026-07-01T10:10:00', ''), ('eye', 20, 1, '2026-07-01T10:35:00', '')",
            [],
        )
        .expect("test reminders should be inserted");
        let now = Utc
            .with_ymd_and_hms(2026, 7, 1, 10, 0, 0)
            .single()
            .expect("fixed test time should be valid");
        let resume_at = now + Duration::hours(1);

        shift_enabled_reminder_schedule_for_temp_dnd(&conn, 3600, now, resume_at)
            .expect("temporary dnd should shift reminder schedules");

        let schedules = conn
            .prepare("SELECT id, next_trigger FROM reminders ORDER BY id")
            .expect("query should prepare")
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .expect("query should run")
            .collect::<Result<Vec<_>, _>>()
            .expect("rows should collect");

        assert_eq!(
            schedules,
            vec![
                ("drink".to_string(), "2026-07-01T11:10:00".to_string()),
                ("eye".to_string(), "2026-07-01T11:35:00".to_string()),
            ]
        );
    }

    #[test]
    fn temp_dnd_reschedules_overdue_reminders_after_the_next_interval() {
        let conn = prepare_pause_test_db();
        conn.execute(
            "INSERT INTO reminders (id, interval_minutes, enabled, next_trigger, updated_at) VALUES ('due', 20, 1, '2026-07-01T09:59:00', '')",
            [],
        )
        .expect("test reminder should be inserted");
        let now = Utc
            .with_ymd_and_hms(2026, 7, 1, 10, 0, 0)
            .single()
            .expect("fixed test time should be valid");
        let resume_at = now + Duration::hours(1);

        shift_enabled_reminder_schedule_for_temp_dnd(&conn, 3600, now, resume_at)
            .expect("temporary dnd should reset overdue reminder");

        let next_trigger: String = conn
            .query_row(
                "SELECT next_trigger FROM reminders WHERE id = 'due'",
                [],
                |row| row.get(0),
            )
            .expect("reminder should exist");

        assert_eq!(next_trigger, "2026-07-01T11:20:00");
    }

    #[test]
    fn new_or_reenabled_schedule_uses_the_frozen_pause_clock() {
        let conn = prepare_pause_test_db();
        let start = NaiveDateTime::parse_from_str("2026-09-20T10:00:00", "%Y-%m-%dT%H:%M:%S")
            .unwrap()
            .and_utc();
        conn.execute("INSERT INTO settings VALUES ('all_reminders_paused', 'true'), ('all_reminders_paused_at', '2026-09-20T10:00:00')", []).unwrap();
        let next = build_next_trigger_from(schedule_base(&conn, start + Duration::minutes(15)), 20);
        conn.execute(
            "INSERT INTO reminders VALUES ('new', 20, 1, ?1, '')",
            [&next],
        )
        .unwrap();
        resume_all_reminders_at(&conn, start + Duration::minutes(30)).unwrap();
        let next: String = conn
            .query_row("SELECT next_trigger FROM reminders", [], |row| row.get(0))
            .unwrap();
        assert_eq!(next, "2026-09-20T10:50:00");
    }

    #[test]
    fn new_schedule_uses_the_earliest_active_pause_and_normal_clock_after_resume() {
        let conn = prepare_pause_test_db();
        let now = Utc::now();
        let manual = now - Duration::minutes(30);
        let system = now - Duration::minutes(20);
        conn.execute(
            "INSERT INTO settings VALUES ('all_reminders_paused', 'true')",
            [],
        )
        .unwrap();
        for (key, start) in [
            (ALL_REMINDERS_PAUSED_AT_KEY, manual),
            (SYSTEM_PAUSED_AT_KEY, system),
        ] {
            conn.execute(
                "INSERT INTO settings VALUES (?1, ?2)",
                (key, start.format("%Y-%m-%dT%H:%M:%S").to_string()),
            )
            .unwrap();
        }
        assert_eq!(schedule_base(&conn, now).timestamp(), manual.timestamp());
        resume_all_reminders_at(&conn, now).unwrap();
        assert_eq!(schedule_base(&conn, now).timestamp(), system.timestamp());
        compensate_system_pause(&conn, system, now).unwrap();
        assert_eq!(schedule_base(&conn, now), now);
    }

    #[test]
    fn temporary_dnd_overlap_is_compensated_once_for_manual_and_system_pauses() {
        let base = NaiveDateTime::parse_from_str("2026-09-20T10:00:00", "%Y-%m-%dT%H:%M:%S")
            .unwrap()
            .and_utc();
        for manual in [false, true] {
            for (dnd_start, dnd_end, pause_start, pause_end) in [
                (0, 30, 0, 30),
                (10, 40, 0, 30),
                (0, 30, 10, 40),
                (10, 20, 0, 40),
                (0, 40, 10, 20),
                (0, 10, 20, 30),
            ] {
                let conn = prepare_pause_test_db();
                conn.execute(
                    "INSERT INTO reminders VALUES ('test', 120, 1, '2026-09-20T12:00:00', '')",
                    [],
                )
                .unwrap();
                let mut events = vec![(dnd_start, 0), (pause_start, 1), (pause_end, 2)];
                events.sort();
                for (offset, kind) in events {
                    let at = base + Duration::minutes(offset);
                    match kind {
                        0 => start_temp_dnd_at(&conn, dnd_end - dnd_start, at).unwrap(),
                        1 => {
                            let key = if manual {
                                ALL_REMINDERS_PAUSED_AT_KEY
                            } else {
                                SYSTEM_PAUSED_AT_KEY
                            };
                            conn.execute(
                                "INSERT OR REPLACE INTO settings VALUES (?1, ?2)",
                                (key, at.format("%Y-%m-%dT%H:%M:%S").to_string()),
                            )
                            .unwrap();
                            if manual {
                                conn.execute("INSERT OR REPLACE INTO settings VALUES ('all_reminders_paused', 'true')", []).unwrap();
                            }
                        }
                        2 => {
                            if manual {
                                resume_all_reminders_at(&conn, at).unwrap();
                            } else {
                                compensate_system_pause(
                                    &conn,
                                    base + Duration::minutes(pause_start),
                                    at,
                                )
                                .unwrap();
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                let overlap = (dnd_end.min(pause_end) - dnd_start.max(pause_start)).max(0);
                let total = dnd_end - dnd_start + pause_end - pause_start - overlap;
                let next: String = conn
                    .query_row("SELECT next_trigger FROM reminders", [], |row| row.get(0))
                    .unwrap();
                assert_eq!(
                    next,
                    (base + Duration::minutes(120 + total))
                        .format("%Y-%m-%dT%H:%M:%S")
                        .to_string(),
                    "manual={manual}, dnd={dnd_start}..{dnd_end}, pause={pause_start}..{pause_end}"
                );
            }
        }
    }

    #[test]
    fn new_schedule_and_multiple_dnd_periods_during_manual_pause_keep_one_interval() {
        let conn = prepare_pause_test_db();
        let base = NaiveDateTime::parse_from_str("2026-09-20T10:00:00", "%Y-%m-%dT%H:%M:%S")
            .unwrap()
            .and_utc();
        conn.execute("INSERT INTO settings VALUES ('all_reminders_paused', 'true'), ('all_reminders_paused_at', '2026-09-20T10:00:00')", []).unwrap();
        start_temp_dnd_at(&conn, 10, base + Duration::minutes(5)).unwrap();
        start_temp_dnd_at(&conn, 10, base + Duration::minutes(20)).unwrap();
        start_temp_dnd_at(&conn, 10, base + Duration::minutes(25)).unwrap();
        let next = build_next_trigger_for_state_at(&conn, 20, base + Duration::minutes(30));
        conn.execute(
            "INSERT INTO reminders VALUES ('new', 20, 1, ?1, '')",
            [&next],
        )
        .unwrap();
        resume_all_reminders_at(&conn, base + Duration::minutes(40)).unwrap();
        let next: String = conn
            .query_row("SELECT next_trigger FROM reminders", [], |row| row.get(0))
            .unwrap();
        assert_eq!(next, "2026-09-20T11:00:00");
    }

    #[test]
    fn new_schedule_during_dnd_starts_its_interval_after_dnd() {
        let conn = prepare_pause_test_db();
        let base = Utc::now();
        start_temp_dnd_at(&conn, 30, base).unwrap();
        let next = build_next_trigger_for_state_at(&conn, 20, base + Duration::minutes(15));
        assert_eq!(
            next,
            (base + Duration::minutes(50))
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string()
        );
    }

    #[test]
    fn importing_a_paused_schedule_preserves_remaining_time() {
        let conn = prepare_pause_test_db();
        let now = Utc::now();
        let start = now - Duration::hours(1);
        let next = (start + Duration::minutes(5))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        conn.execute(
            "INSERT INTO reminders VALUES ('test', 20, 1, ?1, '')",
            [&next],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO settings VALUES ('all_reminders_paused', 'true')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO settings VALUES (?1, ?2)",
            (
                ALL_REMINDERS_PAUSED_AT_KEY,
                start.format("%Y-%m-%dT%H:%M:%S").to_string(),
            ),
        )
        .unwrap();
        repair_imported_reminder_schedule(&conn).unwrap();
        let preserved: String = conn
            .query_row("SELECT next_trigger FROM reminders", [], |row| row.get(0))
            .unwrap();
        assert_eq!(preserved, next);
        resume_all_reminders_at(&conn, now).unwrap();
        let next: String = conn
            .query_row("SELECT next_trigger FROM reminders", [], |row| row.get(0))
            .unwrap();
        let next = NaiveDateTime::parse_from_str(&next, "%Y-%m-%dT%H:%M:%S")
            .unwrap()
            .and_utc();
        assert!((next - now - Duration::minutes(5)).num_seconds().abs() <= 1);
    }

    #[test]
    fn missing_imported_schedule_is_repaired_against_the_pause_baseline() {
        let conn = prepare_pause_test_db();
        let now = Utc::now();
        let start = now - Duration::hours(1);
        conn.execute("INSERT INTO reminders VALUES ('test', 20, 1, NULL, '')", [])
            .unwrap();
        conn.execute(
            "INSERT INTO settings VALUES ('all_reminders_paused', 'true')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO settings VALUES (?1, ?2)",
            (
                ALL_REMINDERS_PAUSED_AT_KEY,
                start.format("%Y-%m-%dT%H:%M:%S").to_string(),
            ),
        )
        .unwrap();
        repair_imported_reminder_schedule(&conn).unwrap();
        resume_all_reminders_at(&conn, now).unwrap();
        let next: String = conn
            .query_row("SELECT next_trigger FROM reminders", [], |row| row.get(0))
            .unwrap();
        let next = NaiveDateTime::parse_from_str(&next, "%Y-%m-%dT%H:%M:%S")
            .unwrap()
            .and_utc();
        assert!((next - now - Duration::minutes(20)).num_seconds().abs() <= 1);
    }
}
