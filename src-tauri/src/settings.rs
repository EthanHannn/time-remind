use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDateTime, NaiveTime, Utc};
use rusqlite::Connection;

pub fn timestamp(value: &str) -> Result<DateTime<Utc>, String> {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .map(|date| date.and_utc())
        .or_else(|_| DateTime::parse_from_rfc3339(value).map(|date| date.with_timezone(&Utc)))
        .map_err(|_| format!("无效的日期时间: {value}"))
}

pub fn is_internal(key: &str) -> bool {
    matches!(
        key,
        "all_reminders_paused"
            | "all_reminders_paused_at"
            | "system_paused_at"
            | "temp_dnd_until"
            | "temp_dnd_compensated_periods"
            | "last_boot_marker"
    )
}

pub fn validate(key: &str, value: &str) -> Result<(), String> {
    let integer = |min, max| value.parse::<i64>().is_ok_and(|n| (min..=max).contains(&n));
    let valid = match key {
        "theme" => matches!(value, "light" | "dark" | "system"),
        "mascot_style" => matches!(value, "classic" | "editorial" | "watercolor"),
        "notification_appearance" => {
            matches!(value, "soft" | "classic" | "minimal" | "illustrated")
        }
        "language" => matches!(
            value,
            "zh-CN"
                | "zh-TW"
                | "en-US"
                | "ja-JP"
                | "ko-KR"
                | "fr-FR"
                | "de-DE"
                | "vi-VN"
                | "th-TH"
                | "ms-MY"
                | "km-KH"
                | "es-ES"
                | "pt-BR"
                | "id-ID"
                | "it-IT"
                | "ru-RU"
                | "tr-TR"
                | "ar"
                | "hi-IN"
        ),
        "sound_enabled"
        | "dnd_enabled"
        | "auto_start"
        | "silent_start"
        | "fullscreen_detection_enabled"
        | "all_reminders_paused" => matches!(value, "true" | "false"),
        "notification_duration" => integer(5, 120),
        "sound_volume" => integer(0, 100),
        "sound_preset" => matches!(value, "soft" | "bright" | "calm" | "anime" | "arcade"),
        "postpone_options" => serde_json::from_str::<Vec<i64>>(value).is_ok_and(|values| {
            !values.is_empty()
                && values.len() <= 8
                && values.iter().all(|v| (1..=1440).contains(v))
                && values.iter().collect::<HashSet<_>>().len() == values.len()
        }),
        "dnd_start" | "dnd_end" => {
            value.len() == 5 && NaiveTime::parse_from_str(value, "%H:%M").is_ok()
        }
        "all_reminders_paused_at" | "system_paused_at" | "temp_dnd_until" => {
            timestamp(value).is_ok()
        }
        "last_boot_marker" => !value.is_empty() && value.len() <= 128,
        "temp_dnd_compensated_periods" => {
            serde_json::from_str::<Vec<Period>>(value).is_ok_and(|periods| {
                periods.len() <= 10_000
                    && periods.iter().all(|p| {
                        p.start <= p.end
                            && DateTime::<Utc>::from_timestamp(p.start, 0).is_some()
                            && DateTime::<Utc>::from_timestamp(p.end, 0).is_some()
                    })
            })
        }
        _ => false,
    };
    if valid && value.len() <= 1024 * 1024 {
        Ok(())
    } else {
        Err(format!("无效的设置 {key}: 请检查类型或取值范围"))
    }
}

#[derive(serde::Deserialize)]
struct Period {
    start: i64,
    end: i64,
}

pub fn save_batch(conn: &mut Connection, settings: &HashMap<String, String>) -> Result<(), String> {
    for (key, value) in settings {
        if is_internal(key) {
            return Err(format!("不能直接修改运行状态: {key}"));
        }
        validate(key, value)?;
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for (key, value) in settings {
        tx.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            (key, value),
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn reconcile_autostart(
    desired: bool,
    read: impl Fn() -> Result<bool, String>,
    apply: impl Fn(bool) -> Result<(), String>,
) -> Result<(), String> {
    if read()? != desired {
        apply(desired)?;
    }
    if read()? != desired {
        return Err("系统自启状态与导入设置不一致".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_autostart_applies_both_on_and_off_and_reports_failures() {
        let actual = std::cell::Cell::new(true);
        for desired in [false, true] {
            reconcile_autostart(
                desired,
                || Ok(actual.get()),
                |enabled| {
                    actual.set(enabled);
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(actual.get(), desired);
        }
        assert!(reconcile_autostart(false, || Ok(true), |_| Err("denied".into())).is_err());
    }

    #[test]
    fn accepts_current_and_legacy_notification_layouts_for_settings_and_backups() {
        for appearance in ["soft", "classic", "minimal", "illustrated"] {
            assert!(validate("notification_appearance", appearance).is_ok());
        }
        assert!(validate("notification_appearance", "unknown").is_err());
    }

    #[test]
    fn rejects_invalid_settings_without_partial_writes() {
        let db = crate::db::Database::in_memory();
        let mut conn = db.conn.lock().unwrap();
        for (key, value) in [
            ("postpone_options", "not json"),
            ("notification_duration", "NaN"),
            ("postpone_options", "[0,5]"),
            ("dnd_start", "25:00"),
            ("sound_enabled", "1"),
            ("theme", "unknown"),
        ] {
            assert!(save_batch(
                &mut conn,
                &HashMap::from([("theme".into(), "dark".into()), (key.into(), value.into())])
            )
            .is_err());
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM settings", [], |row| row
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
        save_batch(
            &mut conn,
            &HashMap::from([
                ("sound_volume".into(), "0".into()),
                ("postpone_options".into(), "[5,10,15]".into()),
            ]),
        )
        .unwrap();
    }
}
