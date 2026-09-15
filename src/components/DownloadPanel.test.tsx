import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DownloadJob, YoutubeSearchItem } from "../types";
import DownloadPanel from "./DownloadPanel";

const mocks = vi.hoisted(() => ({
  enqueue: vi.fn(),
  listen: vi.fn(),
  youtubeSearch: vi.fn(),
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
    youtubeSearch: mocks.youtubeSearch,
    listen: mocks.listen,
  },
}));

const searchHit: YoutubeSearchItem = {
  videoId: "rFZHOHl-L8A",
  url: "https://www.youtube.com/watch?v=rFZHOHl-L8A",
  title: "lofi hip hop radio",
  channel: "Lofi Girl",
  durationSecs: null,
  thumbnailUrl: null,
};

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
    mocks.youtubeSearch.mockReset().mockResolvedValue([searchHit]);
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

  it("searches YouTube and queues a result at the selected quality", async () => {
    render(<DownloadPanel onOpenLibrary={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("Search YouTube"), { target: { value: "lofi radio" } });
    fireEvent.click(screen.getByRole("button", { name: "Search" }));
    expect(await screen.findByText("lofi hip hop radio")).toBeInTheDocument();
    expect(screen.getByText("Lofi Girl")).toBeInTheDocument();
    expect(mocks.youtubeSearch).toHaveBeenCalledWith("lofi radio");
    fireEvent.click(within(screen.getByRole("listitem")).getByRole("button", { name: "Download" }));
    await waitFor(() => expect(mocks.enqueue).toHaveBeenCalledWith({
      url: searchHit.url,
      quality: "720p",
    }));
  });
});
