use crate::jellyfin::Session;
use std::path::PathBuf;
use tauri::Manager;

fn session_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data directory: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create app data directory: {e}"))?;
    Ok(dir.join("session.json"))
}

pub fn save(app: &tauri::AppHandle, session: &Session) -> Result<(), String> {
    let path = session_file(app)?;
    let json = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| format!("Could not persist session: {e}"))
}

pub fn load(app: &tauri::AppHandle) -> Option<Session> {
    let path = session_file(app).ok()?;
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

pub fn clear(app: &tauri::AppHandle) -> Result<(), String> {
    let path = session_file(app)?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| format!("Could not clear session: {e}"))?;
    }
    Ok(())
}
