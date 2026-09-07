use std::sync::mpsc;
use std::sync::{Arc, Once};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::ClientOptions;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{ClientToScreen, GetStockObject, BLACK_BRUSH, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassExW, SetWindowPos,
    SWP_NOACTIVATE, SWP_NOZORDER, WNDCLASSEXW, WNDCLASS_STYLES, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};

const CLASS_NAME: &str = "SaucefinMpvHost";
static REGISTER_CLASS: Once = Once::new();

/// Latest known playback position, kept up to date from mpv's observed-property
/// IPC events so a periodic reporter task can push progress to Jellyfin
/// without needing its own round-trip to mpv.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlaybackProgress {
    pub time_pos: f64,
    pub paused: bool,
}

pub type SharedProgress = Arc<std::sync::Mutex<PlaybackProgress>>;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn register_class_once() {
    REGISTER_CLASS.call_once(|| unsafe {
        let class_name = to_wide(CLASS_NAME);
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW failed");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        // SAFETY: class registered once, name/proc live for program lifetime.
        RegisterClassExW(&wc);
    });
}

/// Converts a point relative to `hwnd`'s client area into screen coordinates.
fn client_to_screen(hwnd: HWND, x: i32, y: i32) -> (i32, i32) {
    let mut point = POINT { x, y };
    unsafe {
        let _ = ClientToScreen(hwnd, &mut point);
    }
    (point.x, point.y)
}

/// Returns the main window's HWND value as a raw `isize`, not `windows::Win32::Foundation::HWND`:
/// Tauri's `Window::hwnd()` returns that type from ITS OWN transitive `windows` crate
/// dependency, which can be (and here is) a different major version than the one this
/// crate depends on directly -- the two `HWND` newtypes don't unify even though they're
/// structurally identical. Converting to `isize` at the boundary sidesteps that.
fn get_parent_hwnd(app: &AppHandle) -> Result<isize, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    Ok(hwnd.0 as isize)
}

/// Creates a native Win32 window that mpv will attach to via `--wid`.
///
/// This is an OWNED TOP-LEVEL window (`WS_POPUP` with the main Tauri window
/// as owner), not a child window. WebView2 uses DirectComposition internally,
/// which paints over sibling child windows regardless of normal Win32 z-order
/// ("airspace" problem) -- mpv's video renders successfully but is invisible,
/// hidden behind the webview's own composited surface. An owned top-level
/// window is composited by DWM at the OS level instead, which correctly keeps
/// it glued above its owner. Coordinates passed in are relative to the main
/// window's client area (as measured by the frontend); they're converted to
/// screen coordinates here since popup windows position in screen space.
pub fn create_host_window(app: &AppHandle, x: i32, y: i32, width: i32, height: i32) -> Result<isize, String> {
    let parent_val = get_parent_hwnd(app)?;

    let (tx, rx) = mpsc::channel::<Result<isize, String>>();
    app.run_on_main_thread(move || {
        register_class_once();
        let class_name = to_wide(CLASS_NAME);
        let hinstance = match unsafe { GetModuleHandleW(None) } {
            Ok(h) => h,
            Err(e) => {
                let _ = tx.send(Err(e.to_string()));
                return;
            }
        };
        let parent = HWND(parent_val as *mut std::ffi::c_void);
        let (screen_x, screen_y) = client_to_screen(parent, x, y);
        let result = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                PCWSTR(class_name.as_ptr()),
                PCWSTR::null(),
                WS_POPUP | WS_VISIBLE,
                screen_x,
                screen_y,
                width,
                height,
                Some(parent),
                None,
                Some(hinstance.into()),
                None,
            )
        };
        let mapped = result.map(|h| h.0 as isize).map_err(|e| e.to_string());
        let _ = tx.send(mapped);
    })
    .map_err(|e| e.to_string())?;

    rx.recv().map_err(|e| e.to_string())?
}

/// Repositions/resizes the owned popup window. `x`/`y` are relative to the
/// main window's client area (same convention as `create_host_window`) and
/// are converted to screen coordinates using the current owner window
/// position, so this stays correct if the window has moved since creation.
pub fn resize_host_window(app: &AppHandle, hwnd_val: isize, x: i32, y: i32, width: i32, height: i32) -> Result<(), String> {
    let parent_val = get_parent_hwnd(app)?;
    let parent_hwnd = HWND(parent_val as *mut std::ffi::c_void);
    let (screen_x, screen_y) = client_to_screen(parent_hwnd, x, y);
    let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);
    unsafe {
        SetWindowPos(hwnd, None, screen_x, screen_y, width, height, SWP_NOZORDER | SWP_NOACTIVATE)
            .map_err(|e| e.to_string())
    }
}

pub fn destroy_host_window(app: &AppHandle, hwnd_val: isize) -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    app.run_on_main_thread(move || {
        let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);
        let result = unsafe { DestroyWindow(hwnd) }.map_err(|e| e.to_string());
        let _ = tx.send(result);
    })
    .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}

/// Spawns the mpv sidecar attached to `hwnd_val` via `--wid`, playing `stream_url`.
///
/// The HWND MUST be cast to `u32` (not a signed integer) per mpv's own documented
/// win32 embedding convention -- a signed cast can make the value negative, which
/// mpv silently rejects by detaching into its own top-level window instead of
/// embedding into ours.
pub fn spawn_mpv(
    app: &AppHandle,
    hwnd_val: isize,
    pipe_name: &str,
    stream_url: &str,
    start_seconds: f64,
) -> Result<tauri_plugin_shell::process::CommandChild, String> {
    let hwnd_u32 = hwnd_val as u32;

    let sidecar = app
        .shell()
        .sidecar("mpv")
        .map_err(|e| format!("Could not resolve mpv sidecar: {e}"))?;

    let mut args = vec![
        format!("--wid={}", hwnd_u32),
        format!("--input-ipc-server={}", pipe_name),
        "--no-input-default-bindings".to_string(),
        "--no-osc".to_string(),
        "--keep-open=yes".to_string(),
        // Query the actual display's current colorspace/HDR state (via DXGI)
        // and match the swapchain to it, instead of always presenting SDR and
        // tone-mapping HDR content down -- this is the entire reason mpv is
        // used here instead of a browser <video> tag, so it must be explicit.
        "--target-colorspace-hint=yes".to_string(),
        "-v".to_string(),
    ];
    if start_seconds > 0.0 {
        args.push(format!("--start={start_seconds}"));
    }
    args.push(stream_url.to_string());

    let (mut rx, child) = sidecar
        .args(args)
        .spawn()
        .map_err(|e| format!("Failed to spawn mpv: {e}"))?;

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stderr(line) => {
                    let text = String::from_utf8_lossy(&line).to_string();
                    eprintln!("[mpv] {text}");
                    let _ = app_handle.emit("mpv://log", text);
                }
                CommandEvent::Stdout(line) => {
                    eprintln!("[mpv] {}", String::from_utf8_lossy(&line));
                }
                CommandEvent::Error(err) => {
                    eprintln!("[mpv] process error: {err}");
                }
                CommandEvent::Terminated(payload) => {
                    eprintln!("[mpv] terminated: {payload:?}");
                }
                _ => {}
            }
        }
    });

    Ok(child)
}

/// Connects to mpv's JSON IPC named pipe, retrying briefly since mpv needs a
/// moment after spawn to create it. Spawns a background task that forwards
/// observed-property events (time-pos, duration, pause) to the frontend and
/// updates `progress`, and returns a sender for writing commands
/// (pause/seek/volume) to mpv.
pub async fn connect_ipc(
    app: AppHandle,
    pipe_name: String,
    progress: SharedProgress,
) -> Result<tokio::sync::mpsc::UnboundedSender<String>, String> {
    let mut last_err = String::new();
    let mut client = None;
    for _ in 0..50 {
        match ClientOptions::new().open(&pipe_name) {
            Ok(c) => {
                client = Some(c);
                break;
            }
            Err(e) => {
                last_err = e.to_string();
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
    let pipe = client.ok_or_else(|| format!("Could not connect to mpv IPC pipe: {last_err}"))?;
    let (reader_half, mut writer_half) = tokio::io::split(pipe);

    for prop in ["time-pos", "duration", "pause"] {
        let cmd = serde_json::json!({ "command": ["observe_property", 1, prop] });
        let _ = writer_half.write_all(format!("{}\n", cmd).as_bytes()).await;
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    tauri::async_runtime::spawn(async move {
        while let Some(line) = rx.recv().await {
            if writer_half.write_all(format!("{}\n", line).as_bytes()).await.is_err() {
                break;
            }
        }
    });

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(reader_half).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                if let (Some(name), Some(data)) = (value.get("name"), value.get("data")) {
                    if let Ok(mut p) = progress.lock() {
                        match name.as_str() {
                            Some("time-pos") => {
                                if let Some(v) = data.as_f64() {
                                    p.time_pos = v;
                                }
                            }
                            Some("pause") => {
                                if let Some(v) = data.as_bool() {
                                    p.paused = v;
                                }
                            }
                            _ => {}
                        }
                    }
                    let _ = app_handle.emit(
                        "mpv://property-change",
                        serde_json::json!({ "name": name, "value": data }),
                    );
                }
            }
        }
    });

    Ok(tx)
}

/// Spawns a task that periodically reports the current playback position to
/// Jellyfin (so "continue watching" / resume works), roughly matching the
/// cadence official clients use. Returns the task handle so it can be
/// aborted when playback stops (a final report is sent separately at that
/// point with the exact stop position, not just the last periodic sample).
pub fn spawn_progress_reporter(
    session: crate::jellyfin::Session,
    item_id: String,
    play_session_id: String,
    progress: SharedProgress,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let (time_pos, paused) = {
                let p = progress.lock().unwrap();
                (p.time_pos, p.paused)
            };
            crate::jellyfin::report_playback_progress(&session, &item_id, &play_session_id, time_pos, paused).await;
        }
    })
}

pub fn ipc_command(sender: &tokio::sync::mpsc::UnboundedSender<String>, command: serde_json::Value) -> Result<(), String> {
    let payload = serde_json::json!({ "command": command });
    sender
        .send(payload.to_string())
        .map_err(|e| format!("mpv IPC channel closed: {e}"))
}
