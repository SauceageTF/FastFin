import { invoke } from "@tauri-apps/api/core";
import type { Library, Item, Track, SkipSegment } from "./types";

interface SessionInfo {
  serverUrl: string;
  accessToken: string;
}

// getImageUrl/getBackdropUrl are called for every poster on screen -- a page
// like the home page easily has 80+ of them. Each used to be its own IPC
// round-trip to Rust just to format a string with zero actual async work
// behind it. Fetching the session info once (memoized -- concurrent callers
// share the same in-flight request) and building URLs locally afterward
// removes that per-image overhead entirely. Cleared on login/logout so a
// session change can't leave a stale server/token cached.
let sessionInfoPromise: Promise<SessionInfo> | null = null;

function sessionInfo(): Promise<SessionInfo> {
  if (!sessionInfoPromise) sessionInfoPromise = invoke("get_session_info");
  return sessionInfoPromise;
}

export async function login(serverUrl: string, username: string, password: string): Promise<void> {
  await invoke("login", { serverUrl, username, password });
  sessionInfoPromise = null;
}

export async function tryRestoreSession(): Promise<boolean> {
  sessionInfoPromise = null;
  return invoke("try_restore_session");
}

export async function logout(): Promise<void> {
  await invoke("logout");
  sessionInfoPromise = null;
}

export async function getLibraries(): Promise<Library[]> {
  return invoke("get_libraries");
}

export async function getItems(libraryId: string): Promise<Item[]> {
  return invoke("get_items", { libraryId });
}

export async function getLatestItems(libraryId: string): Promise<Item[]> {
  return invoke("get_latest_items", { libraryId });
}

export async function getItem(itemId: string): Promise<Item> {
  return invoke("get_item", { itemId });
}

export async function getPlaylists(): Promise<Item[]> {
  return invoke("get_playlists");
}

export async function getResume(): Promise<Item[]> {
  return invoke("get_resume");
}

/** Title search for movies and series only -- never individual episodes. */
export async function searchItems(term: string): Promise<Item[]> {
  return invoke("search_items", { term });
}

export async function getBackdropUrl(itemId: string, maxWidth = 1920): Promise<string> {
  const { serverUrl, accessToken } = await sessionInfo();
  // Backdrop images are a per-item array in Jellyfin (unlike Primary), so
  // the image index must be explicit -- omitting it 404s rather than
  // defaulting. maxWidth asks Jellyfin to serve (and cache) a resized copy
  // rather than the original -- these are frequently several MB at full
  // resolution for no visible benefit at the size this actually renders at,
  // so this cuts both transfer time and the decoded bitmap's footprint in
  // the webview's memory.
  return `${serverUrl}/Items/${itemId}/Images/Backdrop/0?api_key=${accessToken}&maxWidth=${maxWidth}`;
}

// Jellyfin's "Thumb" image is a dedicated landscape (16:9) still, distinct
// from both the portrait Primary poster and the full-width Backdrop. Only
// call this once you know the item has one (`item.ImageTags?.Thumb`) --
// like Logo, it 404s rather than falling back when missing.
export async function getThumbUrl(itemId: string, maxWidth = 800): Promise<string> {
  const { serverUrl, accessToken } = await sessionInfo();
  return `${serverUrl}/Items/${itemId}/Images/Thumb?api_key=${accessToken}&maxWidth=${maxWidth}`;
}

// Only call this once you know the item actually has a Logo image (check
// `hasLogo()` from `./types` first) -- most items don't have one, and
// requesting it anyway would just 404.
export async function getLogoUrl(itemId: string, maxWidth = 800): Promise<string> {
  const { serverUrl, accessToken } = await sessionInfo();
  return `${serverUrl}/Items/${itemId}/Images/Logo?api_key=${accessToken}&maxWidth=${maxWidth}`;
}

export async function getSeasons(seriesId: string): Promise<Item[]> {
  return invoke("get_seasons", { seriesId });
}

export async function getEpisodes(seriesId: string, seasonId: string): Promise<Item[]> {
  return invoke("get_episodes", { seriesId, seasonId });
}

/** Every episode of a series across all seasons. */
export async function getSeriesEpisodes(seriesId: string): Promise<Item[]> {
  return invoke("get_series_episodes", { seriesId });
}

export async function getImageUrl(itemId: string, maxWidth = 480): Promise<string> {
  const { serverUrl, accessToken } = await sessionInfo();
  // Poster cards render at 160-340px in CSS; 480 covers that comfortably
  // even at 2x DPR without asking the server for a multi-MB original. See
  // getBackdropUrl's comment -- same reasoning, smaller default because
  // posters render much smaller than the hero backdrop does.
  return `${serverUrl}/Items/${itemId}/Images/Primary?api_key=${accessToken}&maxWidth=${maxWidth}`;
}

export async function getSimilarItems(itemId: string): Promise<Item[]> {
  return invoke("get_similar_items", { itemId });
}

export async function startPlayback(itemId: string, startSeconds: number): Promise<void> {
  await invoke("start_playback", { itemId, startSeconds });
}

export async function stopPlayback(): Promise<void> {
  await invoke("stop_playback");
}

export async function goBackToItem(itemId: string): Promise<void> {
  await invoke("go_back_to_item", { itemId });
}

export async function toggleMainFullscreen(): Promise<boolean> {
  return invoke("toggle_main_fullscreen");
}

// Floats the video as a small always-on-top corner window and minimizes the
// main window to get it out of the way. There's no matching `exitPip` --
// restoring the main window (taskbar, Alt+Tab, ...) is the only way out, and
// the Rust side handles that itself from the window's focus event.
export async function enterPip(): Promise<void> {
  await invoke("enter_pip");
}

// The PiP overlay's own "return to FastFin" control. Just restores focus to
// the main window -- the Rust side tears PiP down itself in reaction to that
// focus change, the same as if the user had clicked the taskbar icon.
export async function restoreFromPip(): Promise<void> {
  await invoke("restore_from_pip");
}

export async function mpvSetPause(paused: boolean): Promise<void> {
  await invoke("mpv_set_pause", { paused });
}

export async function mpvSeek(seconds: number): Promise<void> {
  await invoke("mpv_seek", { seconds });
}

export async function mpvSetVolume(volume: number): Promise<void> {
  await invoke("mpv_set_volume", { volume });
}

export async function getTracks(): Promise<Track[]> {
  return invoke("get_tracks");
}

export async function mpvSetSubtitleTrack(trackId: number | null): Promise<void> {
  await invoke("mpv_set_subtitle_track", { trackId });
}

export async function mpvSetAudioTrack(trackId: number): Promise<void> {
  await invoke("mpv_set_audio_track", { trackId });
}

/** Jellyfin's "Next Up" -- the next unwatched episode per started series,
 * or, with `seriesId`, just that show's next episode (S1E1 if unstarted). */
export async function getNextUp(seriesId?: string): Promise<Item[]> {
  return invoke("get_next_up", { seriesId: seriesId ?? null });
}

/** The episode after `episodeId` in running order, or null at the end of the show. */
export async function getNextEpisode(episodeId: string): Promise<Item | null> {
  return invoke("get_next_episode", { episodeId });
}

export async function setPlayed(itemId: string, played: boolean): Promise<void> {
  return invoke("set_played", { itemId, played });
}

/** Intro/credits segments from Intro Skipper (or Jellyfin's native media
 * segments). Empty when neither is available -- never throws. */
export async function getSkipSegments(itemId: string): Promise<SkipSegment[]> {
  return invoke("get_skip_segments", { itemId }).catch(() => []) as Promise<SkipSegment[]>;
}

/** From the HUD window: asks `main` to switch playback to another item. */
export async function playItem(itemId: string): Promise<void> {
  return invoke("play_item", { itemId });
}
