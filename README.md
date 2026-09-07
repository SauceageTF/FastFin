# Saucefin

A Windows desktop Jellyfin client. Tauri (Rust) + SvelteKit frontend, video playback rendered through an embedded real `mpv` process (not a browser `<video>` tag) for proper HDR/codec support — the same reason Jellyfin's own official desktop client uses mpv/libmpv.

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
