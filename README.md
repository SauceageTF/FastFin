# FastFin

A fast, lightweight Windows desktop Jellyfin client. Tauri (Rust) + SvelteKit frontend, video playback rendered through an embedded real `mpv` process (not a browser `<video>` tag) for proper HDR/codec support — the same reason Jellyfin's own official desktop client uses mpv/libmpv.

## Why it's light

- **No bundled browser engine.** Tauri renders the UI through the OS's own WebView2 runtime instead of shipping a full Chromium copy with the app (the way Electron-based clients do) — a much smaller install and lower idle memory footprint for the UI layer.
- **Video never touches a browser media pipeline.** Playback is a real, separate `mpv` process with hardware-accelerated decode, driven over a native window handle and a JSON IPC pipe — not a browser `<video>` tag. That sidesteps the decode/tone-mapping overhead and HDR quality loss common in web-based Jellyfin clients, and gets mpv's own broad, efficient codec support for free.
- **The window chrome is native, not re-rendered video.** The always-on-top HUD (playback controls, track pickers) is a second, tiny transparent overlay window layered above the video surface at the OS compositor level — the video frame itself is never redrawn or scaled to make room for controls.
- **Picture-in-Picture is a real floating OS window, not a second player.** Entering PiP just detaches and shrinks the existing native video window and lays a small overlay on top of it; there's no second decode pipeline or duplicated webview involved.

## Setup

1. Install [Rust](https://www.rust-lang.org/tools/install) and the MSVC C++ Build Tools (Visual Studio Build Tools, "Desktop development with C++" workload).
2. `npm install`
3. Download the mpv sidecar binary (not committed to source control — ~120MB):
   - Get a 64-bit build from [sourceforge.net/projects/mpv-player-windows](https://sourceforge.net/projects/mpv-player-windows/files/64bit/) (the same source mpv.io links to for Windows).
   - Extract it and copy `mpv.exe` to `src-tauri/binaries/mpv-x86_64-pc-windows-msvc.exe`.
   - mpv is GPLv2+; it's bundled here as a spawned subprocess (not linked), which doesn't impose GPL obligations on this repo's own code, but ship its license/attribution alongside any public distribution of the built app.
4. `npm run tauri dev`

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
