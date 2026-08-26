import type { DownloadQuality } from "../types";

export type LibrarySort = "recent" | "title" | "creator" | "duration" | "size";
export type ViewingFilter = "all" | "unwatched" | "watching" | "watched";
export type ManagedFilter = "all" | "hometube" | "local";

export interface LibraryViewPreferences {
  sort: LibrarySort;
  viewing: ViewingFilter;
  creator: string;
  format: string;
  managed: ManagedFilter;
}

const LIBRARY_KEY = "hometube:library-view:v1";
const DOWNLOAD_KEY = "hometube:download-quality:v1";

export const DEFAULT_LIBRARY_VIEW: LibraryViewPreferences = {
  sort: "recent",
  viewing: "all",
  creator: "",
  format: "",
  managed: "all",
};

const SORTS = new Set<LibrarySort>(["recent", "title", "creator", "duration", "size"]);
const VIEWING = new Set<ViewingFilter>(["all", "unwatched", "watching", "watched"]);
const MANAGED = new Set<ManagedFilter>(["all", "hometube", "local"]);
const QUALITIES = new Set<DownloadQuality>(["720p", "1080p", "best"]);

function storage() {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
}

export function loadLibraryView(): LibraryViewPreferences {
  try {
    const parsed = JSON.parse(storage()?.getItem(LIBRARY_KEY) || "{}") as Partial<LibraryViewPreferences>;
    return {
      sort: SORTS.has(parsed.sort as LibrarySort) ? parsed.sort as LibrarySort : "recent",
      viewing: VIEWING.has(parsed.viewing as ViewingFilter) ? parsed.viewing as ViewingFilter : "all",
      creator: typeof parsed.creator === "string" ? parsed.creator : "",
      format: typeof parsed.format === "string" ? parsed.format : "",
      managed: MANAGED.has(parsed.managed as ManagedFilter) ? parsed.managed as ManagedFilter : "all",
    };
  } catch {
    return DEFAULT_LIBRARY_VIEW;
  }
}

export function saveLibraryView(value: LibraryViewPreferences) {
  storage()?.setItem(LIBRARY_KEY, JSON.stringify(value));
}

export function loadDownloadQuality(): DownloadQuality {
  const value = storage()?.getItem(DOWNLOAD_KEY) as DownloadQuality | null;
  return value && QUALITIES.has(value) ? value : "1080p";
}

export function saveDownloadQuality(value: DownloadQuality) {
  storage()?.setItem(DOWNLOAD_KEY, value);
}
