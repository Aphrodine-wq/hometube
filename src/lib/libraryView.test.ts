import { describe, expect, it } from "vitest";
import type { MediaItem } from "../types";
import { applyLibraryView } from "./libraryView";
import { DEFAULT_LIBRARY_VIEW } from "./uiPreferences";

const media = (overrides: Partial<MediaItem>): MediaItem => ({
  id: "one",
  path: "/tmp/one.mp4",
  title: "One",
  creator: "Channel",
  description: null,
  sourceUrl: null,
  durationSecs: 60,
  addedAt: 1,
  modifiedAt: 1,
  sizeBytes: 10,
  videoCodec: "h264",
  audioCodec: "aac",
  container: "mp4",
  artworkPath: null,
  playable: true,
  favorite: false,
  progressSecs: 0,
  watched: false,
  conversionPath: null,
  managedByHomeTube: false,
  ...overrides,
});

describe("applyLibraryView", () => {
  it("filters by origin and viewing state", () => {
    const items = [
      media({ id: "local" }),
      media({ id: "managed", managedByHomeTube: true, progressSecs: 20 }),
      media({ id: "watched", managedByHomeTube: true, watched: true }),
    ];
    expect(applyLibraryView(items, {
      ...DEFAULT_LIBRARY_VIEW,
      managed: "hometube",
      viewing: "watching",
    }).map((item) => item.id)).toEqual(["managed"]);
  });

  it("sorts without mutating the input", () => {
    const items = [media({ id: "b", title: "Beta" }), media({ id: "a", title: "Alpha" })];
    expect(applyLibraryView(items, { ...DEFAULT_LIBRARY_VIEW, sort: "title" }).map((item) => item.id)).toEqual(["a", "b"]);
    expect(items.map((item) => item.id)).toEqual(["b", "a"]);
  });
});
