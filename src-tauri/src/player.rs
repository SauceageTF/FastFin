use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
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
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetSystemMetrics, RegisterClassExW,
    SetWindowLongPtrW, SetWindowPos, GWLP_HWNDPARENT, HWND_NOTOPMOST, HWND_TOPMOST,
    SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SWP_SHOWWINDOW, WNDCLASSEXW, WNDCLASS_STYLES, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_POPUP, WS_VISIBLE,
};

const CLASS_NAME: &str = "FastFinMpvHost";
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Track {
    pub id: i64,
    #[serde(rename = "type")]
    pub track_type: String,
    pub lang: Option<String>,
    pub title: Option<String>,
    pub selected: bool,
}

type PendingRequests = Arc<std::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>>>;

/// Handles for talking to mpv over its JSON IPC pipe: a fire-and-forget sender
/// for commands (pause/seek/volume/loadfile), plus a request/response layer
/// (`pending` + `next_id`) for commands that need mpv's actual reply, like
/// reading the track list. Cheap to clone -- every field is a shared handle.
#[derive(Clone)]
pub struct IpcHandles {
    pub sender: tokio::sync::mpsc::UnboundedSender<String>,
    pending: PendingRequests,
    next_id: Arc<AtomicU64>,
}

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

/// Creates the always-on-top, transparent overlay window that hosts the HUD
/// (top bar + playback controls). It's a real Tauri window (its own WebView2
/// instance), not a raw win32 window like mpv's -- it needs to render actual
/// HTML.
///
/// `owner()` turned out not to reliably guarantee this window actually
/// receives visible rendering/input above mpv's popup in practice (mpv's own
/// popup is *also* owned by `main`, and the two owned siblings' relative
/// z-order isn't a hard guarantee the way plain `always_on_top` is) -- so
/// this uses `always_on_top` instead, which unconditionally draws above
/// every non-topmost window, mpv's popup included, with no z-order race to
/// reason about at all. The tradeoff `always_on_top` normally has --
/// drawing over *other apps* too -- is handled separately by hiding this
/// window whenever `main` isn't the focused window (see the `Focused` event
/// handling in `lib.rs`).
pub fn create_hud_window(app: &AppHandle, item_id: &str, host_hwnd: isize) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    let position = main.inner_position().map_err(|e| e.to_string())?;
    let size = main.inner_size().map_err(|e| e.to_string())?;

    eprintln!("[hud] main position={position:?} size={size:?} host_hwnd={host_hwnd}");

    let url = format!("player/{item_id}/hud");
    let hud = tauri::WebviewWindowBuilder::new(app, "hud", tauri::WebviewUrl::App(url.into()))
        .always_on_top(true)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        // `focused(false)` only stops this window from taking focus when it's
        // first shown -- it's still activatable, so any click on its chrome
        // (pause, seek, ...) would activate it and deactivate `main`, whose
        // `Focused(false)` handler in lib.rs then hides this window. Nothing
        // could bring it back either: the mouse then lands on mpv's host
        // window, which is WS_EX_NOACTIVATE, so `main` never regains focus
        // until an Alt+Tab. Non-focusable (WS_EX_NOACTIVATE) still receives
        // mouse input fine; it just never steals activation from `main`.
        .focusable(false)
        .visible(true)
        .build()
        .map_err(|e| e.to_string())?;

    eprintln!("[hud] built ok, is_visible={:?}", hud.is_visible());

    // The builder's own position/size setters take logical pixels; setting
    // physical values post-build instead sidesteps needing to do our own
    // DPI conversion to match `main`'s actual client rect exactly.
    if let Err(e) = hud.set_position(tauri::Position::Physical(position)) {
        eprintln!("[hud] set_position failed: {e}");
    }
    if let Err(e) = hud.set_size(tauri::Size::Physical(size)) {
        eprintln!("[hud] set_size failed: {e}");
    }

    eprintln!(
        "[hud] after set: inner_position={:?} inner_size={:?}",
        hud.inner_position(),
        hud.inner_size()
    );
    Ok(())
}

/// Keeps the HUD window glued to `main`'s current client rect. Called
/// whenever `main` moves or resizes, including fullscreen transitions --
/// even an always-on-top window doesn't automatically track another
/// window's geometry.
pub fn resize_hud_window(app: &AppHandle, _host_hwnd: isize) {
    let (Some(main), Some(hud)) = (app.get_webview_window("main"), app.get_webview_window("hud")) else {
        return;
    };
    if let (Ok(position), Ok(size)) = (main.inner_position(), main.inner_size()) {
        let _ = hud.set_position(tauri::Position::Physical(position));
        let _ = hud.set_size(tauri::Size::Physical(size));
    }
}

pub fn destroy_hud_window(app: &AppHandle) {
    if let Some(hud) = app.get_webview_window("hud") {
        let _ = hud.close();
    }
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

/// Small floating rect anchored to the bottom-right corner of the primary
/// monitor, sized to a fixed 16:9 box. Multi-monitor placement and DPI
/// awareness aren't worth the complexity here -- this is meant to be a
/// minimal "keep the video visible while I do something else" surface, not a
/// fully general floating window.
fn pip_rect() -> (i32, i32, i32, i32) {
    const WIDTH: i32 = 380;
    const HEIGHT: i32 = 214;
    const MARGIN_X: i32 = 24;
    const MARGIN_Y: i32 = 56;
    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    (screen_w - WIDTH - MARGIN_X, screen_h - HEIGHT - MARGIN_Y, WIDTH, HEIGHT)
}

/// Detaches the mpv host window from `main` and floats it as a small,
/// always-on-top window in the corner of the screen, independent of `main`'s
/// own state -- crucially including minimized, since an *owned* popup would
/// otherwise be minimized right along with its owner, which would defeat the
/// entire point of a PiP window. Returns the rect chosen so the caller can
/// lay the HUD overlay exactly on top of it -- the HUD carries the PiP
/// controls and now covers this window completely, so it (not this window)
/// is what actually receives drag/resize input; see `sync_pip_from_hud`. The
/// caller is responsible for actually minimizing `main`.
pub fn enter_pip(hwnd_val: isize) -> (i32, i32, i32, i32) {
    let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);
    let rect @ (x, y, width, height) = pip_rect();
    unsafe {
        let _ = SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, 0);
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, width, height, SWP_SHOWWINDOW);
    }
    rect
}

/// Reverses `enter_pip`: restores `main` as the owner and drops topmost.
/// Repositioning back over `main`'s client rect is left to the caller's
/// normal `resize_host_window` call afterward, the same path any other
/// resize takes.
pub fn exit_pip(app: &AppHandle, hwnd_val: isize) -> Result<(), String> {
    let parent_val = get_parent_hwnd(app)?;
    let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);
    unsafe {
        let _ = SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, parent_val);
        let _ = SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
    Ok(())
}

/// Keeps the video glued to the HUD overlay while the user drags or resizes
/// the PiP window by its HUD chrome -- the HUD covers the video completely
/// in PiP mode and is what actually receives the mouse, so the video has to
/// be told explicitly where and how big to follow. Reads the HUD's current
/// position/size fresh rather than trusting whichever event (`Moved` or
/// `Resized`) triggered the call, since a resize from the top or left edge
/// moves the window's origin too -- both need the same full sync either way.
pub fn sync_pip_from_hud(app: &AppHandle, hwnd_val: isize) {
    let Some(hud) = app.get_webview_window("hud") else {
        return;
    };
    let (Ok(pos), Ok(size)) = (hud.outer_position(), hud.inner_size()) else {
        return;
    };
    let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            None,
            pos.x,
            pos.y,
            size.width as i32,
            size.height as i32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
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

/// Spawns the mpv sidecar attached to `hwnd_val` via `--wid`, idle (no file loaded
/// yet -- see `load_file_via_ipc`).
///
/// The HWND MUST be cast to `u32` (not a signed integer) per mpv's own documented
/// win32 embedding convention -- a signed cast can make the value negative, which
/// mpv silently rejects by detaching into its own top-level window instead of
/// embedding into ours.
///
/// The stream URL is deliberately NOT passed as a CLI argument here: doing so
/// reproducibly corrupted it specifically when spawned through this sidecar
/// path (one "/" silently dropped right after the scheme, e.g. `http://host`
/// became `http:host`) even though the exact same argument list works
/// correctly when mpv is invoked directly (outside this sidecar wrapper) --
/// so the URL is loaded after the fact via JSON IPC instead, which sidesteps
/// whatever is mangling this particular argument on the way to CreateProcess.
pub fn spawn_mpv(
    app: &AppHandle,
    hwnd_val: isize,
    pipe_name: &str,
) -> Result<tauri_plugin_shell::process::CommandChild, String> {
    let hwnd_u32 = hwnd_val as u32;

    let sidecar = app
        .shell()
        .sidecar("mpv")
        .map_err(|e| format!("Could not resolve mpv sidecar: {e}"))?;

    let args = vec![
        format!("--wid={}", hwnd_u32),
        format!("--input-ipc-server={}", pipe_name),
        "--idle=yes".to_string(),
        "--no-input-default-bindings".to_string(),
        "--no-osc".to_string(),
        "--keep-open=yes".to_string(),
        // Ignores any mpv.conf/input.conf/scripts sitting in mpv's normal
        // config locations (e.g. %APPDATA%\mpv). This sidecar's behavior
        // should be fully pinned to the flags passed here -- without this, a
        // stray system-wide mpv config (a common thing for anyone who's ever
        // installed mpv standalone, e.g. a uosc/thumbfast setup) could
        // silently override the cache/thread limits below or load scripts
        // that add their own memory overhead (thumbnail caches in
        // particular can be large), defeating the whole point of tuning
        // this budget explicitly.
        "--no-config".to_string(),
        // Query the actual display's current colorspace/HDR state (via DXGI)
        // and match the swapchain to it, instead of always presenting SDR and
        // tone-mapping HDR content down -- this is the entire reason mpv is
        // used here instead of a browser <video> tag, so it must be explicit.
        "--target-colorspace-hint=yes".to_string(),
        // Deliberately NOT --hwdec: this app is meant to run alongside a game
        // on the same GPU, and even mpv's "copy back to system RAM" hwdec
        // modes still allocate a pool of GPU decode surfaces for the
        // duration of playback, which is exactly the VRAM contention we want
        // to avoid here. Software decoding is the tradeoff that keeps this
        // entirely off the GPU.
        //
        // The two flags below trim the two parts of mpv's RAM use that are
        // *not* inherent to that tradeoff:
        //
        // - mpv defaults to a 150MiB-forward/50MiB-back network demuxer
        //   cache regardless of resolution. Jellyfin is typically LAN/local,
        //   which doesn't need that much slack to ride out network hiccups.
        //   A real test run at these limits still held ~195s of forward
        //   buffer in just 64MB (i.e. a typical stream's bitrate leaves
        //   these limits nowhere near full) -- so both are cut further here.
        "--demuxer-max-bytes=32MiB".to_string(),
        "--demuxer-max-back-bytes=8MiB".to_string(),
        // - `--vd-lavc-threads` defaults to 0 (auto = one decode thread per
        //   CPU core). FFmpeg's frame-threaded decoders need one full
        //   decoded-frame buffer per thread to pipeline across them, so on a
        //   high-core-count CPU this alone can hold a dozen-plus 4K frames
        //   in memory at once. Capping it bounds that regardless of core
        //   count -- and leaves more CPU headroom for whatever game is
        //   running, as a side benefit.
        //
        //   This does NOT touch the codec's own reference-frame buffer
        //   (DPB): HEVC/H.264 require keeping a resolution-dependent number
        //   of decoded reference frames around to decode B-frames at all, so
        //   4K will always need meaningfully more RAM than 1080p no matter
        //   what caching/threading is set -- that part isn't a tunable, it's
        //   how the codec works.
        "--vd-lavc-threads=4".to_string(),
    ];

    let (mut rx, child) = sidecar
        .args(args)
        .spawn()
        .map_err(|e| format!("Failed to spawn mpv: {e}"))?;

    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                // `-v` used to be passed above, which made mpv print a
                // statusline update (and a wall of GPU/shader/codec init
                // detail) many times a second for the entire duration of
                // playback -- not just at startup. Every one of those lines
                // was also re-emitted as a "mpv://log" Tauri event that
                // nothing in the frontend actually listens for, so it was
                // pure IPC-serialization overhead competing with real work
                // on the same async runtime, for the entire session. Kept
                // as a plain eprintln (still useful when actually
                // debugging) now that the flood is gone at the source.
                CommandEvent::Stderr(line) => {
                    eprintln!("[mpv] {}", String::from_utf8_lossy(&line));
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
    pending_start: Arc<std::sync::Mutex<Option<f64>>>,
) -> Result<IpcHandles, String> {
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

    for prop in ["time-pos", "duration", "pause", "eof-reached"] {
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

    let pending: PendingRequests = Arc::new(std::sync::Mutex::new(HashMap::new()));

    let app_handle = app.clone();
    let seek_tx = tx.clone();
    let pending_for_reader = pending.clone();
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(reader_half).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                // Responses to requests we tagged with a request_id (e.g. get_property
                // for track-list) are routed back to whoever is awaiting them, distinct
                // from the observed-property push events handled below.
                if let Some(request_id) = value.get("request_id").and_then(|v| v.as_u64()) {
                    if let Some(sender) = pending_for_reader.lock().unwrap().remove(&request_id) {
                        let data = value.get("data").cloned().unwrap_or(serde_json::Value::Null);
                        let _ = sender.send(data);
                    }
                    continue;
                }

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
                    // Once the freshly-loaded file's duration is known, mpv is
                    // ready to seek -- apply the pending resume position (if
                    // any) exactly once. `loadfile`'s own inline options can't
                    // carry it: in this mpv build, a 4th positional argument
                    // there is parsed as an integer playlist index, not a
                    // "key=value" options string.
                    if name.as_str() == Some("duration") && data.as_f64().unwrap_or(0.0) > 0.0 {
                        let start = pending_start.lock().unwrap().take();
                        if let Some(seconds) = start {
                            let _ = ipc_command(&seek_tx, serde_json::json!(["seek", seconds, "absolute"]));
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

    Ok(IpcHandles {
        sender: tx,
        pending,
        next_id: Arc::new(AtomicU64::new(1)),
    })
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

/// Loads `stream_url` into the already-running (idle) mpv instance over IPC.
/// See `spawn_mpv`'s doc comment for why this isn't just a CLI argument at
/// spawn time. The resume position (if any) is applied separately once the
/// file's duration becomes known -- see `connect_ipc`.
pub fn load_file_via_ipc(sender: &tokio::sync::mpsc::UnboundedSender<String>, stream_url: &str) -> Result<(), String> {
    ipc_command(sender, serde_json::json!(["loadfile", stream_url, "replace"]))
}

/// Asks mpv for its current track list (audio + subtitle + video tracks) via
/// a request/response round-trip over IPC, tagging the request with a fresh
/// id so the reader task in `connect_ipc` can route the specific response back
/// here instead of broadcasting it as a property-change event.
pub async fn get_track_list(ipc: &IpcHandles) -> Result<Vec<Track>, String> {
    let id = ipc.next_id.fetch_add(1, Ordering::SeqCst);
    let (tx, rx) = tokio::sync::oneshot::channel();
    ipc.pending.lock().unwrap().insert(id, tx);

    let payload = serde_json::json!({ "command": ["get_property", "track-list"], "request_id": id });
    if ipc.sender.send(payload.to_string()).is_err() {
        ipc.pending.lock().unwrap().remove(&id);
        return Err("mpv IPC channel closed".to_string());
    }

    let data = tokio::time::timeout(std::time::Duration::from_secs(5), rx)
        .await
        .map_err(|_| {
            ipc.pending.lock().unwrap().remove(&id);
            "Timed out waiting for mpv's track list".to_string()
        })?
        .map_err(|_| "mpv IPC response channel closed before replying".to_string())?;

    eprintln!("[tracks] raw track-list response: {data}");
    let parsed = serde_json::from_value(data).map_err(|e| format!("Unexpected track-list response from mpv: {e}"));
    if let Err(ref e) = parsed {
        eprintln!("[tracks] parse error: {e}");
    }
    parsed
}

pub fn set_subtitle_track(sender: &tokio::sync::mpsc::UnboundedSender<String>, track_id: Option<i64>) -> Result<(), String> {
    let value = match track_id {
        Some(id) => serde_json::json!(id),
        None => serde_json::json!("no"),
    };
    ipc_command(sender, serde_json::json!(["set_property", "sid", value]))
}

pub fn set_audio_track(sender: &tokio::sync::mpsc::UnboundedSender<String>, track_id: i64) -> Result<(), String> {
    ipc_command(sender, serde_json::json!(["set_property", "aid", track_id]))
}

pub fn ipc_command(sender: &tokio::sync::mpsc::UnboundedSender<String>, command: serde_json::Value) -> Result<(), String> {
    let payload = serde_json::json!({ "command": command });
    sender
        .send(payload.to_string())
        .map_err(|e| format!("mpv IPC channel closed: {e}"))
}
