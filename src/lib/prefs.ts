// Small persisted user preferences, kept in localStorage. Both windows
// (main and the player HUD overlay) share the same origin, so a preference
// set in main's settings popover is visible to the HUD on its next playback.

function read<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw === null ? fallback : (JSON.parse(raw) as T);
  } catch {
    return fallback;
  }
}

function write(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Storage unavailable -- preferences just don't persist this session.
  }
}

export function getVolume(): number {
  const v = read<number>("fastfin.volume", 100);
  return Number.isFinite(v) ? Math.min(100, Math.max(0, v)) : 100;
}

export function setVolume(volume: number) {
  write("fastfin.volume", volume);
}

/** A language option for the audio/subtitle preference pickers. `codes` are
 * every ISO 639 spelling a track might be tagged with (639-1, 639-2/B,
 * 639-2/T) so a preference matches regardless of how the file was muxed. */
export interface LanguageOption {
  code: string;
  label: string;
  codes: string[];
}

export const LANGUAGE_OPTIONS: LanguageOption[] = [
  { code: "eng", label: "English", codes: ["en", "eng"] },
  { code: "jpn", label: "Japanese", codes: ["ja", "jpn"] },
  { code: "spa", label: "Spanish", codes: ["es", "spa"] },
  { code: "fre", label: "French", codes: ["fr", "fre", "fra"] },
  { code: "ger", label: "German", codes: ["de", "ger", "deu"] },
  { code: "ita", label: "Italian", codes: ["it", "ita"] },
  { code: "por", label: "Portuguese", codes: ["pt", "por"] },
  { code: "kor", label: "Korean", codes: ["ko", "kor"] },
  { code: "chi", label: "Chinese", codes: ["zh", "chi", "zho"] },
  { code: "hin", label: "Hindi", codes: ["hi", "hin"] },
  { code: "ara", label: "Arabic", codes: ["ar", "ara"] },
  { code: "rus", label: "Russian", codes: ["ru", "rus"] },
];

/** "" = leave mpv's default. */
export function getPreferredAudio(): string {
  return read<string>("fastfin.prefAudio", "");
}

export function setPreferredAudio(code: string) {
  write("fastfin.prefAudio", code);
}

/** "" = leave mpv's default, "off" = no subtitles, else a language code. */
export function getPreferredSubtitle(): string {
  return read<string>("fastfin.prefSubtitle", "");
}

export function setPreferredSubtitle(code: string) {
  write("fastfin.prefSubtitle", code);
}

/** Whether a track's language tag matches a preference code. */
export function languageMatches(trackLang: string | null | undefined, prefCode: string): boolean {
  if (!trackLang) return false;
  const option = LANGUAGE_OPTIONS.find((o) => o.code === prefCode);
  const lang = trackLang.toLowerCase();
  return option ? option.codes.includes(lang) : lang === prefCode.toLowerCase();
}

export type SortKey = "name" | "added" | "year" | "rating" | "runtime";

export interface LibrarySort {
  key: SortKey;
  descending: boolean;
}

export function getLibrarySort(libraryId: string): LibrarySort {
  return read<LibrarySort>(`fastfin.sort.${libraryId}`, { key: "name", descending: false });
}

export function setLibrarySort(libraryId: string, sort: LibrarySort) {
  write(`fastfin.sort.${libraryId}`, sort);
}
