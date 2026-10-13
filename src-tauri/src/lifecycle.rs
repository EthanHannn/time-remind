use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::models::Reminder;

pub const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS active_notifications (
    notification_id TEXT PRIMARY KEY, reminder_id TEXT NOT NULL UNIQUE,
    triggered_at TEXT NOT NULL, started_at TEXT, deadline TEXT
);";

fn stamp(time: DateTime<Utc>) -> String {
    time.format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[derive(Debug, Serialize)]
pub struct ActionProgress {
    pub remaining_seconds: i64,
}

pub fn shown(
    conn: &Connection,
    reminder_id: &str,
    notification_id: &str,
    now: DateTime<Utc>,
) -> Result<(), String> {
    conn.execute("INSERT INTO active_notifications (notification_id, reminder_id, triggered_at) VALUES (?1, ?2, ?3)",
        (notification_id, reminder_id, stamp(now))).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn progress(
    conn: &Connection,
    notification_id: &str,
    now: DateTime<Utc>,
) -> Result<Option<ActionProgress>, String> {
    let deadline: Option<String> = conn
        .query_row(
            "SELECT deadline FROM active_notifications WHERE notification_id = ?1",
            [notification_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    deadline
        .map(|deadline| {
            let millis = (crate::settings::timestamp(&deadline)? - now).num_milliseconds();
            Ok(ActionProgress {
                remaining_seconds: ((millis + 999) / 1000).max(0),
            })
        })
        .transpose()
}

pub fn respond(
    conn: &Connection,
    reminder: &Reminder,
    notification_id: &str,
    action: &str,
    postpone_minutes: Option<i64>,
    now: DateTime<Utc>,
) -> Result<Option<ActionProgress>, String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let (triggered, started): (String, Option<String>) = tx.query_row(
        "SELECT triggered_at, started_at FROM active_notifications WHERE notification_id = ?1 AND reminder_id = ?2",
        (notification_id, &reminder.id), |r| Ok((r.get(0)?, r.get(1)?))).map_err(|_| crate::errors::code("unavailable"))?;
    if !reminder.enabled {
        return Err(crate::errors::code("unavailable"));
    }
    if action == "completed" && reminder.action_enabled && reminder.action_duration_seconds > 0 {
        if started.is_none() {
            tx.execute("UPDATE active_notifications SET started_at = ?1, deadline = ?2 WHERE notification_id = ?3",
                (stamp(now), stamp(now + Duration::seconds(reminder.action_duration_seconds)), notification_id)).map_err(|e| e.to_string())?;
        }
        let result = progress(&tx, notification_id, now)?;
        tx.commit().map_err(|e| e.to_string())?;
        return Ok(result);
    }
    if started.is_some() {
        return Err(crate::errors::code("actionRunning"));
    }
    let minutes = postpone_minutes.unwrap_or(reminder.interval_minutes);
    record_outcome(
        &tx,
        reminder,
        notification_id,
        action,
        &triggered,
        now,
        minutes,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(None)
}

fn record_outcome(
    conn: &Connection,
    reminder: &Reminder,
    id: &str,
    action: &str,
    triggered: &str,
    now: DateTime<Utc>,
    interval: i64,
) -> Result<(), String> {
    conn.execute("INSERT INTO reminder_logs (id, reminder_id, action, triggered_at, responded_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        (id, &reminder.id, action, triggered, stamp(now))).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE reminders SET next_trigger = ?1, updated_at = ?2 WHERE id = ?3",
        (
            reminder
                .enabled
                .then(|| stamp(now + Duration::minutes(interval))),
            stamp(now),
            &reminder.id,
        ),
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM active_notifications WHERE notification_id = ?1",
        [id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn finish(
    conn: &Connection,
    reminder: &Reminder,
    id: &str,
    manual: bool,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let (triggered, deadline): (String, String) = tx.query_row(
        "SELECT triggered_at, deadline FROM active_notifications WHERE notification_id = ?1 AND reminder_id = ?2 AND started_at IS NOT NULL",
        (id, &reminder.id), |r| Ok((r.get(0)?, r.get(1)?))).map_err(|_| crate::errors::code("unavailable"))?;
    if !manual && crate::settings::timestamp(&deadline)? > now {
        return Err(crate::errors::code("actionPending"));
    }
    record_outcome(
        &tx,
        reminder,
        id,
        "completed",
        &triggered,
        now,
        reminder.interval_minutes,
    )?;
    tx.commit().map_err(|e| e.to_string())
}

// Manual pause, disable, import and restart interrupt an action; they never complete it.
pub fn interrupt(
    conn: &Connection,
    reminder_id: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    interrupt_in_transaction(&tx, reminder_id, now)?;
    tx.commit().map_err(|e| e.to_string())
}

pub fn interrupt_in_transaction(
    tx: &Connection,
    reminder_id: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let mut stmt = tx.prepare("SELECT notification_id, reminder_id, triggered_at FROM active_notifications WHERE started_at IS NOT NULL AND (?1 IS NULL OR reminder_id = ?1)").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([reminder_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);
    for (id, reminder, triggered) in rows {
        tx.execute("INSERT OR IGNORE INTO reminder_logs (id, reminder_id, action, triggered_at, responded_at) VALUES (?1, ?2, 'interrupted', ?3, ?4)",
            (&id, &reminder, triggered, stamp(now))).map_err(|e| e.to_string())?;
        let interval = tx
            .query_row(
                "SELECT interval_minutes FROM reminders WHERE id = ?1 AND enabled = 1",
                [&reminder],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(interval) = interval {
            tx.execute(
                "UPDATE reminders SET next_trigger = ?1 WHERE id = ?2",
                (
                    crate::commands::build_next_trigger_for_state_at(tx, interval, now),
                    &reminder,
                ),
            )
            .map_err(|e| e.to_string())?;
        }
    }
    tx.execute(
        "DELETE FROM active_notifications WHERE ?1 IS NULL OR reminder_id = ?1",
        [reminder_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn extend_deadlines(conn: &Connection, seconds: i64) -> Result<(), String> {
    conn.execute("UPDATE active_notifications SET deadline = strftime('%Y-%m-%dT%H:%M:%S', deadline, ?1 || ' seconds') WHERE deadline IS NOT NULL", [seconds.max(0)]).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup() -> (Database, Reminder, DateTime<Utc>) {
        let db = Database::in_memory();
        let now = crate::settings::timestamp("2026-10-10T10:00:00").unwrap();
        db.conn.lock().unwrap().execute("INSERT INTO reminders (id, name, message, enabled, interval_minutes, action_enabled, action_duration_seconds, created_at, updated_at) VALUES ('one','Rest','Rest',1,60,1,20,?1,?1)", [stamp(now)]).unwrap();
        let reminder =
            crate::commands::get_reminder_by_id(&db.conn.lock().unwrap(), "one").unwrap();
        (db, reminder, now)
    }

    #[test]
    fn start_does_not_complete_and_finish_is_atomic_and_idempotent() {
        let (db, reminder, now) = setup();
        let conn = db.conn.lock().unwrap();
        shown(&conn, "one", "instance", now).unwrap();
        respond(
            &conn,
            &reminder,
            "instance",
            "completed",
            None,
            now + Duration::seconds(3),
        )
        .unwrap();
        respond(
            &conn,
            &reminder,
            "instance",
            "completed",
            None,
            now + Duration::seconds(4),
        )
        .unwrap();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM reminder_logs", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(finish(
            &conn,
            &reminder,
            "instance",
            false,
            now + Duration::seconds(5)
        )
        .is_err());
        finish(
            &conn,
            &reminder,
            "instance",
            false,
            now + Duration::seconds(23),
        )
        .unwrap();
        assert!(finish(
            &conn,
            &reminder,
            "instance",
            true,
            now + Duration::seconds(24)
        )
        .is_err());
        let log: (String, String, String) = conn
            .query_row(
                "SELECT action, triggered_at, responded_at FROM reminder_logs",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            log,
            (
                "completed".into(),
                stamp(now),
                stamp(now + Duration::seconds(23))
            )
        );
        assert_eq!(
            conn.query_row("SELECT next_trigger FROM reminders", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "2026-10-10T11:00:23"
        );
    }

    #[test]
    fn interruptions_and_system_pause_do_not_fabricate_completions() {
        let (db, reminder, now) = setup();
        let conn = db.conn.lock().unwrap();
        shown(&conn, "one", "instance", now).unwrap();
        respond(&conn, &reminder, "instance", "completed", None, now).unwrap();
        extend_deadlines(&conn, 3600).unwrap();
        assert!(finish(
            &conn,
            &reminder,
            "instance",
            false,
            now + Duration::seconds(20)
        )
        .is_err());
        interrupt(&conn, None, now + Duration::seconds(10)).unwrap();
        interrupt(&conn, None, now + Duration::seconds(11)).unwrap();
        assert_eq!(
            conn.query_row("SELECT action FROM reminder_logs", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "interrupted"
        );
        assert!(respond(&conn, &reminder, "instance", "completed", None, now).is_err());
        shown(&conn, "one", "new-instance", now + Duration::minutes(61)).unwrap();
        respond(
            &conn,
            &reminder,
            "new-instance",
            "completed",
            None,
            now + Duration::minutes(61),
        )
        .unwrap();
        finish(
            &conn,
            &reminder,
            "new-instance",
            true,
            now + Duration::minutes(61),
        )
        .unwrap();
    }

    #[test]
    fn failed_log_write_keeps_action_and_schedule_available_for_retry() {
        let (db, reminder, now) = setup();
        let conn = db.conn.lock().unwrap();
        shown(&conn, "one", "instance", now).unwrap();
        respond(&conn, &reminder, "instance", "completed", None, now).unwrap();
        conn.execute_batch("CREATE TRIGGER fail_log BEFORE INSERT ON reminder_logs BEGIN SELECT RAISE(ABORT, 'disk full'); END;").unwrap();
        assert!(finish(&conn, &reminder, "instance", true, now).is_err());
        assert!(progress(&conn, "instance", now).unwrap().is_some());
        conn.execute_batch("DROP TRIGGER fail_log").unwrap();
        finish(&conn, &reminder, "instance", true, now).unwrap();
    }
}
