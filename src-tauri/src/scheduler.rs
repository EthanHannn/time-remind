use chrono::{DateTime, Duration as ChronoDuration, Local, NaiveDateTime, NaiveTime, Utc};
use serde::Serialize;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Event, Listener, Manager};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{interval, sleep, Duration};

use crate::db::Database;
use crate::fullscreen::is_foreground_window_fullscreen;
use crate::power::{PowerMonitor, PowerStateChanged};

const NOTIFICATION_WINDOW_WIDTH: f64 = 360.0;
const NOTIFICATION_WINDOW_HEIGHT: f64 = 224.0;
const NOTIFICATION_WINDOW_RIGHT_MARGIN: f64 = 20.0;
const NOTIFICATION_WINDOW_BOTTOM_MARGIN: f64 = 96.0;
const NEXT_NOTIFICATION_DELAY_MS: u64 = 220;

/// 通知窗口显示数据
#[derive(Debug, Clone, Serialize)]
pub struct NotificationData {
    pub reminder_id: String,
    pub name: String,
    pub icon: String,
    pub reminder_type: String,
    pub message: String,
    pub break_duration_minutes: i64,
    pub break_notification_enabled: bool,
    pub action_enabled: bool,
    pub action_title: String,
    pub action_message: String,
    pub action_duration_seconds: i64,
    pub action_completion_mode: String,
    pub pending_count: usize,
}

/// 通知队列状态
#[derive(Debug, Clone, Serialize)]
struct NotificationQueueState {
    pub current_reminder_id: Option<String>,
    pub pending_count: usize,
}

/// 定时器 tick 事件数据
#[derive(Debug, Clone, Serialize)]
pub struct TimerTick {
    pub reminder_id: String,
    pub remaining_seconds: i64,
}

/// 提醒触发事件数据
#[derive(Debug, Clone, Serialize)]
pub struct ReminderTriggered {
    pub reminder_id: String,
    pub name: String,
    pub icon: String,
    pub reminder_type: String,
    pub break_duration_minutes: i64,
    pub break_notification_enabled: bool,
    pub action_enabled: bool,
    pub action_title: String,
    pub action_message: String,
    pub action_duration_seconds: i64,
    pub action_completion_mode: String,
    pub message: String,
}

/// 免打扰配置
#[derive(Debug, Clone)]
struct DndConfig {
    enabled: bool,
    start: NaiveTime,
    end: NaiveTime,
}

/// 静默调度决策
#[derive(Debug, Clone)]
struct DndDecision {
    next_trigger: DateTime<Utc>,
}

/// 从数据库读取免打扰配置
fn get_dnd_config_from_db(db: &Database) -> DndConfig {
    let conn = db.conn.lock().unwrap();

    let enabled: bool = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'dnd_enabled'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(false);

    let start_str: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'dnd_start'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .unwrap_or_else(|| "22:00".to_string());

    let end_str: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'dnd_end'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .unwrap_or_else(|| "08:00".to_string());

    let start = NaiveTime::parse_from_str(&start_str, "%H:%M")
        .unwrap_or_else(|_| NaiveTime::from_hms_opt(22, 0, 0).unwrap());
    let end = NaiveTime::parse_from_str(&end_str, "%H:%M")
        .unwrap_or_else(|_| NaiveTime::from_hms_opt(8, 0, 0).unwrap());

    DndConfig {
        enabled,
        start,
        end,
    }
}

/// 检查当前是否在免打扰时间段内
fn is_in_dnd_period(config: &DndConfig) -> bool {
    if !config.enabled {
        return false;
    }

    let now = Local::now().time();

    if config.start <= config.end {
        now >= config.start && now < config.end
    } else {
        now >= config.start || now < config.end
    }
}

fn next_dnd_end_utc(config: &DndConfig, now: DateTime<Utc>) -> DateTime<Utc> {
    let local_now = now.with_timezone(&Local);
    let today = local_now.date_naive();
    let end_date = if config.start <= config.end {
        today
    } else if local_now.time() >= config.start {
        today
            .checked_add_signed(ChronoDuration::days(1))
            .unwrap_or(today)
    } else {
        today
    };

    let end_naive = end_date.and_time(config.end);
    match end_naive.and_local_timezone(Local) {
        chrono::LocalResult::Single(value) => value.with_timezone(&Utc),
        chrono::LocalResult::Ambiguous(value, _) => value.with_timezone(&Utc),
        chrono::LocalResult::None => now + ChronoDuration::minutes(5),
    }
}

/// 读取临时免打扰到期时间
fn get_temp_dnd_until(db: &Database) -> Option<DateTime<Utc>> {
    let conn = db.conn.lock().unwrap();
    let until_str: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'temp_dnd_until'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok();

    until_str
        .and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S").ok())
        .map(|timestamp| timestamp.and_utc())
}

fn is_fullscreen_detection_enabled(db: &Database) -> bool {
    let conn = db.conn.lock().unwrap();
    conn.query_row(
        "SELECT value FROM settings WHERE key = 'fullscreen_detection_enabled'",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .and_then(|value| value.parse().ok())
    .unwrap_or(true)
}

fn are_all_reminders_paused(db: &Database) -> bool {
    let conn = db.conn.lock().unwrap();
    crate::commands::all_reminders_paused(&conn)
}

fn resolve_dnd_decision(db: &Database, now: DateTime<Utc>) -> Option<DndDecision> {
    let dnd_config = get_dnd_config_from_db(db);
    if is_in_dnd_period(&dnd_config) {
        return Some(DndDecision {
            next_trigger: next_dnd_end_utc(&dnd_config, now),
        });
    }

    if let Some(temp_until) = get_temp_dnd_until(db) {
        if temp_until > now {
            return Some(DndDecision {
                next_trigger: temp_until,
            });
        }
    }

    if is_fullscreen_detection_enabled(db) && is_foreground_window_fullscreen() {
        return Some(DndDecision {
            next_trigger: now + ChronoDuration::minutes(5),
        });
    }

    None
}

fn shifted_next_trigger(
    next_trigger: Option<&str>,
    interval_minutes: i64,
    paused_seconds: i64,
    now: DateTime<Utc>,
) -> String {
    next_trigger
        .and_then(|value| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").ok())
        .map(|timestamp| timestamp.and_utc() + ChronoDuration::seconds(paused_seconds))
        .unwrap_or_else(|| now + ChronoDuration::minutes(interval_minutes))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

fn shift_enabled_reminders(db: &Database, paused_seconds: i64) -> Result<i64, String> {
    let conn = db.conn.lock().unwrap();
    let now = Utc::now();
    let now_str = now.format("%Y-%m-%dT%H:%M:%S").to_string();

    let mut stmt = conn
        .prepare("SELECT id, interval_minutes, next_trigger FROM reminders WHERE enabled = 1")
        .map_err(|e| e.to_string())?;

    let reminders: Vec<(String, i64, Option<String>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    for (id, interval_minutes, next_trigger) in &reminders {
        let new_next = shifted_next_trigger(
            next_trigger.as_deref(),
            *interval_minutes,
            paused_seconds,
            now,
        );
        let _ = conn.execute(
            "UPDATE reminders SET next_trigger = ?1, updated_at = ?2 WHERE id = ?3",
            (&new_next, &now_str, id),
        );
    }

    Ok(reminders.len() as i64)
}

/// 定时器引擎
#[derive(Clone)]
pub struct Scheduler {
    app: AppHandle,
    running: Arc<AsyncMutex<bool>>,
    active_reminders: Arc<std::sync::Mutex<HashSet<String>>>,
    current_notification: Arc<std::sync::Mutex<Option<String>>>,
    pending_notifications: Arc<std::sync::Mutex<VecDeque<NotificationData>>>,
}

impl Scheduler {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            running: Arc::new(AsyncMutex::new(false)),
            active_reminders: Arc::new(std::sync::Mutex::new(HashSet::new())),
            current_notification: Arc::new(std::sync::Mutex::new(None)),
            pending_notifications: Arc::new(std::sync::Mutex::new(VecDeque::new())),
        }
    }

    pub fn clear_active(&self, reminder_id: &str) {
        let mut active_reminders = self.active_reminders.lock().unwrap();
        active_reminders.remove(reminder_id);
    }

    pub fn clear_all_active(&self) {
        self.active_reminders.lock().unwrap().clear();
        self.current_notification.lock().unwrap().take();
        self.pending_notifications.lock().unwrap().clear();
        self.emit_queue_state();
        crate::set_tray_visual_state(&self.app, crate::TrayVisualState::Idle);
    }

    pub fn enqueue_notification(&self, data: NotificationData) -> Result<(), String> {
        let should_show_now = {
            let mut current = self.current_notification.lock().unwrap();
            if current.is_none() {
                *current = Some(data.reminder_id.clone());
                true
            } else {
                false
            }
        };

        if should_show_now {
            if let Err(error) = self.show_notification(&data) {
                self.current_notification.lock().unwrap().take();
                self.emit_queue_state();
                return Err(error);
            }
        } else {
            self.pending_notifications.lock().unwrap().push_back(data);
            self.emit_queue_state();
        }

        crate::set_tray_visual_state(&self.app, crate::TrayVisualState::Alert);
        Ok(())
    }

    pub fn release_notification(&self, reminder_id: &str) -> bool {
        self.clear_active(reminder_id);

        let removed_current = {
            let mut current = self.current_notification.lock().unwrap();
            if current.as_deref() == Some(reminder_id) {
                current.take();
                true
            } else {
                false
            }
        };

        if !removed_current {
            let mut pending = self.pending_notifications.lock().unwrap();
            pending.retain(|item| item.reminder_id != reminder_id);
            drop(pending);
            self.emit_queue_state();
        }

        self.schedule_next_notification();
        removed_current
    }

    fn schedule_next_notification(&self) {
        let scheduler = self.clone();
        tauri::async_runtime::spawn(async move {
            sleep(Duration::from_millis(NEXT_NOTIFICATION_DELAY_MS)).await;
            scheduler.show_next_notification();
        });
    }

    fn show_next_notification(&self) {
        if self.current_notification.lock().unwrap().is_some() {
            self.update_tray_state();
            return;
        }

        loop {
            let next = self.pending_notifications.lock().unwrap().pop_front();

            let Some(data) = next else {
                self.emit_queue_state();
                self.update_tray_state();
                break;
            };

            {
                let mut current = self.current_notification.lock().unwrap();
                *current = Some(data.reminder_id.clone());
            }

            if self.show_notification(&data).is_ok() {
                self.emit_queue_state();
                crate::set_tray_visual_state(&self.app, crate::TrayVisualState::Alert);
                break;
            }

            self.current_notification.lock().unwrap().take();
            self.clear_active(&data.reminder_id);
            self.emit_queue_state();
        }
    }

    fn update_tray_state(&self) {
        let has_current = self.current_notification.lock().unwrap().is_some();
        let has_pending = !self.pending_notifications.lock().unwrap().is_empty();

        if has_current || has_pending {
            crate::set_tray_visual_state(&self.app, crate::TrayVisualState::Alert);
        } else {
            crate::set_tray_visual_state(&self.app, crate::TrayVisualState::Idle);
        }
    }

    fn show_notification(&self, data: &NotificationData) -> Result<(), String> {
        let Some(notification_window) = self.app.get_webview_window("notification") else {
            return Err("通知窗口不存在".to_string());
        };

        let payload = NotificationData {
            pending_count: self.pending_notifications.lock().unwrap().len(),
            ..data.clone()
        };

        notification_window
            .emit("notification:show", payload)
            .map_err(|e| e.to_string())?;
        notification_window.show().map_err(|e| e.to_string())?;

        let monitor = self
            .app
            .cursor_position()
            .ok()
            .and_then(|position| {
                self.app
                    .monitor_from_point(position.x, position.y)
                    .ok()
                    .flatten()
            })
            .or_else(|| self.app.primary_monitor().ok().flatten());

        if let Some(monitor) = monitor {
            let work_area = monitor.work_area();
            let scale_factor = monitor.scale_factor();
            let left = work_area.position.x as f64 / scale_factor;
            let top = work_area.position.y as f64 / scale_factor;
            let width = work_area.size.width as f64 / scale_factor;
            let height = work_area.size.height as f64 / scale_factor;
            let x = (left + width - NOTIFICATION_WINDOW_WIDTH - NOTIFICATION_WINDOW_RIGHT_MARGIN)
                .max(left);
            let y = (top + height - NOTIFICATION_WINDOW_HEIGHT - NOTIFICATION_WINDOW_BOTTOM_MARGIN)
                .max(top);

            let _ = notification_window
                .set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
        }

        Ok(())
    }

    fn emit_queue_state(&self) {
        let current_reminder_id = self.current_notification.lock().unwrap().clone();
        let pending_count = self.pending_notifications.lock().unwrap().len();
        let _ = self.app.emit(
            "notification:queue-updated",
            NotificationQueueState {
                current_reminder_id,
                pending_count,
            },
        );
    }

    async fn run_loop(app: AppHandle, running: Arc<AsyncMutex<bool>>, scheduler: Scheduler) {
        let mut tick_interval = interval(Duration::from_secs(1));

        loop {
            tick_interval.tick().await;

            {
                let r = running.lock().await;
                if !*r {
                    break;
                }
            }

            let db = app.state::<Database>();
            if are_all_reminders_paused(&db) {
                continue;
            }

            if app
                .try_state::<PowerMonitor>()
                .map(|monitor| monitor.is_paused())
                .unwrap_or(false)
            {
                continue;
            }

            let now = Utc::now();

            let reminders = {
                let conn = db.conn.lock().unwrap();
                let mut stmt = conn
                    .prepare(
                        "SELECT id, name, reminder_type, icon, message, break_duration_minutes, break_notification_enabled, action_enabled, action_title, action_message, action_duration_seconds, action_completion_mode, next_trigger
                             FROM reminders
                             WHERE enabled = 1 AND next_trigger IS NOT NULL",
                    )
                    .unwrap();

                stmt.query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i32>(6)? != 0,
                        row.get::<_, i32>(7)? != 0,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, i64>(10)?,
                        row.get::<_, String>(11)?,
                        row.get::<_, String>(12)?,
                    ))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
            };

            for (
                id,
                name,
                reminder_type,
                icon,
                message,
                break_duration_minutes,
                break_notification_enabled,
                action_enabled,
                action_title,
                action_message,
                action_duration_seconds,
                action_completion_mode,
                next_trigger_str,
            ) in &reminders
            {
                let next_trigger =
                    chrono::NaiveDateTime::parse_from_str(next_trigger_str, "%Y-%m-%dT%H:%M:%S");

                if let Ok(next_trigger_naive) = next_trigger {
                    let next_trigger_utc = next_trigger_naive.and_utc();
                    let remaining = (next_trigger_utc - now).num_seconds();

                    if remaining <= 0 {
                        {
                            let mut active = scheduler.active_reminders.lock().unwrap();
                            if active.contains(id) {
                                continue;
                            }
                            active.insert(id.clone());
                        }

                        if let Some(dnd_decision) = resolve_dnd_decision(&db, now) {
                            scheduler.clear_active(id);

                            let new_next = dnd_decision
                                .next_trigger
                                .format("%Y-%m-%dT%H:%M:%S")
                                .to_string();
                            let conn = db.conn.lock().unwrap();
                            let _ = conn.execute(
                                "UPDATE reminders SET next_trigger = ?1 WHERE id = ?2",
                                (&new_next, id),
                            );
                            continue;
                        }

                        let event = ReminderTriggered {
                            reminder_id: id.clone(),
                            name: name.clone(),
                            icon: icon.clone(),
                            reminder_type: reminder_type.clone(),
                            break_duration_minutes: *break_duration_minutes,
                            break_notification_enabled: *break_notification_enabled,
                            action_enabled: *action_enabled,
                            action_title: action_title.clone(),
                            action_message: action_message.clone(),
                            action_duration_seconds: *action_duration_seconds,
                            action_completion_mode: action_completion_mode.clone(),
                            message: message.clone(),
                        };
                        let _ = app.emit("reminder:triggered", &event);

                        let notification = NotificationData {
                            reminder_id: id.clone(),
                            name: name.clone(),
                            icon: icon.clone(),
                            reminder_type: reminder_type.clone(),
                            message: message.clone(),
                            break_duration_minutes: *break_duration_minutes,
                            break_notification_enabled: *break_notification_enabled,
                            action_enabled: *action_enabled,
                            action_title: action_title.clone(),
                            action_message: action_message.clone(),
                            action_duration_seconds: *action_duration_seconds,
                            action_completion_mode: action_completion_mode.clone(),
                            pending_count: 0,
                        };

                        if scheduler.enqueue_notification(notification).is_err() {
                            scheduler.clear_active(id);
                            scheduler.current_notification.lock().unwrap().take();
                            scheduler.update_tray_state();
                        }
                    } else {
                        let tick = TimerTick {
                            reminder_id: id.clone(),
                            remaining_seconds: remaining,
                        };
                        let _ = app.emit("timer:tick", &tick);
                    }
                }
            }
        }
    }

    /// 启动定时器
    pub async fn start(&self) {
        let mut running = self.running.lock().await;
        if *running {
            return;
        }
        *running = true;
        drop(running);

        crate::app_log::info("定时器监督任务启动");

        let app = self.app.clone();
        let running = self.running.clone();
        let scheduler = self.clone();

        let app_clone = app.clone();
        app.listen("power:state-changed", move |event: Event| {
            if let Ok(data) = serde_json::from_str::<PowerStateChanged>(&event.payload()) {
                if data.state == "resume" {
                    let db = app_clone.state::<Database>();
                    let paused_seconds = data.paused_seconds.unwrap_or(0).max(0);
                    if paused_seconds == 0 {
                        return;
                    }

                    if let Ok(affected) = shift_enabled_reminders(&db, paused_seconds) {
                        let _ = app_clone.emit(
                            "power:resumed",
                            crate::power::PowerResumed {
                                resumed_at: Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                                affected_reminders: affected,
                            },
                        );
                        let _ = app_clone.emit("reminders:changed", ());
                    }
                }
            }
        });

        tokio::spawn(async move {
            loop {
                {
                    let r = running.lock().await;
                    if !*r {
                        break;
                    }
                }

                let worker_app = app.clone();
                let worker_running = running.clone();
                let worker_scheduler = scheduler.clone();

                let result = tokio::spawn(async move {
                    Self::run_loop(worker_app, worker_running, worker_scheduler).await;
                })
                .await;

                match result {
                    Ok(()) => {
                        let r = running.lock().await;
                        if *r {
                            crate::app_log::warn("定时器工作循环异常结束，准备重启");
                            drop(r);
                            sleep(Duration::from_secs(2)).await;
                            continue;
                        }
                        break;
                    }
                    Err(error) if error.is_panic() => {
                        crate::app_log::error(format!("定时器工作循环崩溃，准备重启：{error}"));
                        scheduler.clear_all_active();
                        sleep(Duration::from_secs(2)).await;
                    }
                    Err(error) => {
                        crate::app_log::error(format!("定时器工作循环退出：{error}"));
                        break;
                    }
                }
            }
        });
    }

    /// 停止定时器
    #[allow(dead_code)]
    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn shifted_next_trigger_keeps_remaining_time_after_pause() {
        let now = Utc
            .with_ymd_and_hms(2026, 7, 1, 10, 0, 0)
            .single()
            .expect("fixed time should be valid");

        let shifted = shifted_next_trigger(Some("2026-07-01T10:01:00"), 20, 180, now);

        assert_eq!(shifted, "2026-07-01T10:04:00");
    }

    #[test]
    fn shifted_next_trigger_repairs_missing_timestamp() {
        let now = Utc
            .with_ymd_and_hms(2026, 7, 1, 10, 0, 0)
            .single()
            .expect("fixed time should be valid");

        let shifted = shifted_next_trigger(None, 20, 180, now);

        assert_eq!(shifted, "2026-07-01T10:20:00");
    }
}
