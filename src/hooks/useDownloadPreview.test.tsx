import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { bridge } from "../lib/bridge";
import { useDownloadPreview } from "./useDownloadPreview";

vi.mock("../lib/bridge", () => ({
  bridge: { previewDownload: vi.fn() },
}));

const preview = {
  kind: "video" as const,
  title: "A preview",
  channel: "Channel",
  thumbnailUrl: null,
  itemCount: null,
  durationSecs: 120,
  sourceMaxHeight: 1080,
};

describe("useDownloadPreview", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(bridge.previewDownload).mockReset().mockResolvedValue(preview);
  });

  afterEach(() => vi.useRealTimers());

  it("debounces valid YouTube links", async () => {
    const { result } = renderHook(() => useDownloadPreview("https://youtu.be/example"));
    expect(bridge.previewDownload).not.toHaveBeenCalled();
    await act(async () => vi.advanceTimersByTime(499));
    expect(bridge.previewDownload).not.toHaveBeenCalled();
    await act(async () => {
      vi.advanceTimersByTime(1);
      await Promise.resolve();
    });
    expect(result.current.status).toBe("ready");
    expect(bridge.previewDownload).toHaveBeenCalledWith("https://youtu.be/example");
  });

  it("clears preview state for malformed links", () => {
    const { result } = renderHook(() => useDownloadPreview("not a url"));
    expect(result.current.status).toBe("idle");
    expect(result.current.isValidUrl).toBe(false);
  });
});
