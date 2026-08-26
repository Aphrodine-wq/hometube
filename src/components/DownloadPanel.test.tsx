import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DownloadJob } from "../types";
import DownloadPanel from "./DownloadPanel";

const mocks = vi.hoisted(() => ({
  enqueue: vi.fn(),
  listen: vi.fn(),
}));

vi.mock("../lib/bridge", () => ({
  bridge: {
    downloadQueue: vi.fn().mockResolvedValue({ jobs: [], activeJobId: null }),
    previewDownload: vi.fn().mockResolvedValue({
      kind: "video", title: "Example", channel: "Channel", thumbnailUrl: null,
      itemCount: null, durationSecs: 60, sourceMaxHeight: 1080,
    }),
    enqueueDownload: mocks.enqueue,
    cancelDownload: vi.fn().mockResolvedValue(undefined),
    retryDownload: vi.fn(),
    clearFinishedDownloads: vi.fn().mockResolvedValue(undefined),
    listen: mocks.listen,
  },
}));

const queued: DownloadJob = {
  id: "job-one",
  url: "https://youtu.be/abc",
  quality: "1080p",
  status: "queued",
  progress: 0,
  currentTitle: null,
  itemIndex: 0,
  itemCount: 0,
  completedCount: 0,
  speed: null,
  eta: null,
  error: null,
  createdAt: 1,
  updatedAt: 1,
};

describe("DownloadPanel", () => {
  afterEach(cleanup);

  beforeEach(() => {
    mocks.enqueue.mockReset().mockResolvedValue(queued);
    mocks.listen.mockReset().mockResolvedValue(() => undefined);
  });

  it("queues a YouTube playlist at the selected quality", async () => {
    render(<DownloadPanel onOpenLibrary={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("YouTube URL"), {
      target: { value: "https://www.youtube.com/playlist?list=PL123" },
    });
    fireEvent.change(screen.getByLabelText("Quality"), { target: { value: "720p" } });
    fireEvent.click(screen.getByRole("button", { name: "Download" }));
    await waitFor(() => expect(mocks.enqueue).toHaveBeenCalledWith({
      url: "https://www.youtube.com/playlist?list=PL123",
      quality: "720p",
    }));
    expect(await screen.findByText("Queued YouTube link")).toBeInTheDocument();
  });

  it("rejects non-YouTube URLs before invoking the backend", async () => {
    render(<DownloadPanel onOpenLibrary={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("YouTube URL"), {
      target: { value: "https://example.com/video" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Download" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("youtube.com or youtu.be");
    expect(mocks.enqueue).not.toHaveBeenCalled();
  });
});
