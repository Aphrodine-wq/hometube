import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { MediaItem } from "../types";
import { MediaCard } from "./MediaCard";

const item: MediaItem = {
  id: "one", path: "/tmp/one.mp4", title: "First Light", creator: "North Channel",
  description: null, sourceUrl: null, durationSecs: 100, addedAt: 1, modifiedAt: 1,
  sizeBytes: 10, videoCodec: "h264", audioCodec: "aac", container: "mp4",
  artworkPath: null, playable: true, favorite: false, progressSecs: 50, watched: false,
  conversionPath: null, managedByHomeTube: false,
};

describe("MediaCard", () => {
  it("exposes playback and favorite actions", () => {
    const play = vi.fn();
    const favorite = vi.fn();
    render(<MediaCard item={item} onPlay={play} onFavorite={favorite} />);
    fireEvent.click(screen.getByRole("button", { name: "Play First Light" }));
    fireEvent.click(screen.getByRole("button", { name: "Add First Light to favorites" }));
    expect(play).toHaveBeenCalledWith(item);
    expect(favorite).toHaveBeenCalledWith(item);
  });
});
