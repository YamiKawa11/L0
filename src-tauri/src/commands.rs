use tauri::State;
use std::sync::Arc;
use crate::storage::{Storage, Session};
use crate::session_manager::SessionManager;

#[tauri::command]
pub fn get_sessions(app_handle: tauri::AppHandle, search: Option<String>) -> Result<Vec<Session>, String> {
    let storage = Storage::new(&app_handle);
    storage.get_sessions(search).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_sessions(app_handle: tauri::AppHandle, ids: Vec<i64>) -> Result<(), String> {
    let storage = Storage::new(&app_handle);
    storage.delete_sessions(ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_setting(app_handle: tauri::AppHandle, manager: State<'_, Arc<SessionManager>>, key: String, value: String) -> Result<(), String> {
    let storage = Storage::new(&app_handle);
    storage.save_setting(&key, &value).map_err(|e| e.to_string())?;

    // Update session manager state
    let mut state = manager.state.lock();
    match key.as_str() {
        "session_timeout_seconds" => state.session_timeout = value.parse().unwrap_or(20),
        "min_session_length" => state.min_session_length = value.parse().unwrap_or(20),
        "enable_clipboard_capture" => state.capture_clipboard = value == "true",
        "enable_recording" => state.is_recording = value == "true",
        _ => {}
    }
    Ok(())
}

#[tauri::command]
pub fn get_setting(app_handle: tauri::AppHandle, key: String, default: String) -> String {
    let storage = Storage::new(&app_handle);
    storage.get_setting(&key, &default)
}

#[tauri::command]
pub fn toggle_recording(manager: State<'_, Arc<SessionManager>>, enabled: bool) -> Result<(), String> {
    let mut state = manager.state.lock();
    state.is_recording = enabled;
    Ok(())
}
