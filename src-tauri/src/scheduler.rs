use chrono::{DateTime, Duration as ChronoDuration, Local, NaiveDateTime, NaiveTime, Utc};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{interval, sleep, Duration};

use crate::db::Database;
use crate::fullscreen::is_foreground_window_fullscreen;
use crate::power::PowerMonitor;

const NOTIFICATION_WINDOW_WIDTH: f64 = 360.0;
const NOTIFICATION_WINDOW_HEIGHT: f64 = 224.0;
const NOTIFICATION_WINDOW_RIGHT_MARGIN: f64 = 20.0;
const NOTIFICATION_WINDOW_BOTTOM_MARGIN: f64 = 96.0;
const NEXT_NOTIFICATION_DELAY_MS: u64 = 220;

fn notification_window_position(
    work_area_position: tauri::PhysicalPosition<i32>,
    work_area_size: tauri::PhysicalSize<u32>,
    scale_factor: f64,
) -> tauri::PhysicalPosition<i32> {
    let window_width = NOTIFICATION_WINDOW_WIDTH * scale_factor;
    let window_height = NOTIFICATION_WINDOW_HEIGHT * scale_factor;
    let right_margin = NOTIFICATION_WINDOW_RIGHT_MARGIN * scale_factor;
    let bottom_margin = NOTIFICATION_WINDOW_BOTTOM_MARGIN * scale_factor;

    let left = f64::from(work_area_position.x);
    let top = f64::from(work_area_position.y);
    let x = (left + f64::from(work_area_size.width) - window_width - right_margin).max(left);
    let y = (top + f64::from(work_area_size.height) - window_height - bottom_margin).max(top);

    tauri::PhysicalPosition::new(x.round() as i32, y.round() as i32)
}

/// 通知窗口显示数据
#[derive(Debug, Clone, Serialize)]
pub struct NotificationData {
    pub queue_revision: u64,
    pub notification_id: String,
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

#[derive(Clone)]
struct NotificationIdentity {
    reminder_id: String,
    notification_id: String,
}

impl NotificationIdentity {
    fn matches(&self, reminder_id: &str, notification_id: &str) -> bool {
        self.reminder_id == reminder_id && self.notification_id == notification_id
    }
}

/// 通知队列状态
#[derive(Debug, Clone, Serialize)]
struct NotificationQueueState {
    pub queue_revision: u64,
    pub current_notification_id: Option<String>,
    pub current_reminder_id: Option<String>,
    pub pending_count: usize,
}

#[derive(Default)]
struct NotificationQueue {
    current: Option<NotificationIdentity>,
    pending: VecDeque<NotificationData>,
    revision: u64,
}

impl NotificationQueue {
    fn snapshot(&self) -> NotificationQueueState {
        NotificationQueueState {
            queue_revision: self.revision,
            current_reminder_id: self.current.as_ref().map(|item| item.reminder_id.clone()),
            current_notification_id: self
                .current
                .as_ref()
                .map(|item| item.notification_id.clone()),
            pending_count: self.pending.len(),
        }
    }
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
fn get_dnd_config_from_db(conn: &Connection) -> DndConfig {
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
fn get_temp_dnd_until(conn: &Connection) -> Option<DateTime<Utc>> {
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

fn is_fullscreen_detection_enabled(conn: &Connection) -> bool {
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

fn resolve_dnd_decision(
    conn: &Connection,
    now: DateTime<Utc>,
    fullscreen: bool,
) -> Option<DndDecision> {
    let dnd_config = get_dnd_config_from_db(conn);
    if is_in_dnd_period(&dnd_config) {
        return Some(DndDecision {
            next_trigger: next_dnd_end_utc(&dnd_config, now),
        });
    }

    if let Some(temp_until) = get_temp_dnd_until(conn) {
        if temp_until > now {
            return Some(DndDecision {
                next_trigger: temp_until,
            });
        }
    }

    if is_fullscreen_detection_enabled(conn) && fullscreen {
        return Some(DndDecision {
            next_trigger: now + ChronoDuration::minutes(5),
        });
    }

    None
}

fn notifications_suppressed(
    conn: &Connection,
    now: DateTime<Utc>,
    system_paused: bool,
    fullscreen: bool,
) -> bool {
    crate::commands::all_reminders_paused(conn)
        || system_paused
        || resolve_dnd_decision(conn, now, fullscreen).is_some()
}

fn reminder_is_due(conn: &Connection, id: &str, now: DateTime<Utc>) -> bool {
    conn.query_row(
        "SELECT enabled = 1 AND next_trigger <= ?1 FROM reminders WHERE id = ?2",
        (now.format("%Y-%m-%dT%H:%M:%S").to_string(), id),
        |row| row.get::<_, bool>(0),
    )
    .unwrap_or(false)
}

/// 定时器引擎
#[derive(Clone)]
pub struct Scheduler {
    app: AppHandle,
    running: Arc<AsyncMutex<bool>>,
    active_reminders: Arc<std::sync::Mutex<HashSet<String>>>,
    queue: Arc<std::sync::Mutex<NotificationQueue>>,
    next_notification_at: Arc<std::sync::Mutex<std::time::Instant>>,
}

impl Scheduler {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            running: Arc::new(AsyncMutex::new(false)),
            active_reminders: Arc::new(std::sync::Mutex::new(HashSet::new())),
            queue: Arc::new(std::sync::Mutex::new(NotificationQueue::default())),
            next_notification_at: Arc::new(std::sync::Mutex::new(std::time::Instant::now())),
        }
    }

    pub fn clear_active(&self, reminder_id: &str) {
        let mut active_reminders = self.active_reminders.lock().unwrap();
        active_reminders.remove(reminder_id);
    }

    pub fn is_current_notification(&self, reminder_id: &str, notification_id: &str) -> bool {
        self.queue
            .lock()
            .unwrap()
            .current
            .as_ref()
            .is_some_and(|current| current.matches(reminder_id, notification_id))
    }

    pub fn clear_all_active(&self) {
        self.active_reminders.lock().unwrap().clear();
        {
            let mut queue = self.queue.lock().unwrap();
            queue.current = None;
            queue.pending.clear();
            queue.revision += 1;
            self.hide_notification_window();
        }
        self.emit_queue_state();
        crate::set_tray_visual_state(&self.app, crate::TrayVisualState::Idle);
    }

    pub fn enqueue_notification(&self, data: NotificationData) -> Result<(), String> {
        if let Some(power) = self.app.try_state::<PowerMonitor>() {
            power.refresh();
        }
        let db = self.app.state::<Database>();
        let conn = db.conn.lock().unwrap();
        let now = Utc::now();
        // Revalidate under the same DB lock used by pause/edit/respond commands.
        // The run loop may have read this reminder before one of those commands.
        if crate::commands::all_reminders_paused(&conn)
            || !reminder_is_due(&conn, &data.reminder_id, now)
        {
            self.clear_active(&data.reminder_id);
            return Ok(());
        }
        if let Some(decision) = resolve_dnd_decision(&conn, now, is_foreground_window_fullscreen())
        {
            self.clear_active(&data.reminder_id);
            conn.execute(
                "UPDATE reminders SET next_trigger = ?1 WHERE id = ?2",
                (
                    decision
                        .next_trigger
                        .format("%Y-%m-%dT%H:%M:%S")
                        .to_string(),
                    &data.reminder_id,
                ),
            )
            .map_err(|error| error.to_string())?;
            return Ok(());
        }
        {
            let mut queue = self.queue.lock().unwrap();
            queue.pending.push_back(data);
            queue.revision += 1;
        }
        drop(conn);
        self.show_next_notification();
        self.emit_queue_state();
        Ok(())
    }

    pub fn release_notification(&self, reminder_id: &str) -> bool {
        self.clear_active(reminder_id);

        let removed_current = {
            let mut queue = self.queue.lock().unwrap();
            let removed = queue
                .current
                .as_ref()
                .is_some_and(|item| item.reminder_id == reminder_id);
            if removed {
                queue.current = None;
            }
            queue.pending.retain(|item| item.reminder_id != reminder_id);
            queue.revision += 1;
            removed
        };
        self.emit_queue_state();

        if removed_current {
            // Close on the native side before scheduling another instance.
            // A delayed webview hide request could otherwise hide its successor.
            self.hide_notification_window();
            *self.next_notification_at.lock().unwrap() =
                std::time::Instant::now() + Duration::from_millis(NEXT_NOTIFICATION_DELAY_MS);
        }
        self.schedule_next_notification();
        removed_current
    }

    fn hide_notification_window(&self) {
        if let Some(window) = self.app.get_webview_window("notification") {
            if let Err(error) = window.hide() {
                crate::app_log::warn(format!("隐藏通知窗口失败：{error}"));
            }
        }
    }

    fn schedule_next_notification(&self) {
        let scheduler = self.clone();
        tauri::async_runtime::spawn(async move {
            sleep(Duration::from_millis(NEXT_NOTIFICATION_DELAY_MS)).await;
            scheduler.show_next_notification();
        });
    }

    fn show_next_notification(&self) {
        // Native monitor/window getters can wait for the UI thread. Dispatch
        // there before acquiring the DB lock, so a tray command cannot deadlock
        // waiting for that lock while a worker waits for the UI thread.
        let scheduler = self.clone();
        if let Err(error) = self
            .app
            .run_on_main_thread(move || scheduler.dispatch_next_notification())
        {
            crate::app_log::error(format!("调度通知窗口失败：{error}"));
        }
    }

    fn dispatch_next_notification(&self) {
        if let Some(power) = self.app.try_state::<PowerMonitor>() {
            power.refresh();
        }
        // Serialize dispatch with database mutations and other queue workers.
        let db = self.app.state::<Database>();
        let conn = db.conn.lock().unwrap();
        if self.queue.lock().unwrap().current.is_some() {
            self.update_tray_state();
            return;
        }

        if std::time::Instant::now() < *self.next_notification_at.lock().unwrap() {
            return;
        }
        let system_paused = self
            .app
            .try_state::<PowerMonitor>()
            .map(|monitor| monitor.is_paused())
            .unwrap_or(false);
        if notifications_suppressed(
            &conn,
            Utc::now(),
            system_paused,
            is_foreground_window_fullscreen(),
        ) {
            return;
        }

        loop {
            let next = {
                let mut queue = self.queue.lock().unwrap();
                let next = queue.pending.pop_front();
                if next.is_some() {
                    queue.revision += 1;
                }
                next
            };

            let Some(data) = next else {
                self.emit_queue_state();
                self.update_tray_state();
                break;
            };

            if !reminder_is_due(&conn, &data.reminder_id, Utc::now()) {
                self.clear_active(&data.reminder_id);
                continue;
            }
            {
                let mut queue = self.queue.lock().unwrap();
                queue.revision += 1;
                queue.current = Some(NotificationIdentity {
                    reminder_id: data.reminder_id.clone(),
                    notification_id: data.notification_id.clone(),
                });
            }

            if self.show_notification(&data).is_ok() {
                self.emit_queue_state();
                crate::set_tray_visual_state(&self.app, crate::TrayVisualState::Alert);
                break;
            }

            {
                let mut queue = self.queue.lock().unwrap();
                queue.current = None;
                queue.revision += 1;
            }
            self.clear_active(&data.reminder_id);
            self.emit_queue_state();
        }
    }

    fn update_tray_state(&self) {
        let state = self.queue.lock().unwrap().snapshot();
        let has_current = state.current_reminder_id.is_some();
        let has_pending = state.pending_count > 0;

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

        let queue = self.queue.lock().unwrap();
        if !queue
            .current
            .as_ref()
            .is_some_and(|current| current.matches(&data.reminder_id, &data.notification_id))
        {
            return Err("通知已取消".into());
        }
        let state = queue.snapshot();
        let payload = NotificationData {
            queue_revision: state.queue_revision,
            pending_count: state.pending_count,
            ..data.clone()
        };

        notification_window
            .emit("notification:show", payload)
            .map_err(|e| e.to_string())?;

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

        let target_position = monitor.map(|monitor| {
            let work_area = monitor.work_area();
            let scale_factor = monitor.scale_factor();
            notification_window_position(work_area.position, work_area.size, scale_factor)
        });

        #[cfg(not(target_os = "linux"))]
        if let Some(position) = target_position {
            if let Err(error) =
                notification_window.set_position(tauri::Position::Physical(position))
            {
                crate::app_log::warn(format!("通知窗口定位失败：{error}"));
            }
        }

        notification_window.show().map_err(|e| e.to_string())?;
        notification_window
            .set_always_on_top(true)
            .map_err(|e| e.to_string())?;

        // GTK window managers may ignore positioning requests made before an
        // initially hidden window has been mapped. Position once immediately
        // after showing, then repeat shortly after the map event has settled.
        #[cfg(target_os = "linux")]
        if let Some(position) = target_position {
            if let Err(error) =
                notification_window.set_position(tauri::Position::Physical(position))
            {
                crate::app_log::warn(format!("通知窗口定位失败：{error}"));
            }

            let window = notification_window.clone();
            tauri::async_runtime::spawn(async move {
                sleep(Duration::from_millis(60)).await;
                if let Err(error) = window.set_position(tauri::Position::Physical(position)) {
                    crate::app_log::warn(format!("通知窗口二次定位失败：{error}"));
                }
            });
        }

        Ok(())
    }

    fn emit_queue_state(&self) {
        let state = self.queue.lock().unwrap().snapshot();
        let _ = self.app.emit("notification:queue-updated", state);
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

            if let Some(power) = app.try_state::<PowerMonitor>() {
                power.refresh();
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

            scheduler.show_next_notification();
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
                            queue_revision: 0,
                            notification_id: uuid::Uuid::new_v4().to_string(),
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

    #[test]
    fn a_new_notification_for_the_same_reminder_rejects_the_old_response() {
        let current = NotificationIdentity {
            reminder_id: "drink".into(),
            notification_id: "after-resume".into(),
        };
        assert!(current.matches("drink", "after-resume"));
        assert!(!current.matches("drink", "before-pause"));
        assert!(!current.matches("rest", "after-resume"));
    }

    #[test]
    fn queued_notifications_recheck_fullscreen_and_lock_state() {
        let db = Database::in_memory();
        let conn = db.conn.lock().unwrap();
        let now = Utc::now();
        assert!(!notifications_suppressed(&conn, now, false, false));
        assert!(notifications_suppressed(&conn, now, false, true));
        assert!(notifications_suppressed(&conn, now, true, false));
        // Leaving fullscreen permits the queued reminder on the next tick.
        assert!(!notifications_suppressed(&conn, now, false, false));
        conn.execute("INSERT OR REPLACE INTO settings (key, value) VALUES ('fullscreen_detection_enabled', 'false')", []).unwrap();
        assert!(!notifications_suppressed(&conn, now, false, true));
    }

    #[test]
    fn queued_notifications_recheck_manual_and_timed_dnd() {
        let db = Database::in_memory();
        let conn = db.conn.lock().unwrap();
        let now = Utc::now();
        crate::commands::pause_all_reminders(&conn).unwrap();
        assert!(notifications_suppressed(&conn, now, false, false));
        crate::commands::resume_all_reminders(&conn).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('temp_dnd_until', ?1)",
            [(now + ChronoDuration::minutes(30))
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string()],
        )
        .unwrap();
        assert!(notifications_suppressed(&conn, now, false, false));
        assert!(!notifications_suppressed(
            &conn,
            now + ChronoDuration::minutes(31),
            false,
            false
        ));
        conn.execute("DELETE FROM settings WHERE key = 'temp_dnd_until'", [])
            .unwrap();
        let start = (Local::now() - ChronoDuration::hours(1))
            .format("%H:%M")
            .to_string();
        let end = (Local::now() + ChronoDuration::hours(1))
            .format("%H:%M")
            .to_string();
        for (key, value) in [
            ("dnd_enabled", "true"),
            ("dnd_start", start.as_str()),
            ("dnd_end", end.as_str()),
        ] {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                (key, value),
            )
            .unwrap();
        }
        assert!(notifications_suppressed(&conn, now, false, false));
    }

    #[test]
    fn dispatch_rejects_disabled_deleted_and_rescheduled_reminders() {
        let db = Database::in_memory();
        let conn = db.conn.lock().unwrap();
        let now = Utc::now();
        conn.execute("INSERT INTO reminders (id, name, reminder_type, icon, message, interval_minutes, enabled, next_trigger, created_at, updated_at) VALUES ('test', 'test', 'drink', '', '', 20, 1, '2000-01-01T00:00:00', '', '')", []).unwrap();
        assert!(reminder_is_due(&conn, "test", now));
        conn.execute("UPDATE reminders SET enabled = 0", [])
            .unwrap();
        assert!(!reminder_is_due(&conn, "test", now));
        conn.execute(
            "UPDATE reminders SET enabled = 1, next_trigger = '2999-01-01T00:00:00'",
            [],
        )
        .unwrap();
        assert!(!reminder_is_due(&conn, "test", now));
        conn.execute("DELETE FROM reminders", []).unwrap();
        assert!(!reminder_is_due(&conn, "test", now));
    }

    #[test]
    fn notification_position_uses_target_monitor_physical_coordinates() {
        let position = notification_window_position(
            tauri::PhysicalPosition::new(1920, 0),
            tauri::PhysicalSize::new(2560, 1400),
            1.5,
        );

        assert_eq!(position, tauri::PhysicalPosition::new(3910, 920));
    }
}
