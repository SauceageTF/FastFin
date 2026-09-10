# Changelog

All notable changes to this project are documented in this file.

## [0.2.1] - 2026-09-10

### Fixed

- Closing the app while a video was playing left `mpv.exe` running and the HUD overlay stuck on screen — nothing tore playback down when the main window closed. Closing now runs the same cleanup as the in-app "back"/"stop" actions: mpv is sent a `quit` command and killed, and both the video host window and HUD window are destroyed.

### Changed

- Reduced mpv's memory footprint: the demuxer's network cache is capped much lower (from 150MB/50MB down to 32MB/8MB — Jellyfin is typically LAN/local and doesn't need that much slack), software decode thread count is capped instead of scaling with CPU core count, and mpv no longer reads any system-wide config/scripts, so behavior stays fully pinned to what this app passes it. Hardware decoding was deliberately left off so mpv doesn't compete for GPU/VRAM with other GPU-heavy use (e.g. gaming) on the same machine.

## [0.2.0] - 2026-09-10

### Renamed

- Project renamed from **Saucefin** to **FastFin**, reflecting a focus on staying lightweight: no bundled browser engine for the UI, and video decode/render handled entirely by a native `mpv` process instead of a browser media pipeline. See the README's "Why it's light" section for details.

### Added

- **Picture-in-Picture mode.** Shrinks the video into a small, always-on-top floating window and gets the main window out of the way (minimized), so playback keeps going while you do something else.
  - Hover the floating window to reveal transport controls (play/pause, skip back 7s, skip forward 5s) and a button to snap back to the main window; move the mouse away and it fades back to just the video.
  - Movable by dragging anywhere on the window's empty background.
  - Resizable from any corner, locked to a fixed 16:9 aspect ratio so the video is never stretched or squished.
  - Restoring the main window (clicking it, Alt+Tab, or the floating window's own restore button) automatically exits PiP and snaps the video back into place.

### Fixed

- Window drag/resize on the PiP overlay now actually works — it depends on two Tauri window permissions (`allow-start-dragging`, `allow-start-resize-dragging`) that weren't granted, so both calls were previously being silently rejected.

## [0.1.0] - 2026-09-07

### Added

- Initial release: a Windows desktop Jellyfin client built with Tauri (Rust) and SvelteKit.
- Library browsing, item/season/episode navigation, and resume/continue-watching support against a Jellyfin server.
- Video playback via an embedded native `mpv` process (not a browser `<video>` tag), for proper HDR passthrough and broad codec support — the same approach Jellyfin's own official desktop client takes.
- Custom always-on-top HUD overlay for playback controls, audio/subtitle track selection, seeking, volume, and fullscreen.
