/** Ported straight from the desktop app's app.css tokens and theme.ts. */
export const Theme = {
  background: "#0a0a0e",
  backgroundElevated: "#18181c",
  backgroundHover: "#232327",
  text: "#f2f1f6",
  textDim: "#a9a7b5",
  danger: "#ff5c5c",
  border: "#2c2c30",
} as const;

export type ThemeColorName = "white" | "teal" | "ember";

export const THEME_COLORS: Record<ThemeColorName, { accent: string; accentHover: string }> = {
  white: { accent: "#f2f1f6", accentHover: "#d8d7db" },
  teal: { accent: "#2dd4c8", accentHover: "#55e0d6" },
  ember: { accent: "#ff5a1f", accentHover: "#ff7a3d" },
};
