use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Emitter};
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
pub struct PowerMonitor {
    app: AppHandle,
    running: Arc<Mutex<bool>>,
    pause_state: Arc<StdMutex<Option<SystemPause>>>,
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

        let app = self.app.clone();
        let running = self.running.clone();
        let pause_state = self.pause_state.clone();

        tokio::spawn(async move {
            let mut tick_interval = interval(Duration::from_secs(1));
            let mut last_tick = Utc::now();
            let mut last_locked = is_session_locked();

            loop {
                tick_interval.tick().await;

                // 检查是否应该继续运行
                {
                    let r = running.lock().await;
                    if !*r {
                        break;
                    }
                }

                let now = Utc::now();
                let elapsed = (now - last_tick).num_seconds();
                let locked = is_session_locked();

                match (last_locked, locked) {
                    (Some(false), Some(true)) => {
                        start_pause(&app, &pause_state, "locked", now);
                    }
                    (Some(true), Some(false)) => {
                        finish_pause(&app, &pause_state, "unlocked", now);
                    }
                    _ => {}
                }

                // 如果间隔超过 5 秒，认为系统从休眠中恢复
                if elapsed > 5 && !is_paused(&pause_state) {
                    let _ = app.emit(
                        "power:state-changed",
                        PowerStateChanged {
                            state: "resume".to_string(),
                            reason: "wake".to_string(),
                            paused_seconds: Some(elapsed),
                        },
                    );
                    let _ = app.emit(
                        "system:resumed",
                        PowerStateChanged {
                            state: "resume".to_string(),
                            reason: "wake".to_string(),
                            paused_seconds: Some(elapsed),
                        },
                    );
                }

                last_tick = now;
                if locked.is_some() {
                    last_locked = locked;
                }
            }
        });
    }

    /// 停止电源事件监听
    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }
}

fn is_paused(pause_state: &Arc<StdMutex<Option<SystemPause>>>) -> bool {
    pause_state.lock().unwrap().is_some()
}

fn start_pause(
    app: &AppHandle,
    pause_state: &Arc<StdMutex<Option<SystemPause>>>,
    reason: &str,
    now: DateTime<Utc>,
) {
    {
        let mut state = pause_state.lock().unwrap();
        if state.is_some() {
            return;
        }

        *state = Some(SystemPause { paused_at: now });
    }

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
        let mut state = pause_state.lock().unwrap();
        state.take()
    };

    let Some(pause) = pause else {
        return;
    };

    let paused_seconds = (now - pause.paused_at).num_seconds().max(0);
    let payload = PowerStateChanged {
        state: "resume".to_string(),
        reason: reason.to_string(),
        paused_seconds: Some(paused_seconds),
    };
    let _ = app.emit("power:state-changed", &payload);
    let _ = app.emit("system:resumed", &payload);
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
