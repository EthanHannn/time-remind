//! Track built-in copy per field. An explicit empty list means user-owned copy;
//! NULL only occurs in legacy databases/backups and is inferred once.
use rusqlite::{Connection, OptionalExtension};

use crate::models::{Reminder, UpdateReminderRequest};

pub const LANGUAGES: &[&str] = &[
    "zh-CN", "zh-TW", "en-US", "ja-JP", "ko-KR", "fr-FR", "de-DE", "vi-VN", "th-TH", "ms-MY",
    "km-KH", "es-ES", "pt-BR", "id-ID", "it-IT", "ru-RU", "tr-TR", "ar", "hi-IN",
];
const FIELDS: [&str; 4] = ["name", "message", "action_title", "action_message"];

fn copy(id: &str, kind: &str, language: &str) -> Option<[&'static str; 4]> {
    let t = crate::reminder_template_text(language);
    match (id, kind) {
        ("default-drink", "drink") => Some([t.drink_name, t.drink_message, "", ""]),
        ("default-rest", "rest") => Some([
            t.rest_name,
            t.rest_message,
            t.rest_action_title,
            t.rest_action_message,
        ]),
        ("default-eye-care", "eye_care") => Some([
            t.eye_name,
            t.eye_message,
            t.eye_action_title,
            t.eye_action_message,
        ]),
        _ => None,
    }
}

fn values(reminder: &Reminder) -> [&str; 4] {
    [
        &reminder.name,
        &reminder.message,
        &reminder.action_title,
        &reminder.action_message,
    ]
}

fn legacy_copy(reminder: &Reminder, field: &str, value: &str) -> bool {
    // These strings were written by older action-field migrations.
    matches!(
        (
            reminder.id.as_str(),
            reminder.reminder_type.as_str(),
            field,
            value
        ),
        ("default-rest", "rest", "action_title", "休息中")
            | ("default-eye-care", "eye_care", "action_title", "护眼中")
            | (
                "default-eye-care",
                "eye_care",
                "action_message",
                "看向远处，让眼睛放松一下。"
            )
    )
}

pub fn fields(reminder: &Reminder) -> Vec<String> {
    let actual = values(reminder);
    FIELDS
        .iter()
        .enumerate()
        .filter(|(index, field)| {
            // Metadata cannot mark arbitrary user text as a built-in translation.
            reminder
                .template_fields
                .as_ref()
                .is_none_or(|fields| fields.iter().any(|f| f == **field))
                && !actual[*index].is_empty()
                && (LANGUAGES.iter().any(|language| {
                    copy(&reminder.id, &reminder.reminder_type, language)
                        .is_some_and(|text| text[*index] == actual[*index])
                }) || legacy_copy(reminder, field, actual[*index]))
        })
        .map(|(_, field)| (*field).to_string())
        .collect()
}

pub fn after_edit(current: &Reminder, request: &UpdateReminderRequest) -> Vec<String> {
    if request
        .reminder_type
        .as_ref()
        .is_some_and(|kind| kind != &current.reminder_type)
    {
        return vec![];
    }
    let next = [
        &request.name,
        &request.message,
        &request.action_title,
        &request.action_message,
    ];
    let actual = values(current);
    fields(current)
        .into_iter()
        .filter(|field| {
            let index = FIELDS.iter().position(|name| name == field).unwrap();
            next[index]
                .as_ref()
                .is_none_or(|value| value.trim() == actual[index])
        })
        .collect()
}

pub fn ensure_language(conn: &Connection, fallback: &str) -> rusqlite::Result<String> {
    let saved: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'language'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let language = saved
        .filter(|value| LANGUAGES.contains(&value.as_str()))
        .unwrap_or_else(|| crate::match_template_language(fallback).to_string());
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('language', ?1)",
        [&language],
    )?;
    Ok(language)
}

/// Caller owns the transaction. Only copy/metadata changes, never schedules or history.
pub fn sync(conn: &Connection, language: &str) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("SELECT id FROM reminders WHERE template_fields IS NULL OR id IN ('default-drink', 'default-rest', 'default-eye-care')")?;
    let ids = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    for id in ids {
        let reminder = crate::commands::get_reminder_by_id(conn, &id)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e))?;
        let fields = fields(&reminder);
        let mut text = values(&reminder);
        if let Some(translated) = copy(&id, &reminder.reminder_type, language) {
            for (index, field) in FIELDS.iter().enumerate() {
                if fields.iter().any(|f| f == field) {
                    text[index] = translated[index];
                }
            }
        }
        if text == values(&reminder) && reminder.template_fields.as_ref() == Some(&fields) {
            continue;
        }
        conn.execute("UPDATE reminders SET name = ?1, message = ?2, action_title = ?3, action_message = ?4, template_fields = ?5 WHERE id = ?6",
            (text[0], text[1], text[2], text[3], serde_json::to_string(&fields).unwrap(), id))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn change_language(conn: &mut Connection, language: &str) {
        crate::settings::save_batch(conn, &HashMap::from([("language".into(), language.into())]))
            .unwrap();
    }

    #[test]
    fn startup_persists_one_language_and_keeps_saved_preferences_on_upgrade() {
        let db = crate::db::Database::in_memory();
        let mut conn = db.conn.lock().unwrap();
        crate::initialize_default_reminders(&mut conn, "ja-JP").unwrap();
        assert_eq!(ensure_language(&conn, "en-US").unwrap(), "ja-JP");
        crate::initialize_default_reminders(&mut conn, "zh-CN").unwrap();
        assert_eq!(ensure_language(&conn, "en-US").unwrap(), "ja-JP");
        assert_eq!(
            crate::commands::get_reminder_by_id(&conn, "default-rest")
                .unwrap()
                .name,
            crate::reminder_template_text("ja-JP").rest_name
        );
        conn.execute("DELETE FROM reminders", []).unwrap();
        conn.execute("DELETE FROM settings WHERE key = 'language'", [])
            .unwrap();
        crate::initialize_default_reminders(&mut conn, "pt-BR").unwrap();
        assert_eq!(ensure_language(&conn, "en-US").unwrap(), "pt-BR");
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM reminders", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(crate::match_template_language("zh_TW.UTF-8"), "zh-TW");
        assert_eq!(crate::match_template_language("de_DE.UTF-8@euro"), "de-DE");
    }

    #[test]
    fn language_changes_translate_builtins_without_resetting_the_schedule_or_custom_copy() {
        let db = crate::db::Database::in_memory();
        let mut conn = db.conn.lock().unwrap();
        crate::initialize_default_reminders(&mut conn, "zh-CN").unwrap();
        let before = crate::commands::get_reminder_by_id(&conn, "default-rest").unwrap();
        let request: UpdateReminderRequest = serde_json::from_value(
            serde_json::json!({"name": "My stretch", "action_message": "My routine"}),
        )
        .unwrap();
        let retained = after_edit(&before, &request);
        conn.execute("UPDATE reminders SET name = 'My stretch', action_message = 'My routine', template_fields = ?1, enabled = 0 WHERE id = 'default-rest'", [serde_json::to_string(&retained).unwrap()]).unwrap();
        for language in LANGUAGES {
            change_language(&mut conn, language);
            let reminder = crate::commands::get_reminder_by_id(&conn, "default-rest").unwrap();
            let text = crate::reminder_template_text(language);
            assert_eq!(reminder.name, "My stretch");
            assert_eq!(reminder.action_message, "My routine");
            assert_eq!(reminder.message, text.rest_message);
            assert_eq!(reminder.action_title, text.rest_action_title);
            assert_eq!(reminder.next_trigger, before.next_trigger);
            assert_eq!(reminder.updated_at, before.updated_at);
            assert!(!reminder.enabled);
        }
        let current = crate::commands::get_reminder_by_id(&conn, "default-rest").unwrap();
        let unchanged: UpdateReminderRequest = serde_json::from_value(
            serde_json::json!({"message": current.message, "interval_minutes": 10}),
        )
        .unwrap();
        assert_eq!(after_edit(&current, &unchanged), retained);
        let changed_type: UpdateReminderRequest =
            serde_json::from_value(serde_json::json!({"reminder_type": "custom"})).unwrap();
        assert!(after_edit(&current, &changed_type).is_empty());
    }

    #[test]
    fn legacy_metadata_is_inferred_once_and_explicit_user_ownership_survives_restart() {
        let db = crate::db::Database::in_memory();
        let mut conn = db.conn.lock().unwrap();
        crate::initialize_default_reminders(&mut conn, "zh-CN").unwrap();
        conn.execute("UPDATE reminders SET template_fields = NULL, action_title = '护眼中', message = 'My message' WHERE id = 'default-eye-care'", []).unwrap();
        change_language(&mut conn, "en-US");
        let reminder = crate::commands::get_reminder_by_id(&conn, "default-eye-care").unwrap();
        assert_eq!(reminder.message, "My message");
        assert_eq!(
            reminder.action_title,
            crate::reminder_template_text("en-US").eye_action_title
        );
        // Even copy identical to a known template stays untouched once explicitly user-owned.
        conn.execute(
            "UPDATE reminders SET template_fields = '[]' WHERE id = 'default-eye-care'",
            [],
        )
        .unwrap();
        change_language(&mut conn, "ar");
        crate::initialize_default_reminders(&mut conn, "zh-CN").unwrap();
        let after = crate::commands::get_reminder_by_id(&conn, "default-eye-care").unwrap();
        assert_eq!(after.name, reminder.name);
        assert_eq!(after.action_title, reminder.action_title);
        assert_eq!(after.template_fields, Some(vec![]));
    }

    #[test]
    fn language_and_translations_roll_back_together_if_a_write_fails() {
        let db = crate::db::Database::in_memory();
        let mut conn = db.conn.lock().unwrap();
        crate::initialize_default_reminders(&mut conn, "zh-CN").unwrap();
        conn.execute_batch("CREATE TRIGGER fail_translation BEFORE UPDATE ON reminders BEGIN SELECT RAISE(ABORT, 'disk full'); END;").unwrap();
        assert!(crate::settings::save_batch(
            &mut conn,
            &HashMap::from([("language".into(), "en-US".into())])
        )
        .is_err());
        assert_eq!(ensure_language(&conn, "en-US").unwrap(), "zh-CN");
        assert_eq!(
            crate::commands::get_reminder_by_id(&conn, "default-rest")
                .unwrap()
                .name,
            crate::reminder_template_text("zh-CN").rest_name
        );
    }
}
