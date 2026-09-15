# Changelog

All notable changes to this project are documented in this file.

## [0.3.0] - 2026-09-15

### Added

- **Search.** A search page (magnifier in the header, or press `/` / `Ctrl+K` anywhere) that searches every library as you type and lists movies and series — never individual episodes, so a show isn't buried under its own episode titles.
- **Skip Intro.** Integrates with the Intro Skipper plugin: a "Skip Intro" / "Skip Recap" button appears while you're inside a detected segment (or press `S`). Falls back to the plugin's older single-intro endpoint and then to Jellyfin 10.10's native media segments; without any of them the button simply never shows.
- **Autoplay next episode.** When the credits start (or the file ends, if there's no credits data) an "Up next" card counts down from 10 seconds and plays the next episode — across season boundaries — with Play now / Cancel. Seeking back out of the credits withdraws it.
- **Keyboard shortcuts in the player.** Space/`K` play-pause, `←`/`→` (or `J`/`L`) seek 10s — hold Shift for 60s — `↑`/`↓` volume, `M` mute, `F` fullscreen, `N` next episode, `E` episode picker, `Esc` closes panels / leaves fullscreen. Listed in the settings popover.
- **Episode picker in the player.** A list button in the control bar opens the current season's episodes (with watched marks) so you can jump around without leaving playback. The top bar now also shows what's playing.
- **Next Up row** on the home page — the next unwatched episode of every show you're partway through, deduplicated against Continue Watching.
- **Play / Continue button on series pages** that resumes exactly where you left off in the show (or starts S1E1), plus a **Random Episode** button that picks any episode across all seasons (specials excluded). Movie libraries get a matching **Surprise me** button.
- **Watched indicators** everywhere: a checkmark badge on watched items and a progress bar on partially watched ones, on the home rows, library grid, season episode list (watched episodes are dimmed) and More Like This. **Right-click any card** to mark it watched or unwatched.
- **Library sorting and filtering.** Sort any library by name, date added, release year, rating or runtime with an ascending/descending toggle (remembered per library), and filter by genre with a chip row.
- **Playback preferences.** Volume is remembered between sessions, and a preferred audio and subtitle language can be set in the settings popover — matching tracks are selected automatically on every new file.
- Season posters on the series page (falling back to the series poster when a season has no artwork of its own), instead of plain name buttons.

### Changed

- New typography: Inter for UI text at a slightly heavier weight, Plus Jakarta Sans for headings, and everything a touch larger.
- The home page hero no longer crops the backdrop. The 16:9 image keeps its full height and sits flush right with its left edge fading into the page, so nothing is cut off on any window at least as wide as 16:9.
- Movies in Continue Watching use their landscape thumb/backdrop art instead of the portrait poster squeezed into a wide card.
- Carousel arrows only appear when there is somewhere to scroll in that direction.
- `Esc` goes back a page while browsing (on the search page it clears the box first).

### Fixed

- **The player HUD stopped responding to the mouse after a while** — typically after clicking something on it (pause, seeking near the end of an episode). The HUD overlay window was activatable, so a click on it took focus away from the main window, whose lost-focus handler then hid the HUD; and since mpv's video surface never takes focus, nothing brought it back short of Alt+Tab. The HUD is now a non-activatable window: it still takes mouse input but never steals focus.
- "Recently Added" rows in TV libraries showed a bare episode whenever a show had exactly one new episode (Jellyfin only groups episodes into their series once there are two or more). Every such episode is now folded back into its series, so the row only ever shows shows and movies.

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
