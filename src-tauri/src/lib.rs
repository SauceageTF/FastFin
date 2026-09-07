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
                    if let WindowEvent::Moved(_) = event {
                        let state = handle.state::<AppState>();
                        commands::reposition_for_owner_move(&handle, &state);
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
            commands::get_item,
            commands::get_seasons,
            commands::get_episodes,
            commands::get_resume,
            commands::get_playlists,
            commands::get_image_url,
            commands::get_backdrop_url,
            commands::start_playback,
            commands::resize_video_surface,
            commands::stop_playback,
            commands::mpv_set_pause,
            commands::mpv_seek,
            commands::mpv_set_volume,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
