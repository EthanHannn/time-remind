use chrono::Utc;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init(app: &AppHandle) {
    let Ok(log_dir) = app.path().app_data_dir() else {
        return;
    };

    if create_dir_all(&log_dir).is_err() {
        return;
    }

    let _ = LOG_PATH.set(log_dir.join("runtime.log"));
    info("应用日志初始化完成");
}

pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|value| format!("{}:{}", value.file(), value.line()))
            .unwrap_or_else(|| "unknown".to_string());
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|value| (*value).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic payload".to_string());

        error(format!("致命异常：{message}，位置：{location}"));
    }));
}

pub fn info(message: impl AsRef<str>) {
    write_line("INFO", message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    write_line("WARN", message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    write_line("ERROR", message.as_ref());
}

fn write_line(level: &str, message: &str) {
    let Some(path) = LOG_PATH.get() else {
        return;
    };

    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let line = format!("[{timestamp}] [{level}] {message}\n");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}
