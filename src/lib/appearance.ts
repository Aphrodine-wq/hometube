import type { AppearancePreferences } from "../types";

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
}
