#[cfg(target_os = "windows")]
pub fn is_foreground_window_fullscreen() -> bool {
    use std::mem::size_of;

    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Gdi::{
        ClientToScreen, GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClientRect, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsIconic,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
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
        let tolerance = 8;

        let window_covers_monitor = (window_rect.left - monitor_rect.left).abs() <= tolerance
            && (window_rect.top - monitor_rect.top).abs() <= tolerance
            && (window_rect.right - monitor_rect.right).abs() <= tolerance
            && (window_rect.bottom - monitor_rect.bottom).abs() <= tolerance;

        if !window_covers_monitor {
            return false;
        }

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

        (client_top_left.x - monitor_rect.left).abs() <= tolerance
            && (client_top_left.y - monitor_rect.top).abs() <= tolerance
            && (client_bottom_right.x - monitor_rect.right).abs() <= tolerance
            && (client_bottom_right.y - monitor_rect.bottom).abs() <= tolerance
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_foreground_window_fullscreen() -> bool {
    false
}
