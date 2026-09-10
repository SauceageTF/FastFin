import { writable } from "svelte/store";

export type ThemeColor = "white" | "teal" | "ember";

const THEMES: Record<ThemeColor, { accent: string; accentHover: string }> = {
  white: { accent: "#f2f1f6", accentHover: "#d8d7db" },
  teal: { accent: "#2dd4c8", accentHover: "#55e0d6" },
  ember: { accent: "#ff5a1f", accentHover: "#ff7a3d" },
};

const STORAGE_KEY = "fastfin-theme-color";

function loadInitial(): ThemeColor {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "white" || stored === "teal" || stored === "ember") return stored;
  } catch {
    // localStorage unavailable; fall through to default
  }
  return "ember";
}

export const themeColor = writable<ThemeColor>(loadInitial());

themeColor.subscribe((value) => {
  const { accent, accentHover } = THEMES[value];
  try {
    document.documentElement.style.setProperty("--accent", accent);
    document.documentElement.style.setProperty("--accent-hover", accentHover);
  } catch {
    // not running in a browser context yet
  }
  try {
    localStorage.setItem(STORAGE_KEY, value);
  } catch {
    // ignore persistence failures
  }
});
