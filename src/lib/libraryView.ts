import type { MediaItem } from "../types";
import type { LibraryViewPreferences } from "./uiPreferences";

export function applyLibraryView(items: MediaItem[], preferences: LibraryViewPreferences) {
  const filtered = items.filter((item) => {
    if (preferences.creator && item.creator !== preferences.creator) return false;
    if (preferences.format && item.container !== preferences.format) return false;
    if (preferences.managed === "hometube" && !item.managedByHomeTube) return false;
    if (preferences.managed === "local" && item.managedByHomeTube) return false;
    if (preferences.viewing === "watched" && !item.watched) return false;
    if (preferences.viewing === "watching" && (item.watched || item.progressSecs <= 0)) return false;
    if (preferences.viewing === "unwatched" && (item.watched || item.progressSecs > 0)) return false;
    return true;
  });

  return [...filtered].sort((a, b) => {
    if (preferences.sort === "title") return a.title.localeCompare(b.title);
    if (preferences.sort === "creator") {
      return a.creator.localeCompare(b.creator) || a.title.localeCompare(b.title);
    }
    if (preferences.sort === "duration") return b.durationSecs - a.durationSecs || a.title.localeCompare(b.title);
    if (preferences.sort === "size") return b.sizeBytes - a.sizeBytes || a.title.localeCompare(b.title);
    return b.addedAt - a.addedAt || a.title.localeCompare(b.title);
  });
}

export function activeLibraryFilterCount(preferences: LibraryViewPreferences) {
  return [
    preferences.viewing !== "all",
    Boolean(preferences.creator),
    Boolean(preferences.format),
    preferences.managed !== "all",
  ].filter(Boolean).length;
}
