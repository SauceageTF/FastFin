import { authHeaderFor, type SessionInfo } from "./session";
import type { Item, Library } from "./types";

async function get<T>(session: SessionInfo, path: string, query: Record<string, string | number | boolean | undefined> = {}): Promise<T> {
  const url = new URL(session.serverUrl + path);
  for (const [key, value] of Object.entries(query)) {
    if (value !== undefined) url.searchParams.set(key, String(value));
  }
  const response = await fetch(url.toString(), { headers: { Authorization: authHeaderFor(session) } });
  if (!response.ok) throw new Error(`${path} failed: ${response.status}`);
  return response.json();
}

async function post<T>(session: SessionInfo, path: string, body: unknown, query: Record<string, string | number | boolean | undefined> = {}): Promise<T> {
  const url = new URL(session.serverUrl + path);
  for (const [key, value] of Object.entries(query)) {
    if (value !== undefined) url.searchParams.set(key, String(value));
  }
  const response = await fetch(url.toString(), {
    method: "POST",
    headers: { Authorization: authHeaderFor(session), "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) throw new Error(`${path} failed: ${response.status}`);
  return response.json();
}

export async function getLibraries(session: SessionInfo): Promise<Library[]> {
  const result = await get<{ Items: Library[] }>(session, "/UserViews", { userId: session.userId });
  return result.Items ?? [];
}

export async function getLatestItems(session: SessionInfo, libraryId: string, limit = 16): Promise<Item[]> {
  return get<Item[]>(session, "/Items/Latest", { userId: session.userId, parentId: libraryId, limit });
}

export async function getItems(session: SessionInfo, libraryId: string, limit = 500): Promise<Item[]> {
  const result = await get<{ Items: Item[] }>(session, "/Items", {
    userId: session.userId,
    parentId: libraryId,
    recursive: true,
    includeItemTypes: "Movie,Series",
    sortBy: "SortName",
    limit,
  });
  return result.Items ?? [];
}

export async function search(session: SessionInfo, query: string, limit = 50): Promise<Item[]> {
  if (!query) return [];
  const result = await get<{ Items: Item[] }>(session, "/Items", {
    userId: session.userId,
    searchTerm: query,
    recursive: true,
    includeItemTypes: "Movie,Series",
    sortBy: "SortName",
    limit,
  });
  return result.Items ?? [];
}

export async function getResumeItems(session: SessionInfo, limit = 16): Promise<Item[]> {
  const result = await get<{ Items: Item[] }>(session, "/UserItems/Resume", { userId: session.userId, limit });
  return result.Items ?? [];
}

export async function getItem(session: SessionInfo, id: string): Promise<Item> {
  return get<Item>(session, `/Items/${id}`, { userId: session.userId });
}

export async function getSeasons(session: SessionInfo, seriesId: string): Promise<Item[]> {
  const result = await get<{ Items: Item[] }>(session, `/Shows/${seriesId}/Seasons`, { userId: session.userId });
  return result.Items ?? [];
}

export async function getEpisodes(session: SessionInfo, seriesId: string, seasonId: string): Promise<Item[]> {
  const result = await get<{ Items: Item[] }>(session, `/Shows/${seriesId}/Episodes`, { userId: session.userId, seasonId });
  return result.Items ?? [];
}

export async function getSimilarItems(session: SessionInfo, itemId: string, limit = 16): Promise<Item[]> {
  const result = await get<{ Items: Item[] }>(session, `/Items/${itemId}/Similar`, { userId: session.userId, limit });
  return result.Items ?? [];
}

// MARK: - Images (api_key query param, same as the Swift build -- these
// URLs get handed directly to <Image>/expo-video, which can't attach a
// custom Authorization header).

export function getImageUrl(session: SessionInfo, itemId: string, maxWidth = 480): string {
  return `${session.serverUrl}/Items/${itemId}/Images/Primary?api_key=${session.accessToken}&maxWidth=${maxWidth}`;
}

export function getBackdropUrl(session: SessionInfo, itemId: string, maxWidth = 1920): string {
  return `${session.serverUrl}/Items/${itemId}/Images/Backdrop/0?api_key=${session.accessToken}&maxWidth=${maxWidth}`;
}

export function getLogoUrl(session: SessionInfo, itemId: string, maxWidth = 800): string {
  return `${session.serverUrl}/Items/${itemId}/Images/Logo?api_key=${session.accessToken}&maxWidth=${maxWidth}`;
}

// MARK: - Playback
//
// Same approach that ended up working in the Swift build: call PlaybackInfo
// to get the *real* MediaSourceId and PlaySessionId (guessing MediaSourceId
// == itemId produced NSURLErrorResourceUnavailable-equivalent failures),
// then hand-build the /master.m3u8 transcode URL with an explicit H.264/AAC
// target rather than trusting the negotiated (and often absent without a
// full DeviceProfile) transcodingUrl.

export interface TrackOption {
  index: number;
  title: string;
}

export interface PlaybackSource {
  url: string;
  mediaSourceId: string;
  playSessionId: string;
  audioTracks: TrackOption[];
  subtitleTracks: TrackOption[];
  selectedAudioIndex?: number;
  selectedSubtitleIndex?: number;
}

interface PlaybackInfoResponse {
  PlaySessionId?: string;
  MediaSources?: {
    Id: string;
    MediaStreams?: { Index: number; Type: string; DisplayTitle?: string; Language?: string }[];
  }[];
}

export async function getPlaybackSource(
  session: SessionInfo,
  itemId: string,
  startTicks: number,
  audioStreamIndex?: number,
  subtitleStreamIndex?: number
): Promise<PlaybackSource | null> {
  const response = await post<PlaybackInfoResponse>(
    session,
    `/Items/${itemId}/PlaybackInfo`,
    {},
    { userId: session.userId, startTimeTicks: startTicks, audioStreamIndex, subtitleStreamIndex }
  );

  const mediaSource = response.MediaSources?.[0];
  if (!mediaSource) return null;
  const playSessionId = response.PlaySessionId ?? Math.random().toString(36).slice(2);

  const audioTracks: TrackOption[] = [];
  const subtitleTracks: TrackOption[] = [];
  for (const stream of mediaSource.MediaStreams ?? []) {
    const option = { index: stream.Index, title: stream.DisplayTitle ?? stream.Language ?? `${stream.Type} ${stream.Index}` };
    if (stream.Type === "Audio") audioTracks.push(option);
    if (stream.Type === "Subtitle") subtitleTracks.push(option);
  }

  const url = new URL(`${session.serverUrl}/Videos/${itemId}/master.m3u8`);
  url.searchParams.set("api_key", session.accessToken);
  url.searchParams.set("DeviceId", session.deviceId);
  url.searchParams.set("MediaSourceId", mediaSource.Id);
  url.searchParams.set("PlaySessionId", playSessionId);
  url.searchParams.set("VideoCodec", "h264");
  url.searchParams.set("AudioCodec", "aac");
  url.searchParams.set("TranscodingMaxAudioChannels", "2");
  url.searchParams.set("SegmentContainer", "ts");
  url.searchParams.set("StartTimeTicks", String(startTicks));
  if (audioStreamIndex !== undefined) url.searchParams.set("AudioStreamIndex", String(audioStreamIndex));
  if (subtitleStreamIndex !== undefined) {
    url.searchParams.set("SubtitleStreamIndex", String(subtitleStreamIndex));
    url.searchParams.set("SubtitleMethod", "Encode");
  }

  return {
    url: url.toString(),
    mediaSourceId: mediaSource.Id,
    playSessionId,
    audioTracks,
    subtitleTracks,
    selectedAudioIndex: audioStreamIndex,
    selectedSubtitleIndex: subtitleStreamIndex,
  };
}
