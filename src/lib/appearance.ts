import type { AppearancePreferences } from "../types";
import { saveAppearanceMirror } from "./uiPreferences";

export const DEFAULT_APPEARANCE: AppearancePreferences = {
  themePreference: "system",
  accentPreference: "cinema",
  densityPreference: "comfortable",
  textSizePreference: "standard",
  reducedMotion: false,
};

// Keep in sync with --canvas in styles.css and the inline bootstrap script in index.html.
const THEME_COLORS = { dark: "#0c0d0f", light: "#f3f1ed" } as const;

export function appearanceFromSettings(settings: AppearancePreferences): AppearancePreferences {
  return {
    themePreference: settings.themePreference,
    accentPreference: settings.accentPreference,
    densityPreference: settings.densityPreference,
    textSizePreference: settings.textSizePreference,
    reducedMotion: settings.reducedMotion,
  };
}

export function applyAppearance(preferences: AppearancePreferences) {
  const root = document.documentElement;
  const media = window.matchMedia("(prefers-color-scheme: light)");
  const resolved = preferences.themePreference === "system"
    ? (media.matches ? "light" : "dark")
    : preferences.themePreference;
  root.dataset.theme = resolved;
  root.dataset.accent = preferences.accentPreference;
  root.dataset.density = preferences.densityPreference;
  root.dataset.textSize = preferences.textSizePreference;
  root.dataset.reducedMotion = preferences.reducedMotion ? "true" : "false";
  root.style.colorScheme = resolved;
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", THEME_COLORS[resolved]);
  saveAppearanceMirror(preferences);
}
