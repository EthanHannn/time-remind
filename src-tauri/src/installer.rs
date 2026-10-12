//! Consume the Windows installer's explicit startup choice before restoring
//! autostart or deciding whether an --autostart launch should show a window.
use std::io::Read;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;

const APPLIED_REQUEST_KEY: &str = "installer_startup_request";
const MAX_OPTIONS_BYTES: u64 = 4096;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartupOptions {
    schema: u8,
    request_id: String,
    auto_start: bool,
    silent_start: bool,
}

fn apply_options(conn: &mut Connection, content: &str) -> Result<bool, String> {
    let options: StartupOptions = serde_json::from_str(content).map_err(|e| e.to_string())?;
    if options.schema != 1 {
        return Err("不支持的安装器启动设置格式".into());
    }
    let request_id = uuid::Uuid::parse_str(&options.request_id)
        .map_err(|e| e.to_string())?
        .to_string();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let previous: Option<String> = tx
        .query_row(
            "SELECT value FROM app_metadata WHERE key = ?1",
            [APPLIED_REQUEST_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if previous.as_deref() == Some(&request_id) {
        return Ok(false);
    }
    for (key, enabled) in [
        ("auto_start", options.auto_start),
        ("silent_start", options.auto_start && options.silent_start),
    ] {
        tx.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            (key, enabled.to_string()),
        )
        .map_err(|e| e.to_string())?;
    }
    tx.execute(
        "INSERT OR REPLACE INTO app_metadata (key, value) VALUES (?1, ?2)",
        (APPLIED_REQUEST_KEY, request_id),
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}

fn consume_options(conn: &mut Connection, path: &Path) -> Result<bool, String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.to_string()),
    };
    let mut content = String::new();
    file.take(MAX_OPTIONS_BYTES + 1)
        .read_to_string(&mut content)
        .map_err(|e| e.to_string())?;
    if content.len() as u64 > MAX_OPTIONS_BYTES {
        return Err("安装器启动设置文件过大".into());
    }
    let applied = apply_options(conn, &content)?;
    if let Err(error) = std::fs::remove_file(path) {
        // The committed request ID prevents this marker from overriding a
        // later in-app choice even if removal fails.
        crate::app_log::warn(format!("清理安装器启动设置失败：{error}"));
    }
    Ok(applied)
}

#[cfg(target_os = "windows")]
pub fn apply_startup_options(app: &tauri::AppHandle, db: &crate::db::Database) {
    use tauri::Manager;
    let result = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())
        .and_then(|dir| {
            consume_options(
                &mut db.conn.lock().unwrap(),
                &dir.join("installer-startup.json"),
            )
        });
    match result {
        Ok(true) => crate::app_log::info("已应用安装器选择的启动设置"),
        Ok(false) => {}
        Err(error) => crate::app_log::warn(format!("读取安装器启动设置失败：{error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn options(auto_start: bool, silent_start: bool) -> String {
        serde_json::json!({
            "schema": 1,
            "request_id": format!("{{{}}}", uuid::Uuid::new_v4()),
            "auto_start": auto_start,
            "silent_start": silent_start,
        })
        .to_string()
    }

    fn values(conn: &Connection) -> (String, String) {
        conn.query_row(
            "SELECT a.value, s.value FROM settings a, settings s WHERE a.key = 'auto_start' AND s.key = 'silent_start'",
            [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap()
    }

    #[test]
    fn explicit_installer_choices_replace_old_values_and_gate_silent_start() {
        let db = Database::in_memory();
        let mut conn = db.conn.lock().unwrap();
        for (auto, silent) in [(true, false), (true, true), (false, true), (false, false)] {
            assert!(apply_options(&mut conn, &options(auto, silent)).unwrap());
            assert_eq!(
                values(&conn),
                (auto.to_string(), (auto && silent).to_string())
            );
        }
    }

    #[test]
    fn a_leftover_marker_never_reverts_later_user_changes() {
        let db = Database::in_memory();
        let mut conn = db.conn.lock().unwrap();
        let first_install = options(true, true);
        assert!(apply_options(&mut conn, &first_install).unwrap());
        conn.execute("UPDATE settings SET value = 'false'", [])
            .unwrap();
        assert!(!apply_options(&mut conn, &first_install).unwrap());
        assert_eq!(values(&conn), ("false".into(), "false".into()));
        assert!(apply_options(&mut conn, &options(true, true)).unwrap());
        assert_eq!(values(&conn), ("true".into(), "true".into()));
    }

    #[test]
    fn invalid_options_do_not_change_preferences() {
        let db = Database::in_memory();
        let mut conn = db.conn.lock().unwrap();
        apply_options(&mut conn, &options(false, false)).unwrap();
        for invalid in [
            "{}".to_string(),
            options(true, true).replace("\"schema\":1", "\"schema\":2"),
            options(true, true).replace("\"auto_start\":true", "\"auto_start\":\"true\""),
            r#"{"schema":1,"request_id":"invalid","auto_start":true,"silent_start":true}"#.into(),
        ] {
            assert!(apply_options(&mut conn, &invalid).is_err());
            assert_eq!(values(&conn), ("false".into(), "false".into()));
        }
    }

    #[test]
    fn failed_settings_write_rolls_back_values_and_request_id() {
        let db = Database::in_memory();
        let mut conn = db.conn.lock().unwrap();
        apply_options(&mut conn, &options(false, false)).unwrap();
        conn.execute_batch("CREATE TRIGGER reject_silent BEFORE INSERT ON settings WHEN NEW.key = 'silent_start' BEGIN SELECT RAISE(ABORT, 'disk full'); END;").unwrap();
        let choice = options(true, true);
        assert!(apply_options(&mut conn, &choice).is_err());
        assert_eq!(values(&conn), ("false".into(), "false".into()));
        conn.execute_batch("DROP TRIGGER reject_silent;").unwrap();
        assert!(apply_options(&mut conn, &choice).unwrap());
        assert_eq!(values(&conn), ("true".into(), "true".into()));
    }

    #[test]
    fn no_marker_preserves_existing_settings_and_consumed_marker_is_removed() {
        let db = Database::in_memory();
        let mut conn = db.conn.lock().unwrap();
        apply_options(&mut conn, &options(true, true)).unwrap();
        let dir =
            std::env::temp_dir().join(format!("time-remind-installer-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let marker = dir.join("installer-startup.json");
        assert!(!consume_options(&mut conn, &marker).unwrap());
        assert_eq!(values(&conn), ("true".into(), "true".into()));
        std::fs::write(&marker, options(false, false)).unwrap();
        assert!(consume_options(&mut conn, &marker).unwrap());
        assert!(!marker.exists());
        assert_eq!(values(&conn), ("false".into(), "false".into()));
        std::fs::write(&marker, " ".repeat(MAX_OPTIONS_BYTES as usize + 1)).unwrap();
        assert!(consume_options(&mut conn, &marker).is_err());
        assert!(marker.exists());
        std::fs::remove_file(marker).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
