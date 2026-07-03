use serde::{Deserialize, Serialize};

/// 提醒类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ReminderType {
    Drink,
    Rest,
    EyeCare,
    Custom,
}

#[allow(dead_code)]
impl ReminderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Drink => "drink",
            Self::Rest => "rest",
            Self::EyeCare => "eye_care",
            Self::Custom => "custom",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "drink" => Self::Drink,
            "rest" => Self::Rest,
            "eye_care" => Self::EyeCare,
            _ => Self::Custom,
        }
    }
}

/// 提醒响应动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ResponseAction {
    Completed,
    Postponed,
    Skipped,
    Timeout,
}

#[allow(dead_code)]
impl ResponseAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Postponed => "postponed",
            Self::Skipped => "skipped",
            Self::Timeout => "timeout",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "completed" => Self::Completed,
            "postponed" => Self::Postponed,
            "skipped" => Self::Skipped,
            _ => Self::Timeout,
        }
    }
}

/// 提醒配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub name: String,
    pub reminder_type: String,
    pub icon: String,
    pub message: String,
    pub interval_minutes: i64,
    #[serde(default)]
    pub break_duration_minutes: i64,
    #[serde(default)]
    pub break_notification_enabled: bool,
    #[serde(default)]
    pub action_enabled: bool,
    #[serde(default)]
    pub action_title: String,
    #[serde(default)]
    pub action_message: String,
    #[serde(default)]
    pub action_duration_seconds: i64,
    #[serde(default = "default_action_completion_mode")]
    pub action_completion_mode: String,
    pub enabled: bool,
    pub next_trigger: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 提醒日志
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ReminderLog {
    pub id: String,
    pub reminder_id: String,
    pub action: String,
    pub triggered_at: String,
    pub responded_at: Option<String>,
}

/// 创建提醒请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReminderRequest {
    pub name: String,
    pub reminder_type: String,
    pub icon: String,
    pub message: String,
    pub interval_minutes: i64,
    pub break_duration_minutes: Option<i64>,
    pub break_notification_enabled: Option<bool>,
    pub action_enabled: Option<bool>,
    pub action_title: Option<String>,
    pub action_message: Option<String>,
    pub action_duration_seconds: Option<i64>,
    pub action_completion_mode: Option<String>,
    pub enabled: Option<bool>,
}

/// 更新提醒请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReminderRequest {
    pub name: Option<String>,
    pub reminder_type: Option<String>,
    pub icon: Option<String>,
    pub message: Option<String>,
    pub interval_minutes: Option<i64>,
    pub break_duration_minutes: Option<i64>,
    pub break_notification_enabled: Option<bool>,
    pub action_enabled: Option<bool>,
    pub action_title: Option<String>,
    pub action_message: Option<String>,
    pub action_duration_seconds: Option<i64>,
    pub action_completion_mode: Option<String>,
    pub enabled: Option<bool>,
}

fn default_action_completion_mode() -> String {
    "auto".to_string()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub platform: String,
    pub is_verified_release_platform: bool,
    pub supports_fullscreen_detection: bool,
    pub supports_lock_detection: bool,
    pub supports_tray: bool,
    pub supports_autostart: bool,
    pub supports_silent_start: bool,
}

impl PlatformCapabilities {
    pub fn current() -> Self {
        let is_windows = cfg!(target_os = "windows");

        Self {
            platform: current_platform().to_string(),
            is_verified_release_platform: is_windows,
            supports_fullscreen_detection: is_windows,
            supports_lock_detection: is_windows,
            supports_tray: is_windows,
            supports_autostart: is_windows,
            supports_silent_start: is_windows,
        }
    }
}

fn current_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::PlatformCapabilities;

    #[test]
    fn platform_capabilities_follow_verified_platform_policy() {
        let capabilities = PlatformCapabilities::current();
        let is_windows = cfg!(target_os = "windows");

        assert_eq!(capabilities.is_verified_release_platform, is_windows);
        assert_eq!(capabilities.supports_fullscreen_detection, is_windows);
        assert_eq!(capabilities.supports_lock_detection, is_windows);
        assert_eq!(capabilities.supports_tray, is_windows);
        assert_eq!(capabilities.supports_autostart, is_windows);
        assert_eq!(capabilities.supports_silent_start, is_windows);
    }
}
