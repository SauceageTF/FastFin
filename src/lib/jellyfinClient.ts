import { invoke } from "@tauri-apps/api/core";
import type { Library, Item } from "./types";

export async function login(serverUrl: string, username: string, password: string): Promise<void> {
  await invoke("login", { serverUrl, username, password });
}

export async function tryRestoreSession(): Promise<boolean> {
  return invoke("try_restore_session");
}

export async function logout(): Promise<void> {
  await invoke("logout");
}

export async function getLibraries(): Promise<Library[]> {
  return invoke("get_libraries");
}

export async function getItems(libraryId: string): Promise<Item[]> {
  return invoke("get_items", { libraryId });
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

export async function getBackdropUrl(itemId: string): Promise<string> {
  return invoke("get_backdrop_url", { itemId });
}

export async function getSeasons(seriesId: string): Promise<Item[]> {
  return invoke("get_seasons", { seriesId });
}

export async function getEpisodes(seriesId: string, seasonId: string): Promise<Item[]> {
  return invoke("get_episodes", { seriesId, seasonId });
}

export async function getImageUrl(itemId: string): Promise<string> {
  return invoke("get_image_url", { itemId });
}

export async function startPlayback(
  itemId: string,
  x: number,
  y: number,
  width: number,
  height: number,
  startSeconds: number,
): Promise<void> {
  await invoke("start_playback", { itemId, x, y, width, height, startSeconds });
}

export async function resizeVideoSurface(
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<void> {
  await invoke("resize_video_surface", { x, y, width, height });
}

export async function stopPlayback(): Promise<void> {
  await invoke("stop_playback");
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
