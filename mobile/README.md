# FastFin (Expo)

React Native / Expo port of the FastFin Jellyfin client, replacing the
native Swift attempt in [`../ios`](../ios) -- that approach needed a Mac
for every build and a manual AltStore sideload for every test, which made
iterating on real bugs painfully slow. This runs live on your phone via
Expo Go with no build step at all.

## Run it

```bash
cd mobile
npx expo start
```

Install **Expo Go** from the App Store, scan the QR code the command
prints, and the app loads and live-reloads over your WiFi. No Xcode, no
CI, no sideloading.

## What's here

- **`lib/session.tsx`** -- auth via `expo-secure-store`, and the
  `MediaBrowser DeviceId=..., Client=..., Version=..., Token=...`
  Authorization header format Jellyfin expects (verified against the
  official `jellyfin-sdk-swift` source, not guessed).
- **`lib/jellyfin.ts`** -- the REST client: libraries, items, search,
  resume, seasons/episodes, similar items, image URLs, and
  `getPlaybackSource` (same approach that ended up working in the Swift
  build: call `PlaybackInfo` for the real `MediaSourceId`/`PlaySessionId`,
  then hand-build the `/master.m3u8` transcode URL with an explicit
  H.264/AAC target rather than trusting the negotiated response).
- **`components/`** -- `Hero`, `CarouselRow`, `PosterCard`,
  `ContinueWatchingCard`, matching the approved mockup (image-anchored,
  glass/blur chrome, dark-first).
- **`app/`** -- file-based routes via `expo-router`: `login`, the
  `(tabs)` group (Home/Library/Search/Settings), `item/[id]`,
  `library/[id]`, `season/[id]`, and `player/[id]` (uses `expo-video`,
  which is AVPlayer-backed on iOS -- forced landscape via
  `expo-screen-orientation`, exempted while Picture-in-Picture is active,
  and a custom HUD with a real audio/subtitle track menu).

## Known gaps

- Track switching remounts the player (`key` prop) rather than swapping
  the source in place -- simplest reliable approach given `expo-video`'s
  source-change behavior wasn't confirmed against this exact SDK version.
- No app icon/splash assets customized yet -- using Expo's defaults.
- Verified via `tsc --noEmit` (clean) and a real Metro bundle compile
  (succeeded), but never actually run on a device from this session --
  Windows machine, no simulator. Treat the first real launch as the true
  smoke test.
