#[cfg(target_os = "windows")]
pub fn is_foreground_window_fullscreen() -> bool {
    use std::mem::size_of;

    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Gdi::{
        ClientToScreen, GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetClientRect, GetDesktopWindow, GetForegroundWindow, GetShellWindow,
        GetWindowRect, GetWindowThreadProcessId, IsIconic,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return false;
        }
        // Explorer's desktop covers the monitor but is not a fullscreen app.
        if hwnd == GetDesktopWindow() || hwnd == GetShellWindow() {
            return false;
        }
        let mut class_name = [0u16; 256];
        let length = GetClassNameW(hwnd, &mut class_name);
        if length > 0 && is_desktop_class(&String::from_utf16_lossy(&class_name[..length as usize])) {
            return false;
        }

        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        if process_id == GetCurrentProcessId() {
            return false;
        }

        if IsIconic(hwnd).as_bool() {
            return false;
        }

        let mut window_rect = RECT::default();
        if GetWindowRect(hwnd, &mut window_rect).is_err() {
            return false;
        }

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if monitor.0.is_null() {
            return false;
        }

        let mut monitor_info = MONITORINFO::default();
        monitor_info.cbSize = size_of::<MONITORINFO>() as u32;
        if !GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
            return false;
        }

        let monitor_rect = monitor_info.rcMonitor;

        let mut client_rect = RECT::default();
        if GetClientRect(hwnd, &mut client_rect).is_err() {
            return false;
        }

        let mut client_top_left = POINT {
            x: client_rect.left,
            y: client_rect.top,
        };
        let mut client_bottom_right = POINT {
            x: client_rect.right,
            y: client_rect.bottom,
        };

        if !ClientToScreen(hwnd, &mut client_top_left).as_bool()
            || !ClientToScreen(hwnd, &mut client_bottom_right).as_bool()
        {
            return false;
        }

        bounds_cover_monitor(
            [window_rect.left, window_rect.top, window_rect.right, window_rect.bottom],
            [client_top_left.x, client_top_left.y, client_bottom_right.x, client_bottom_right.y],
            [monitor_rect.left, monitor_rect.top, monitor_rect.right, monitor_rect.bottom],
        )
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_foreground_window_fullscreen() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explorer_desktop_classes_are_excluded() {
        assert!(is_desktop_class("Progman"));
        assert!(is_desktop_class("WorkerW"));
        assert!(!is_desktop_class("Chrome_WidgetWin_1"));
        assert!(!is_desktop_class("ApplicationFrameWindow"));
    }

    #[test]
    fn fullscreen_supports_secondary_monitors_with_negative_coordinates() {
        let monitor = [-2560, -200, 0, 1240];
        assert!(bounds_cover_monitor(monitor, monitor, monitor));
        assert!(bounds_cover_monitor([-2568, -208, 8, 1248], monitor, monitor));
    }

    #[test]
    fn maximized_and_partial_windows_are_not_fullscreen() {
        let monitor = [0, 0, 1920, 1080];
        assert!(!bounds_cover_monitor([-8, -8, 1928, 1048], [0, 30, 1920, 1040], monitor));
        assert!(!bounds_cover_monitor(monitor, [0, 30, 1920, 1080], monitor));
        assert!(!bounds_cover_monitor([0, 0, 960, 1080], [0, 0, 960, 1080], monitor));
        assert!(!bounds_cover_monitor([i32::MIN; 4], [i32::MIN; 4], [i32::MAX; 4]));
    }
}
#[cfg(any(target_os = "windows", test))]
fn is_desktop_class(class_name: &str) -> bool {
    matches!(class_name, "Progman" | "WorkerW")
}

#[cfg(any(target_os = "windows", test))]
fn bounds_cover_monitor(window: [i32; 4], client: [i32; 4], monitor: [i32; 4]) -> bool {
    const TOLERANCE: i64 = 8;
    [window, client].iter().all(|bounds| bounds.iter().zip(monitor).all(|(edge, target)| {
        (i64::from(*edge) - i64::from(target)).abs() <= TOLERANCE
    }))
}
