export interface Library {
  Id: string;
  Name: string;
  CollectionType: string | null;
}

export interface UserData {
  PlaybackPositionTicks: number | null;
  PlayedPercentage: number | null;
  Played: boolean | null;
}

export interface Item {
  Id: string;
  Name: string;
  Type: string;
  Overview: string | null;
  ProductionYear: number | null;
  RunTimeTicks: number | null;
  /** ISO-8601 "added to library" timestamp; only present when requested. */
  DateCreated: string | null;
  IndexNumber: number | null;
  /** Season number for episodes (0 = specials). */
  ParentIndexNumber: number | null;
  SeriesName: string | null;
  SeriesId: string | null;
  SeasonId: string | null;
  UserData: UserData | null;
  BackdropImageTags: string[] | null;
  ParentBackdropItemId: string | null;
  Genres: string[] | null;
  CommunityRating: number | null;
  OfficialRating: string | null;
  Taglines: string[] | null;
  ImageTags: Record<string, string> | null;
}

/** Whether Jellyfin has a Logo (wordmark) image for this item. */
export function hasLogo(item: Item): boolean {
  return !!item.ImageTags?.Logo;
}

/** Picks the item to actually source a backdrop image from: an item's own
 * backdrop if it has one, else falls back to its parent (e.g. an episode
 * falls back to its series' backdrop, since episodes rarely have their own).
 * Returns null if neither is available. */
export function backdropSourceId(item: Item): string | null {
  if (item.BackdropImageTags && item.BackdropImageTags.length > 0) return item.Id;
  return item.ParentBackdropItemId;
}

export interface Track {
  id: number;
  type: string;
  lang: string | null;
  title: string | null;
  selected: boolean;
}

/** A skippable stretch of an item (intro, credits, recap, ...), in seconds. */
export interface SkipSegment {
  kind: "Intro" | "Credits" | "Recap" | "Preview" | "Commercial" | string;
  start: number;
  end: number;
}

/** "S2E5"-style label for an episode, or "" if it has no numbering. */
export function episodeCode(item: Item): string {
  if (item.IndexNumber == null) return "";
  const season = item.ParentIndexNumber != null ? `S${item.ParentIndexNumber}` : "";
  return `${season}E${item.IndexNumber}`;
}
