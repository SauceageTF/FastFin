mod commands;
mod jellyfin;
mod player;
mod session_store;

use commands::AppState;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            if let Some(window) = app.get_webview_window("main") {
                window.on_window_event(move |event| {
                    // The mpv video surface is a window owned by "main" (see
                    // create_host_window's doc comment) -- owned windows
                    // don't auto-follow their owner's geometry, so this has
                    // to be driven explicitly on every move and resize,
                    // including the resize a fullscreen toggle causes. The
                    // HUD overlay isn't owned (see create_hud_window's doc
                    // comment for why), but still needs the same tracking
                    // since nothing else keeps it glued to "main" either.
                    if matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
                        let state = handle.state::<AppState>();
                        commands::reposition_overlays(&handle, &state);
                    }
                    // The HUD is `always_on_top` rather than owned, which
                    // means it would otherwise keep drawing over *other
                    // apps* once "main" loses focus -- hide it whenever
                    // that happens and bring it back when "main" regains
                    // focus, so it only ever floats above this app's own
                    // windows. Skipped entirely while PiP is active: "main"
                    // is deliberately minimized (and thus unfocused) for the
                    // whole time PiP is up, but the HUD is exactly what's
                    // carrying the floating video's controls then, so it
                    // must stay visible regardless.
                    // Playback isn't torn down anywhere on its own when
                    // `main` closes: mpv is a detached sidecar process, and
                    // the HUD overlay is `always_on_top` rather than owned by
                    // `main` (see create_hud_window's doc comment for why),
                    // so neither one goes away on its own when `main` does.
                    // Left alone, closing the app mid-playback left mpv still
                    // running and the HUD still floating on screen.
                    if let WindowEvent::CloseRequested { .. } = event {
                        let state = handle.state::<AppState>();
                        commands::cleanup_playback(&handle, &state);
                    }
                    if let WindowEvent::Focused(focused) = event {
                        let state = handle.state::<AppState>();
                        let pip_active = state
                            .playback
                            .lock()
                            .unwrap()
                            .as_ref()
                            .is_some_and(|p| p.pip);
                        if !pip_active {
                            if let Some(hud) = handle.get_webview_window("hud") {
                                let _ = if *focused { hud.show() } else { hud.hide() };
                            }
                        }
                        // Restoring "main" (taskbar, Alt+Tab, ...) is the
                        // only way out of PiP mode -- see enter_pip's doc
                        // comment -- so this is the one place that needs to
                        // check for it.
                        if *focused {
                            commands::exit_pip_if_active(&handle);
                        }
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::login,
            commands::try_restore_session,
            commands::logout,
            commands::get_libraries,
            commands::get_items,
            commands::get_latest_items,
            commands::get_item,
            commands::get_seasons,
            commands::get_episodes,
            commands::get_resume,
            commands::get_playlists,
            commands::get_session_info,
            commands::get_similar_items,
            commands::start_playback,
            commands::stop_playback,
            commands::go_back_to_item,
            commands::enter_pip,
            commands::restore_from_pip,
            commands::toggle_main_fullscreen,
            commands::mpv_set_pause,
            commands::mpv_seek,
            commands::mpv_set_volume,
            commands::get_tracks,
            commands::mpv_set_subtitle_track,
            commands::mpv_set_audio_track,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
