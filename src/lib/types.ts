export interface Library {
  Id: string;
  Name: string;
  CollectionType: string | null;
}

export interface UserData {
  PlaybackPositionTicks: number | null;
  PlayedPercentage: number | null;
}

export interface Item {
  Id: string;
  Name: string;
  Type: string;
  Overview: string | null;
  ProductionYear: number | null;
  RunTimeTicks: number | null;
  IndexNumber: number | null;
  SeriesName: string | null;
  UserData: UserData | null;
  BackdropImageTags: string[] | null;
  ParentBackdropItemId: string | null;
}

/** Picks the item to actually source a backdrop image from: an item's own
 * backdrop if it has one, else falls back to its parent (e.g. an episode
 * falls back to its series' backdrop, since episodes rarely have their own).
 * Returns null if neither is available. */
export function backdropSourceId(item: Item): string | null {
  if (item.BackdropImageTags && item.BackdropImageTags.length > 0) return item.Id;
  return item.ParentBackdropItemId;
}
