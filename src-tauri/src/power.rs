use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

/// 电源状态变化事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerStateChanged {
    pub state: String,
    pub reason: String,
    pub paused_seconds: Option<i64>,
}

/// 系统唤醒后的恢复信息
#[derive(Debug, Clone, Serialize)]
pub struct PowerResumed {
    pub resumed_at: String,
    pub affected_reminders: i64,
}

/// 通过时间跳跃检测系统休眠/唤醒
/// 原理：正常情况下 tick 间隔应该接近 1 秒，
/// 如果检测到间隔超过 5 秒，说明系统可能从休眠中恢复
#[derive(Clone)]
pub struct PowerMonitor {
    app: AppHandle,
    running: Arc<Mutex<bool>>,
    pause_state: Arc<StdMutex<Option<SystemPause>>>,
    last_observed: Arc<StdMutex<DateTime<Utc>>>,
}

#[derive(Debug, Clone)]
struct SystemPause {
    paused_at: DateTime<Utc>,
}

impl PowerMonitor {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            running: Arc::new(Mutex::new(false)),
            pause_state: Arc::new(StdMutex::new(None)),
            last_observed: Arc::new(StdMutex::new(Utc::now())),
        }
    }

    pub fn is_paused(&self) -> bool {
        self.pause_state.lock().unwrap().is_some()
    }

    /// 启动电源事件监听
    pub async fn start(&self) {
        let mut running = self.running.lock().await;
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let monitor = self.clone();
        tokio::spawn(async move {
            let mut ticks = interval(Duration::from_secs(1));
            ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                ticks.tick().await;
                if !*monitor.running.lock().await {
                    break;
                }
                monitor.refresh();
            }
        });
    }

    /// Called by both polling and notification dispatch, before any DB lock.
    /// Holding the observation lock through compensation prevents another
    /// scheduler from seeing a wake as handled before its schedule is repaired.
    pub fn refresh(&self) {
        let mut previous = self.last_observed.lock().unwrap();
        let now = Utc::now();
        match power_transition(*previous, now, is_session_locked(), self.is_paused()) {
            PowerTransition::Start(at) => start_pause(&self.app, &self.pause_state, "locked", at),
            PowerTransition::Finish => finish_pause(&self.app, &self.pause_state, "unlocked", now),
            PowerTransition::Wake(at) => {
                compensate_pause(&self.app, at, now);
                let payload = PowerStateChanged {
                    state: "resume".into(),
                    reason: "wake".into(),
                    paused_seconds: Some((now - at).num_seconds().max(0)),
                };
                let _ = self.app.emit("power:state-changed", &payload);
                let _ = self.app.emit("system:resumed", &payload);
            }
            PowerTransition::None => {}
        }
        *previous = now;
    }

    /// 停止电源事件监听
    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }
}

#[derive(Debug, PartialEq)]
enum PowerTransition {
    Start(DateTime<Utc>),
    Finish,
    Wake(DateTime<Utc>),
    None,
}

fn power_transition(
    previous: DateTime<Utc>,
    now: DateTime<Utc>,
    locked: Option<bool>,
    paused: bool,
) -> PowerTransition {
    let time_gap = (now - previous).num_seconds() > 5;
    match (locked, paused) {
        (Some(true), false) => PowerTransition::Start(if time_gap { previous } else { now }),
        (Some(false), true) => PowerTransition::Finish,
        (_, false) if time_gap => PowerTransition::Wake(previous),
        _ => PowerTransition::None,
    }
}

fn start_pause(
    app: &AppHandle,
    pause_state: &Arc<StdMutex<Option<SystemPause>>>,
    reason: &str,
    now: DateTime<Utc>,
) {
    let db = app.state::<crate::db::Database>();
    let conn = db.conn.lock().unwrap();
    {
        let mut state = pause_state.lock().unwrap();
        if state.is_some() {
            return;
        }

        *state = Some(SystemPause { paused_at: now });
    }

    if let Err(error) = conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        (
            crate::commands::SYSTEM_PAUSED_AT_KEY,
            now.format("%Y-%m-%dT%H:%M:%S").to_string(),
        ),
    ) {
        crate::app_log::error(format!("记录锁屏暂停失败：{error}"));
    }
    drop(conn);

    let payload = PowerStateChanged {
        state: "pause".to_string(),
        reason: reason.to_string(),
        paused_seconds: None,
    };
    let _ = app.emit("power:state-changed", &payload);
    let _ = app.emit("system:paused", &payload);
}

fn finish_pause(
    app: &AppHandle,
    pause_state: &Arc<StdMutex<Option<SystemPause>>>,
    reason: &str,
    now: DateTime<Utc>,
) {
    let pause = {
        let state = pause_state.lock().unwrap();
        state.clone()
    };

    let Some(pause) = pause else {
        return;
    };

    let paused_seconds = (now - pause.paused_at).num_seconds().max(0);
    compensate_pause(app, pause.paused_at, now);
    pause_state.lock().unwrap().take();
    let payload = PowerStateChanged {
        state: "resume".to_string(),
        reason: reason.to_string(),
        paused_seconds: Some(paused_seconds),
    };
    let _ = app.emit("power:state-changed", &payload);
    let _ = app.emit("system:resumed", &payload);
}

fn compensate_pause(app: &AppHandle, started_at: DateTime<Utc>, now: DateTime<Utc>) {
    let db = app.state::<crate::db::Database>();
    let conn = db.conn.lock().unwrap();
    match crate::commands::compensate_system_pause(&conn, started_at, now) {
        Ok(()) => {
            let affected = conn
                .query_row(
                    "SELECT COUNT(*) FROM reminders WHERE enabled = 1",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0);
            drop(conn);
            let _ = app.emit(
                "power:resumed",
                PowerResumed {
                    resumed_at: now.format("%Y-%m-%dT%H:%M:%S").to_string(),
                    affected_reminders: affected,
                },
            );
            let _ = app.emit("reminders:changed", ());
        }
        Err(error) => crate::app_log::error(format!("恢复锁屏/休眠暂停失败：{error}")),
    }
}

#[cfg(target_os = "windows")]
fn is_session_locked() -> Option<bool> {
    use windows::Win32::System::RemoteDesktop::{
        ProcessIdToSessionId, WTSFreeMemory, WTSQuerySessionInformationW, WTSSessionInfoEx,
        WTSINFOEXW, WTS_CURRENT_SERVER_HANDLE, WTS_SESSIONSTATE_LOCK,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;

    unsafe {
        let mut session_id = 0;
        ProcessIdToSessionId(GetCurrentProcessId(), &mut session_id).ok()?;

        let mut buffer = windows::core::PWSTR::null();
        let mut bytes_returned = 0;
        WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            session_id,
            WTSSessionInfoEx,
            &mut buffer,
            &mut bytes_returned,
        )
        .ok()?;

        if buffer.is_null() {
            return None;
        }

        let info = *(buffer.0 as *const WTSINFOEXW);
        WTSFreeMemory(buffer.0 as *mut _);

        if info.Level != 1 {
            return None;
        }

        let level = info.Data.WTSInfoExLevel1;
        Some(level.SessionFlags as u32 == WTS_SESSIONSTATE_LOCK)
    }
}

#[cfg(not(target_os = "windows"))]
fn is_session_locked() -> Option<bool> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    #[test]
    fn waking_into_lock_screen_keeps_the_sleep_interval() {
        let before = Utc::now();
        let wake = before + ChronoDuration::hours(1);
        assert_eq!(
            power_transition(before, wake, Some(true), false),
            PowerTransition::Start(before)
        );
        assert_eq!(
            power_transition(wake, wake + ChronoDuration::seconds(1), Some(false), true),
            PowerTransition::Finish
        );
    }

    #[test]
    fn ordinary_lock_starts_now_and_locked_sleep_is_not_compensated_twice() {
        let before = Utc::now();
        let now = before + ChronoDuration::seconds(1);
        assert_eq!(
            power_transition(before, now, Some(true), false),
            PowerTransition::Start(now)
        );
        assert_eq!(
            power_transition(before, before + ChronoDuration::hours(1), Some(true), true),
            PowerTransition::None
        );
        assert_eq!(
            power_transition(before, before + ChronoDuration::hours(1), Some(false), true),
            PowerTransition::Finish
        );
    }

    #[test]
    fn unlocked_wake_is_compensated_once_even_without_lock_detection() {
        let before = Utc::now();
        let wake = before + ChronoDuration::hours(1);
        for locked in [None, Some(false)] {
            assert_eq!(
                power_transition(before, wake, locked, false),
                PowerTransition::Wake(before)
            );
            assert_eq!(
                power_transition(wake, wake, locked, false),
                PowerTransition::None
            );
        }
    }
}
