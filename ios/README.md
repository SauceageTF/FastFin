# FastFin iOS

Native SwiftUI + AVKit port of the FastFin Jellyfin client. Written on Windows
(no Xcode here), so it has never been built or run yet -- treat the first
build on a Mac as the real smoke test, not this scaffold.

## Why XcodeGen instead of a checked-in `.xcodeproj`

`project.pbxproj` is a generated, merge-hostile file that's easy to corrupt
by hand and impossible to verify without Xcode. [`project.yml`](project.yml)
is a plain-text spec that [XcodeGen](https://github.com/yonaskolb/XcodeGen)
turns into a real `.xcodeproj` on demand -- regenerate it any time the
project file drifts instead of hand-editing it.

## Build (on a Mac)

```bash
brew install xcodegen
cd ios
xcodegen generate
open FastFin.xcodeproj
```

Then build/run the `FastFin` scheme on a simulator or device from Xcode.

## What's here

- **Session** -- `JellyfinSession` wraps the official
  [`JellyfinAPI`](https://github.com/jellyfin/jellyfin-sdk-swift) Swift SDK
  (added as a Swift Package dependency in `project.yml`), persists the
  server URL/access token/user ID in the Keychain, and mirrors
  `src/lib/jellyfinClient.ts`'s `login` / `tryRestoreSession` / `logout`.
- **Networking** -- `MediaService` mirrors the rest of `jellyfinClient.ts`
  (libraries, latest items, resume, item detail, seasons, episodes, similar
  items, image URLs) but calls the Jellyfin REST API directly instead of
  going through a Rust IPC layer, since there's no Tauri backend on iOS.
- **Views** -- `HomeView`/`ItemDetailView`/`LoginView` follow the approved
  mockup: image-anchored screens, glass (`.ultraThinMaterial`) chrome, and a
  title-art logo instead of plain text where the server provides one.
- **Player** -- `PlayerViewController` is UIKit (`AVPlayerLayer` +
  `AVPictureInPictureController`) with a SwiftUI custom HUD drawn on top,
  matching the desktop app's own custom-chrome player rather than using the
  stock `AVPlayerViewController` controls. Currently direct-play only (see
  the TODO in `MediaService` about `PlaybackInfo` negotiation).

## Known gaps to close on first build

- **Sora font isn't bundled.** `Theme.displayFont` references `"Sora"` by
  name; until the actual `.ttf` files are added under a `Resources/Fonts`
  group and registered as `UIAppFonts` in `project.yml`, headings silently
  render in the system font instead.
- **Playback is direct-play only.** `MediaService` builds a plain
  `/Videos/{id}/stream` URL. Real transcoding fallback needs the
  `PlaybackInfo` negotiation endpoint, which isn't wired up yet.
- A few `JellyfinAPI` parameter struct field names (mostly on the
  `Get*ByIndex` image endpoints) were confirmed by reading the SDK's
  generated source rather than by compiling against it -- if Xcode flags a
  mismatch, it'll be a one-line fix in `MediaService.swift`.
