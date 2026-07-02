use rusqlite::{Connection, Result};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri::Manager;
use uuid::Uuid;

/// 数据库包装器
pub struct Database {
    pub conn: Mutex<Connection>,
    #[allow(dead_code)]
    db_path: PathBuf,
}

impl Database {
    /// 初始化数据库
    pub fn init(app: &AppHandle) -> Result<Self> {
        let db_path = Self::get_db_path(app)?;

        // 尝试打开数据库，如果损坏则重建
        let conn = match Self::try_open(&db_path) {
            Ok(conn) => conn,
            Err(_) => {
                // 数据库可能损坏，备份后重建
                match Self::backup_corrupted(&db_path) {
                    Ok(Some(backup_path)) => {
                        crate::app_log::warn(format!(
                            "数据库打开失败，已备份损坏文件：{}",
                            backup_path.display()
                        ));
                    }
                    Ok(None) => {
                        crate::app_log::warn("数据库打开失败，未发现可备份文件");
                    }
                    Err(error) => {
                        crate::app_log::error(format!("数据库损坏文件备份失败：{error}"));
                    }
                }
                Connection::open(&db_path)?
            }
        };

        let db = Self {
            conn: Mutex::new(conn),
            db_path,
        };
        db.migrate()?;
        Ok(db)
    }

    /// 尝试打开并验证数据库
    fn try_open(db_path: &PathBuf) -> Result<Connection> {
        let conn = Connection::open(db_path)?;

        // 执行完整性检查
        let integrity_check: String =
            conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;

        if integrity_check != "ok" {
            return Err(rusqlite::Error::InvalidParameterName(
                "Database integrity check failed".to_string(),
            ));
        }

        Ok(conn)
    }

    fn corrupted_backup_path(db_path: &Path) -> PathBuf {
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let suffix = Uuid::new_v4().simple().to_string();
        let file_name = db_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("time-remind.db");

        db_path.with_file_name(format!("{file_name}.corrupted.{timestamp}.{suffix}"))
    }

    /// 备份损坏的数据库
    fn backup_corrupted(db_path: &Path) -> std::io::Result<Option<PathBuf>> {
        if db_path.exists() {
            let backup_path = Self::corrupted_backup_path(db_path);
            std::fs::rename(db_path, &backup_path)?;
            return Ok(Some(backup_path));
        }

        Ok(None)
    }

    /// 获取数据库文件路径
    fn get_db_path(app: &AppHandle) -> Result<PathBuf> {
        let data_dir = app
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");
        std::fs::create_dir_all(&data_dir).expect("failed to create data dir");
        Ok(data_dir.join("time-remind.db"))
    }

    /// 执行数据库迁移
    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS reminders (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                reminder_type TEXT NOT NULL DEFAULT 'custom',
                icon TEXT NOT NULL DEFAULT 'custom',
                message TEXT NOT NULL DEFAULT '',
                interval_minutes INTEGER NOT NULL DEFAULT 60,
                break_duration_minutes INTEGER NOT NULL DEFAULT 0,
                break_notification_enabled INTEGER NOT NULL DEFAULT 0,
                action_enabled INTEGER NOT NULL DEFAULT 0,
                action_title TEXT NOT NULL DEFAULT '',
                action_message TEXT NOT NULL DEFAULT '',
                action_duration_seconds INTEGER NOT NULL DEFAULT 0,
                action_completion_mode TEXT NOT NULL DEFAULT 'auto',
                enabled INTEGER NOT NULL DEFAULT 1,
                next_trigger TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS reminder_logs (
                id TEXT PRIMARY KEY,
                reminder_id TEXT NOT NULL,
                action TEXT NOT NULL,
                triggered_at TEXT NOT NULL,
                responded_at TEXT,
                FOREIGN KEY (reminder_id) REFERENCES reminders(id)
            );

            CREATE INDEX IF NOT EXISTS idx_reminder_logs_reminder_id
                ON reminder_logs(reminder_id);

            CREATE INDEX IF NOT EXISTS idx_reminder_logs_triggered_at
                ON reminder_logs(triggered_at);

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;

        let mut stmt = conn.prepare("PRAGMA table_info(reminders)")?;
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;

        if !columns
            .iter()
            .any(|column| column == "break_duration_minutes")
        {
            conn.execute(
                "ALTER TABLE reminders ADD COLUMN break_duration_minutes INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }

        if !columns
            .iter()
            .any(|column| column == "break_notification_enabled")
        {
            conn.execute(
                "ALTER TABLE reminders ADD COLUMN break_notification_enabled INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }

        if !columns
            .iter()
            .any(|column| column == "action_duration_seconds")
        {
            conn.execute(
                "ALTER TABLE reminders ADD COLUMN action_duration_seconds INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }

        if !columns.iter().any(|column| column == "action_enabled") {
            conn.execute(
                "ALTER TABLE reminders ADD COLUMN action_enabled INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }

        if !columns.iter().any(|column| column == "action_title") {
            conn.execute(
                "ALTER TABLE reminders ADD COLUMN action_title TEXT NOT NULL DEFAULT ''",
                [],
            )?;
        }

        if !columns.iter().any(|column| column == "action_message") {
            conn.execute(
                "ALTER TABLE reminders ADD COLUMN action_message TEXT NOT NULL DEFAULT ''",
                [],
            )?;
        }

        if !columns
            .iter()
            .any(|column| column == "action_completion_mode")
        {
            conn.execute(
                "ALTER TABLE reminders ADD COLUMN action_completion_mode TEXT NOT NULL DEFAULT 'auto'",
                [],
            )?;
        }

        conn.execute(
            "UPDATE reminders
             SET action_duration_seconds = break_duration_minutes * 60
             WHERE reminder_type = 'rest'
               AND break_notification_enabled = 1
               AND break_duration_minutes > 0
               AND action_duration_seconds = 0",
            [],
        )?;

        conn.execute(
            "UPDATE reminders
             SET action_duration_seconds = 20
             WHERE reminder_type = 'eye_care'
               AND action_duration_seconds = 0",
            [],
        )?;

        conn.execute(
            "UPDATE reminders
             SET action_enabled = 1,
                 action_title = CASE WHEN action_title = '' THEN '休息中' ELSE action_title END,
                 action_message = CASE WHEN action_message = '' THEN '倒计时结束后再进入下一轮。' ELSE action_message END,
                 action_completion_mode = CASE WHEN action_completion_mode = '' THEN 'auto' ELSE action_completion_mode END
             WHERE reminder_type = 'rest'
               AND break_notification_enabled = 1
               AND action_duration_seconds > 0",
            [],
        )?;

        conn.execute(
            "UPDATE reminders
             SET action_enabled = 1,
                 action_title = CASE WHEN action_title = '' THEN '护眼中' ELSE action_title END,
                 action_message = CASE WHEN action_message = '' THEN '看向远处，让眼睛放松一下。' ELSE action_message END,
                 action_completion_mode = CASE WHEN action_completion_mode = '' THEN 'auto' ELSE action_completion_mode END
             WHERE reminder_type = 'eye_care'
               AND action_duration_seconds > 0",
            [],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("time-remind-db-test-{}", Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).expect("test dir should be created");
        dir.join(name)
    }

    #[test]
    fn backup_corrupted_preserves_database_content() {
        let db_path = temp_db_path("time-remind.db");
        std::fs::write(&db_path, b"broken database").expect("test database should be written");

        let backup_path = Database::backup_corrupted(&db_path)
            .expect("backup should succeed")
            .expect("backup path should exist");

        assert!(!db_path.exists());
        assert!(backup_path.exists());
        assert_eq!(
            std::fs::read(&backup_path).expect("backup should be readable"),
            b"broken database"
        );
        assert!(backup_path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.starts_with("time-remind.db.corrupted.")));

        let _ = std::fs::remove_file(backup_path);
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }

    #[test]
    fn backup_corrupted_ignores_missing_database() {
        let db_path = temp_db_path("missing.db");

        let backup_path = Database::backup_corrupted(&db_path).expect("backup should not fail");

        assert!(backup_path.is_none());
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }
}
