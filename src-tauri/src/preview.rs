use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRequest {
    mascot_style: String,
    reminder_type: String,
    settings: serde_json::Value,
}

#[derive(Default)]
pub struct PreviewState(Mutex<Option<PreviewRequest>>);

// Remember the latest request so the first click also works while the hidden
// preview webview is still loading. Preview never enters the reminder queue.
#[tauri::command]
pub fn get_notification_preview(state: State<'_, PreviewState>) -> Option<PreviewRequest> {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
pub fn preview_notification(
    app: AppHandle,
    state: State<'_, PreviewState>,
    request: PreviewRequest,
) -> Result<(), String> {
    if !matches!(
        request.mascot_style.as_str(),
        "classic" | "editorial" | "watercolor"
    ) || !matches!(
        request.reminder_type.as_str(),
        "drink" | "rest" | "eye_care"
    ) {
        return Err("Unknown preview style or reminder type".into());
    }
    let window = app
        .get_webview_window("notification-preview")
        .ok_or("Preview window unavailable")?;
    *state.0.lock().unwrap() = Some(request.clone());
    window
        .emit("notification:preview", request)
        .map_err(|e| e.to_string())?;

    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|position| {
            app.monitor_from_point(position.x, position.y)
                .ok()
                .flatten()
        })
        .or_else(|| app.primary_monitor().ok().flatten());
    let position = monitor.map(|monitor| {
        let area = monitor.work_area();
        crate::scheduler::notification_window_position(
            area.position,
            area.size,
            monitor.scale_factor(),
        )
    });
    if let Some(position) = position {
        window
            .set_position(tauri::Position::Physical(position))
            .map_err(|e| e.to_string())?;
    }
    window.show().map_err(|e| e.to_string())?;
    // Repeat after mapping for window managers that ignore hidden positioning.
    if let Some(position) = position {
        window
            .set_position(tauri::Position::Physical(position))
            .map_err(|e| e.to_string())?;
    }
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}
