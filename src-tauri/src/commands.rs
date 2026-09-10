use crate::jellyfin::{self, Item, Library, Session};
use crate::player::{self, IpcHandles, SharedProgress, Track};
use crate::session_store;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_shell::process::CommandChild;

#[derive(Default)]
pub struct AppState {
    pub session: Mutex<Option<Session>>,
    pub playback: Mutex<Option<PlaybackHandle>>,
}

pub struct PlaybackHandle {
    pub host_hwnd: isize,
    pub mpv_child: CommandChild,
    pub ipc: IpcHandles,
    pub session: Session,
    pub item_id: String,
    pub play_session_id: String,
    pub progress: SharedProgress,
    pub reporter_task: tauri::async_runtime::JoinHandle<()>,
    pub pip: bool,
    /// The PiP window's size as of the last aspect-ratio correction (see
    /// `enforce_pip_aspect`) -- needed to tell, on the next `Resized` event,
    /// which dimension the user actually just dragged so that one can be
    /// preserved while the other is recomputed to match.
    pub pip_size: (i32, i32),
}

/// The video surface (and the HUD overlay window) always fill the main
/// window's entire client area -- see create_hud_window's doc comment for
/// why the HUD has to be a separate window at all. Client-relative, so
/// always (0, 0, width, height); resize_host_window converts to screen
/// coordinates internally.
fn full_client_rect(app: &AppHandle) -> Result<(i32, i32, i32, i32), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    let size = main.inner_size().map_err(|e| e.to_string())?;
    Ok((0, 0, size.width as i32, size.height as i32))
}

/// Just enough of the session for the frontend to build image/stream URLs
/// itself (see get_session_info's doc comment) -- not the full `Session`,
/// which also carries the user id and device id that aren't needed for that.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub server_url: String,
    pub access_token: String,
}

/// Every image on screen used to cost a full IPC round-trip to Rust just to
/// format a URL string with zero actual async work behind it -- on a page
/// with dozens of posters (the home page easily has 80+) that overhead adds
/// up to real, felt latency. The access token ending up in the frontend's
/// hands isn't a new exposure: every image URL already carried it as a
/// plain query parameter, visible in devtools regardless. Fetching this
/// once and building URLs locally afterward removes that per-image IPC hop
/// entirely.
#[tauri::command]
pub fn get_session_info(state: State<'_, AppState>) -> Result<SessionInfo, String> {
    let session = require_session(&state)?;
    Ok(SessionInfo {
        server_url: session.server_url,
        access_token: session.access_token,
    })
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
pub async fn get_latest_items(state: State<'_, AppState>, library_id: String) -> Result<Vec<Item>, String> {
    let session = require_session(&state)?;
    jellyfin::get_latest_items(&session, &library_id).await
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
pub async fn get_similar_items(state: State<'_, AppState>, item_id: String) -> Result<Vec<Item>, String> {
    let session = require_session(&state)?;
    jellyfin::get_similar_items(&session, &item_id).await
}

#[tauri::command]
pub async fn start_playback(
    app: AppHandle,
    state: State<'_, AppState>,
    item_id: String,
    start_seconds: f64,
) -> Result<(), String> {
    let session = require_session(&state)?;
    stop_playback_internal(&app, &state).await;

    let play_session_id = uuid::Uuid::new_v4().to_string();
    let stream_url = jellyfin::build_stream_url(&session, &item_id, &play_session_id);
    let (x, y, width, height) = full_client_rect(&app)?;
    let host_hwnd = player::create_host_window(&app, x, y, width, height)?;
    let pipe_name = format!(r"\\.\pipe\fastfin-mpv-{}", uuid::Uuid::new_v4());

    let mpv_child = match player::spawn_mpv(&app, host_hwnd, &pipe_name) {
        Ok(child) => child,
        Err(e) => {
            let _ = player::destroy_host_window(&app, host_hwnd);
            return Err(e);
        }
    };

    // host_hwnd is passed through only for diagnostic logging -- see
    // create_hud_window's doc comment for why it no longer needs to be used
    // for z-ordering.
    if let Err(e) = player::create_hud_window(&app, &item_id, host_hwnd) {
        let _ = mpv_child.kill();
        let _ = player::destroy_host_window(&app, host_hwnd);
        return Err(e);
    }

    // In PiP mode the HUD covers the video window completely and carries its
    // controls, including the drag handle and resize corners -- so it, not
    // the video window, is what actually receives drag/resize input. This
    // keeps the video glued to wherever/however big the HUD ends up, and
    // keeps the HUD itself locked to a fixed aspect ratio on resize.
    // Registered once here rather than re-wired on every `enter_pip` call;
    // it's a no-op (checks the live `pip` flag) whenever PiP isn't active.
    if let Some(hud) = app.get_webview_window("hud") {
        let app_for_sync = app.clone();
        hud.on_window_event(move |event| {
            if !matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
                return;
            }
            let state = app_for_sync.state::<AppState>();
            let mut playback = state.playback.lock().unwrap();
            let Some(handle) = playback.as_mut() else {
                return;
            };
            if !handle.pip {
                return;
            }
            if let WindowEvent::Resized(_) = event {
                if let Some(hud) = app_for_sync.get_webview_window("hud") {
                    if let Ok(size) = hud.inner_size() {
                        let current = (size.width as i32, size.height as i32);
                        let corrected = enforce_pip_aspect(current, handle.pip_size);
                        if corrected != current {
                            let _ = hud.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(
                                corrected.0.max(1) as u32,
                                corrected.1.max(1) as u32,
                            )));
                        }
                        handle.pip_size = corrected;
                    }
                }
            }
            player::sync_pip_from_hud(&app_for_sync, handle.host_hwnd);
        });
    }

    let progress: SharedProgress = std::sync::Arc::new(std::sync::Mutex::new(Default::default()));
    let pending_start = std::sync::Arc::new(std::sync::Mutex::new(
        if start_seconds > 0.0 { Some(start_seconds) } else { None },
    ));
    let ipc = player::connect_ipc(app.clone(), pipe_name, progress.clone(), pending_start).await?;
    player::load_file_via_ipc(&ipc.sender, &stream_url)?;

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
        ipc,
        session,
        item_id,
        play_session_id,
        progress,
        reporter_task,
        pip: false,
        pip_size: (0, 0),
    });

    Ok(())
}

/// Keeps the video surface and HUD overlay glued to `main`'s current client
/// rect. Called whenever `main` moves or resizes (including fullscreen
/// transitions) -- an owned window doesn't auto-follow its owner's geometry
/// on Windows, only its z-order and show/hide state, and the HUD's own size
/// no longer has any bearing on the video's rect at all (see
/// create_hud_window's doc comment), so this is the only place either one is
/// ever resized now.
///
/// Skipped entirely while PiP is active: `enter_pip` minimizing `main` fires
/// a `Resized` event of its own, and without this guard it would immediately
/// undo the PiP placement by snapping the video back to `main`'s (now
/// minimized) rect.
pub fn reposition_overlays(app: &AppHandle, state: &AppState) {
    let host_hwnd = {
        let playback = state.playback.lock().unwrap();
        playback.as_ref().and_then(|handle| {
            if handle.pip {
                return None;
            }
            if let Ok((x, y, width, height)) = full_client_rect(app) {
                let _ = player::resize_host_window(app, handle.host_hwnd, x, y, width, height);
            }
            Some(handle.host_hwnd)
        })
    };
    if let Some(host_hwnd) = host_hwnd {
        player::resize_hud_window(app, host_hwnd);
    }
}

async fn stop_playback_internal(app: &AppHandle, state: &State<'_, AppState>) {
    let handle = state.playback.lock().unwrap().take();
    if let Some(handle) = handle {
        handle.reporter_task.abort();
        let final_position = handle.progress.lock().unwrap().time_pos;
        // Fire-and-forget: this is a real network round-trip to Jellyfin,
        // and it doesn't need to block anything below it -- awaiting it
        // here meant the "back" button (which calls this via
        // go_back_to_item) sat there doing nothing, for the full round-trip
        // latency, before the app even started tearing down mpv or telling
        // the frontend to navigate.
        let session = handle.session.clone();
        let item_id = handle.item_id.clone();
        let play_session_id = handle.play_session_id.clone();
        tauri::async_runtime::spawn(async move {
            jellyfin::report_playback_stopped(&session, &item_id, &play_session_id, final_position).await;
        });
        let _ = player::ipc_command(&handle.ipc.sender, serde_json::json!(["quit"]));
        let _ = handle.mpv_child.kill();
        let _ = player::destroy_host_window(app, handle.host_hwnd);
    }
    player::destroy_hud_window(app);
}

#[tauri::command]
pub async fn stop_playback(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    stop_playback_internal(&app, &state).await;
    Ok(())
}

/// Stops playback, tears down the HUD window, and tells "main" to navigate
/// back to the item page. The HUD's "back" button has to go through a
/// command rather than calling SvelteKit's own `goto()` directly: the HUD is
/// a separate Tauri window with its own independent webview/router, so
/// navigating there would change what the (invisible, decoration-less) HUD
/// window itself is showing, not the main window the user actually sees.
#[tauri::command]
pub async fn go_back_to_item(app: AppHandle, state: State<'_, AppState>, item_id: String) -> Result<(), String> {
    stop_playback_internal(&app, &state).await;
    let _ = app.emit_to("main", "player://navigate-back", item_id);
    Ok(())
}

/// Snaps a just-resized PiP window back to a fixed 16:9 aspect ratio.
/// Compares `current` against `previous` (the size as of the last
/// correction) to guess which dimension the user actually just dragged --
/// whichever changed more -- and recomputes the other one to match, rather
/// than always trusting width or always trusting height. Corner-only resize
/// handles (see the HUD's `pip-resize-*` elements) mean both dimensions
/// normally move together anyway, so this mostly just keeps them
/// proportional rather than fighting a drag in one specific direction.
fn enforce_pip_aspect(current: (i32, i32), previous: (i32, i32)) -> (i32, i32) {
    const ASPECT: f64 = 16.0 / 9.0;
    let (width, height) = current;
    let (prev_width, prev_height) = previous;
    if (width - prev_width).abs() >= (height - prev_height).abs() {
        (width, (width as f64 / ASPECT).round() as i32)
    } else {
        ((height as f64 * ASPECT).round() as i32, height)
    }
}

/// Floats the video as a small always-on-top corner window, moves the HUD
/// overlay to sit exactly on top of it (now carrying compact PiP controls
/// instead of the full control bar -- see the HUD's own `pipMode`), and gets
/// `main` out of the way by minimizing it. There's no explicit "exit PiP"
/// command other than the HUD's own restore button, which just calls
/// `restore_from_pip` below -- both that and restoring `main` any other way
/// (taskbar, Alt+Tab, ...) are handled the same way, by `exit_pip_if_active`
/// reacting to `main`'s `Focused` event in `lib.rs`.
#[tauri::command]
pub fn enter_pip(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let host_hwnd = {
        let mut playback = state.playback.lock().unwrap();
        let handle = playback.as_mut().ok_or("No active playback")?;
        if handle.pip {
            return Ok(());
        }
        handle.pip = true;
        handle.host_hwnd
    };
    let (x, y, width, height) = player::enter_pip(host_hwnd);
    if let Some(handle) = state.playback.lock().unwrap().as_mut() {
        handle.pip_size = (width, height);
    }
    if let Some(hud) = app.get_webview_window("hud") {
        let _ = hud.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x, y)));
        let _ = hud.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(width as u32, height as u32)));
        // Normally fixed to whatever size `main` is (see reposition_overlays)
        // so there's nothing to manually resize -- but the PiP window is
        // free-floating, and its own corners are the resize handles (see the
        // HUD's `pip-resize-*` elements), which needs this set.
        let _ = hud.set_resizable(true);
        // Both the HUD and the video are now topmost; z-order between two
        // topmost windows is whichever was raised more recently, so
        // re-asserting this after the video's own SetWindowPos is what
        // guarantees the HUD (and its controls) end up on top, not buried
        // under the video.
        let _ = hud.set_always_on_top(false);
        let _ = hud.set_always_on_top(true);
        let _ = app.emit_to("hud", "player://pip-changed", true);
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.minimize();
    }
    Ok(())
}

/// The HUD's "return to FastFin" button in PiP mode. Just restores focus to
/// `main` -- the actual PiP teardown happens in `exit_pip_if_active`, which
/// that focus change triggers, so this stays a one-line wrapper instead of
/// duplicating that logic.
#[tauri::command]
pub fn restore_from_pip(app: AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    let _ = main.unminimize();
    let _ = main.set_focus();
    Ok(())
}

/// Called whenever `main` regains focus (see the `Focused` handling in
/// `lib.rs`) -- restoring the window is the only way out of PiP, so this is
/// the one place that needs to check for it.
pub fn exit_pip_if_active(app: &AppHandle) {
    let state = app.state::<AppState>();
    let host_hwnd = {
        let mut playback = state.playback.lock().unwrap();
        let Some(handle) = playback.as_mut() else {
            return;
        };
        if !handle.pip {
            return;
        }
        handle.pip = false;
        handle.host_hwnd
    };
    if let Err(e) = player::exit_pip(app, host_hwnd) {
        eprintln!("[pip] exit_pip failed: {e}");
        return;
    }
    if let Ok((x, y, width, height)) = full_client_rect(app) {
        let _ = player::resize_host_window(app, host_hwnd, x, y, width, height);
    }
    player::resize_hud_window(app, host_hwnd);
    if let Some(hud) = app.get_webview_window("hud") {
        let _ = hud.set_resizable(false);
        let _ = hud.show();
    }
    let _ = app.emit_to("hud", "player://pip-changed", false);
}

#[tauri::command]
pub fn toggle_main_fullscreen(app: AppHandle) -> Result<bool, String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    let is_fullscreen = main.is_fullscreen().map_err(|e| e.to_string())?;
    main.set_fullscreen(!is_fullscreen).map_err(|e| e.to_string())?;
    let _ = main.set_focus();
    Ok(!is_fullscreen)
}

#[tauri::command]
pub fn mpv_set_pause(state: State<'_, AppState>, paused: bool) -> Result<(), String> {
    let playback = state.playback.lock().unwrap();
    let handle = playback.as_ref().ok_or("No active playback")?;
    player::ipc_command(
        &handle.ipc.sender,
        serde_json::json!(["set_property", "pause", paused]),
    )
}

#[tauri::command]
pub fn mpv_seek(state: State<'_, AppState>, seconds: f64) -> Result<(), String> {
    let playback = state.playback.lock().unwrap();
    let handle = playback.as_ref().ok_or("No active playback")?;
    player::ipc_command(
        &handle.ipc.sender,
        serde_json::json!(["seek", seconds, "absolute"]),
    )
}

#[tauri::command]
pub fn mpv_set_volume(state: State<'_, AppState>, volume: f64) -> Result<(), String> {
    let playback = state.playback.lock().unwrap();
    let handle = playback.as_ref().ok_or("No active playback")?;
    player::ipc_command(
        &handle.ipc.sender,
        serde_json::json!(["set_property", "volume", volume]),
    )
}

#[tauri::command]
pub async fn get_tracks(state: State<'_, AppState>) -> Result<Vec<Track>, String> {
    eprintln!("[tracks] get_tracks command invoked");
    // Clone the (cheap, all-Arc) IPC handles out while holding the lock only
    // briefly: get_track_list needs to .await, which the std Mutex guard can't
    // cross.
    let ipc: IpcHandles = {
        let playback = state.playback.lock().unwrap();
        let handle = playback.as_ref().ok_or("No active playback")?;
        handle.ipc.clone()
    };
    player::get_track_list(&ipc).await
}

#[tauri::command]
pub fn mpv_set_subtitle_track(state: State<'_, AppState>, track_id: Option<i64>) -> Result<(), String> {
    let playback = state.playback.lock().unwrap();
    let handle = playback.as_ref().ok_or("No active playback")?;
    player::set_subtitle_track(&handle.ipc.sender, track_id)
}

#[tauri::command]
pub fn mpv_set_audio_track(state: State<'_, AppState>, track_id: i64) -> Result<(), String> {
    let playback = state.playback.lock().unwrap();
    let handle = playback.as_ref().ok_or("No active playback")?;
    player::set_audio_track(&handle.ipc.sender, track_id)
}
