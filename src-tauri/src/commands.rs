use crate::jellyfin::{self, Item, Library, Session};
use crate::player::{self, SharedProgress};
use crate::session_store;
use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_shell::process::CommandChild;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Default)]
pub struct AppState {
    pub session: Mutex<Option<Session>>,
    pub playback: Mutex<Option<PlaybackHandle>>,
}

pub struct PlaybackHandle {
    pub host_hwnd: isize,
    pub mpv_child: CommandChild,
    pub ipc_sender: UnboundedSender<String>,
    pub last_rect: (i32, i32, i32, i32),
    pub session: Session,
    pub item_id: String,
    pub play_session_id: String,
    pub progress: SharedProgress,
    pub reporter_task: tauri::async_runtime::JoinHandle<()>,
}

fn require_session(state: &State<AppState>) -> Result<Session, String> {
    state
        .session
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "Not logged in".to_string())
}

#[tauri::command]
pub async fn login(
    app: AppHandle,
    state: State<'_, AppState>,
    server_url: String,
    username: String,
    password: String,
) -> Result<(), String> {
    let device_id = uuid::Uuid::new_v4().to_string();
    let session = jellyfin::authenticate(&server_url, &username, &password, &device_id).await?;

    session_store::save(&app, &session)?;
    *state.session.lock().unwrap() = Some(session);
    Ok(())
}

#[tauri::command]
pub fn try_restore_session(app: AppHandle, state: State<'_, AppState>) -> bool {
    match session_store::load(&app) {
        Some(session) => {
            *state.session.lock().unwrap() = Some(session);
            true
        }
        None => false,
    }
}

#[tauri::command]
pub fn logout(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    *state.session.lock().unwrap() = None;
    session_store::clear(&app)
}

#[tauri::command]
pub async fn get_libraries(state: State<'_, AppState>) -> Result<Vec<Library>, String> {
    let session = require_session(&state)?;
    jellyfin::get_libraries(&session).await
}

#[tauri::command]
pub async fn get_items(state: State<'_, AppState>, library_id: String) -> Result<Vec<Item>, String> {
    let session = require_session(&state)?;
    jellyfin::get_items(&session, &library_id).await
}

#[tauri::command]
pub async fn get_item(state: State<'_, AppState>, item_id: String) -> Result<Item, String> {
    let session = require_session(&state)?;
    jellyfin::get_item(&session, &item_id).await
}

#[tauri::command]
pub async fn get_seasons(state: State<'_, AppState>, series_id: String) -> Result<Vec<Item>, String> {
    let session = require_session(&state)?;
    jellyfin::get_seasons(&session, &series_id).await
}

#[tauri::command]
pub async fn get_episodes(
    state: State<'_, AppState>,
    series_id: String,
    season_id: String,
) -> Result<Vec<Item>, String> {
    let session = require_session(&state)?;
    jellyfin::get_episodes(&session, &series_id, &season_id).await
}

#[tauri::command]
pub async fn get_playlists(state: State<'_, AppState>) -> Result<Vec<Item>, String> {
    let session = require_session(&state)?;
    jellyfin::get_playlists(&session).await
}

#[tauri::command]
pub async fn get_resume(state: State<'_, AppState>) -> Result<Vec<Item>, String> {
    let session = require_session(&state)?;
    jellyfin::get_resume(&session).await
}

#[tauri::command]
pub fn get_image_url(state: State<'_, AppState>, item_id: String) -> Result<String, String> {
    let session = require_session(&state)?;
    Ok(jellyfin::image_url(&session, &item_id))
}

#[tauri::command]
pub fn get_backdrop_url(state: State<'_, AppState>, item_id: String) -> Result<String, String> {
    let session = require_session(&state)?;
    Ok(jellyfin::backdrop_url(&session, &item_id))
}

#[tauri::command]
pub async fn start_playback(
    app: AppHandle,
    state: State<'_, AppState>,
    item_id: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    start_seconds: f64,
) -> Result<(), String> {
    let session = require_session(&state)?;
    stop_playback_internal(&app, &state).await;

    let play_session_id = uuid::Uuid::new_v4().to_string();
    let stream_url = jellyfin::build_stream_url(&session, &item_id, &play_session_id);
    let host_hwnd = player::create_host_window(&app, x, y, width, height)?;
    let pipe_name = format!(r"\\.\pipe\saucefin-mpv-{}", uuid::Uuid::new_v4());

    let mpv_child = match player::spawn_mpv(&app, host_hwnd, &pipe_name, &stream_url, start_seconds) {
        Ok(child) => child,
        Err(e) => {
            let _ = player::destroy_host_window(&app, host_hwnd);
            return Err(e);
        }
    };

    let progress: SharedProgress = std::sync::Arc::new(std::sync::Mutex::new(Default::default()));
    let ipc_sender = player::connect_ipc(app.clone(), pipe_name, progress.clone()).await?;

    jellyfin::report_playback_start(&session, &item_id, &play_session_id, start_seconds).await;

    let reporter_task = player::spawn_progress_reporter(
        session.clone(),
        item_id.clone(),
        play_session_id.clone(),
        progress.clone(),
    );

    *state.playback.lock().unwrap() = Some(PlaybackHandle {
        host_hwnd,
        mpv_child,
        ipc_sender,
        last_rect: (x, y, width, height),
        session,
        item_id,
        play_session_id,
        progress,
        reporter_task,
    });

    Ok(())
}

#[tauri::command]
pub fn resize_video_surface(
    app: AppHandle,
    state: State<'_, AppState>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<(), String> {
    let mut playback = state.playback.lock().unwrap();
    match playback.as_mut() {
        Some(handle) => {
            handle.last_rect = (x, y, width, height);
            player::resize_host_window(&app, handle.host_hwnd, x, y, width, height)
        }
        None => Ok(()),
    }
}

/// Re-applies the last known rect to the popup window, converting through the
/// owner window's *current* screen position. Called when the main window
/// moves without resizing (drag), which wouldn't otherwise trigger the
/// frontend's ResizeObserver -- an owned popup window doesn't auto-follow its
/// owner's position, only its z-order and show/hide state.
pub fn reposition_for_owner_move(app: &AppHandle, state: &AppState) {
    let playback = state.playback.lock().unwrap();
    if let Some(handle) = playback.as_ref() {
        let (x, y, width, height) = handle.last_rect;
        let _ = player::resize_host_window(app, handle.host_hwnd, x, y, width, height);
    }
}

async fn stop_playback_internal(app: &AppHandle, state: &State<'_, AppState>) {
    let handle = state.playback.lock().unwrap().take();
    if let Some(handle) = handle {
        handle.reporter_task.abort();
        let final_position = handle.progress.lock().unwrap().time_pos;
        jellyfin::report_playback_stopped(&handle.session, &handle.item_id, &handle.play_session_id, final_position)
            .await;
        let _ = player::ipc_command(&handle.ipc_sender, serde_json::json!(["quit"]));
        let _ = handle.mpv_child.kill();
        let _ = player::destroy_host_window(app, handle.host_hwnd);
    }
}

#[tauri::command]
pub async fn stop_playback(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    stop_playback_internal(&app, &state).await;
    Ok(())
}

#[tauri::command]
pub fn mpv_set_pause(state: State<'_, AppState>, paused: bool) -> Result<(), String> {
    let playback = state.playback.lock().unwrap();
    let handle = playback.as_ref().ok_or("No active playback")?;
    player::ipc_command(
        &handle.ipc_sender,
        serde_json::json!(["set_property", "pause", paused]),
    )
}

#[tauri::command]
pub fn mpv_seek(state: State<'_, AppState>, seconds: f64) -> Result<(), String> {
    let playback = state.playback.lock().unwrap();
    let handle = playback.as_ref().ok_or("No active playback")?;
    player::ipc_command(
        &handle.ipc_sender,
        serde_json::json!(["seek", seconds, "absolute"]),
    )
}

#[tauri::command]
pub fn mpv_set_volume(state: State<'_, AppState>, volume: f64) -> Result<(), String> {
    let playback = state.playback.lock().unwrap();
    let handle = playback.as_ref().ok_or("No active playback")?;
    player::ipc_command(
        &handle.ipc_sender,
        serde_json::json!(["set_property", "volume", volume]),
    )
}
