import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppearancePreferences,
  AppSettings,
  BootstrapStatus,
  BulkMediaResult,
  CastDeviceInfo,
  CastStatus,
  ConversionJob,
  DownloadJob,
  DownloadPreview,
  DownloadQueueSnapshot,
  DownloadRequest,
  LegacyCandidate,
  LibrarySnapshot,
  RemovalPreview,
  RemovalResult,
  RemovedItem,
  StorageStatus,
} from "../types";

const isTauri = "__TAURI_INTERNALS__" in window;

const browserSettings: AppSettings = {
  libraryPath: "~/Videos",
  ffmpegPath: "ffmpeg",
  ffprobePath: "ffprobe",
  ytDlpPath: "yt-dlp",
  jsRuntimePath: "node",
  conversionQuality: "balanced",
  themePreference: "system",
  accentPreference: "cinema",
  densityPreference: "comfortable",
  textSizePreference: "standard",
  reducedMotion: false,
  libraryVolumeId: null,
};

const browserStorage: StorageStatus = {
  path: "~/Videos", available: false, totalBytes: 0, availableBytes: 0, usedBytes: 0,
  freePercent: 0, libraryBytes: 0, mediaCount: 0, estimatedAdditionalItems: null,
  warning: true, protectionActive: true, volumeId: null, message: "Storage status is available in the desktop app",
};

export const bridge = {
  async bootstrap(): Promise<BootstrapStatus> {
    if (!isTauri) return { ready: false, dependencies: [], settings: browserSettings, storage: browserStorage };
    return invoke("get_bootstrap_status");
  },
  async library(): Promise<LibrarySnapshot> {
    if (!isTauri) return { media: [], libraryPath: browserSettings.libraryPath };
    return invoke("get_library");
  },
  async rescan(): Promise<LibrarySnapshot> {
    if (!isTauri) return { media: [], libraryPath: browserSettings.libraryPath };
    return invoke("rescan_library");
  },
  setFavorite(mediaId: string, favorite: boolean): Promise<void> {
    return isTauri ? invoke("set_favorite", { mediaId, favorite }) : Promise.resolve();
  },
  saveProgress(mediaId: string, positionSecs: number, durationSecs: number): Promise<void> {
    return isTauri
      ? invoke("save_progress", { update: { mediaId, positionSecs, durationSecs } })
      : Promise.resolve();
  },
  saveSettings(settings: AppSettings): Promise<BootstrapStatus> {
    return isTauri ? invoke("save_settings", { settings }) : Promise.resolve({ ready: false, dependencies: [], settings, storage: browserStorage });
  },
  saveAppearance(preferences: AppearancePreferences): Promise<AppSettings> {
    return isTauri
      ? invoke("save_appearance", { preferences })
      : Promise.resolve({ ...browserSettings, ...preferences });
  },
  storageStatus(): Promise<StorageStatus> {
    return isTauri ? invoke("get_storage_status") : Promise.resolve(browserStorage);
  },
  startConversion(mediaId: string): Promise<ConversionJob> {
    return invoke("start_conversion", { mediaId });
  },
  downloadQueue(): Promise<DownloadQueueSnapshot> {
    return isTauri ? invoke("get_download_queue") : Promise.resolve({ jobs: [], activeJobId: null });
  },
  enqueueDownload(request: DownloadRequest): Promise<DownloadJob> {
    return invoke("enqueue_download", { request });
  },
  previewDownload(url: string): Promise<DownloadPreview> {
    return isTauri
      ? invoke("preview_download", { url })
      : Promise.resolve({
          kind: url.includes("playlist") ? "playlist" : "video",
          title: "YouTube preview is available in the desktop app",
          channel: "HomeTube",
          thumbnailUrl: null,
          itemCount: url.includes("playlist") ? 1 : null,
          durationSecs: null,
          sourceMaxHeight: 1080,
        });
  },
  cancelDownload(jobId: string): Promise<void> {
    return invoke("cancel_download", { jobId });
  },
  retryDownload(jobId: string): Promise<DownloadJob> {
    return invoke("retry_download", { jobId });
  },
  clearFinishedDownloads(): Promise<void> {
    return invoke("clear_finished_downloads");
  },
  listLegacyCandidates(): Promise<LegacyCandidate[]> {
    return isTauri ? invoke("list_legacy_candidates") : Promise.resolve([]);
  },
  adoptLegacyDownloads(mediaIds: string[]): Promise<BulkMediaResult> {
    return isTauri
      ? invoke("adopt_legacy_downloads", { mediaIds })
      : Promise.resolve({ succeededIds: mediaIds, failures: [] });
  },
  bulkSetFavorite(mediaIds: string[], favorite: boolean): Promise<BulkMediaResult> {
    return isTauri
      ? invoke("bulk_set_favorite", { mediaIds, favorite })
      : Promise.resolve({ succeededIds: mediaIds, failures: [] });
  },
  bulkSetWatched(mediaIds: string[], watched: boolean): Promise<BulkMediaResult> {
    return isTauri
      ? invoke("bulk_set_watched", { mediaIds, watched })
      : Promise.resolve({ succeededIds: mediaIds, failures: [] });
  },
  previewRemoveMedia(mediaIds: string[]): Promise<RemovalPreview> {
    return invoke("preview_remove_media", { mediaIds });
  },
  previewRemoveAllHomeTube(): Promise<RemovalPreview> {
    return invoke("preview_remove_all_hometube");
  },
  executeRemoval(token: string, confirmation: string): Promise<RemovalResult> {
    return invoke("execute_removal", { token, confirmation });
  },
  recentlyRemoved(): Promise<RemovedItem[]> {
    return isTauri ? invoke("get_recently_removed") : Promise.resolve([]);
  },
  openSystemTrash(): Promise<void> {
    return isTauri ? invoke("open_system_trash") : Promise.resolve();
  },
  openMediaFolder(mediaId: string): Promise<void> {
    return isTauri ? invoke("open_media_folder", { mediaId }) : Promise.resolve();
  },
  playbackUrl(mediaId: string, fallbackPath: string): Promise<string> {
    return isTauri ? invoke("get_playback_url", { mediaId }) : Promise.resolve(fallbackPath);
  },
  discoverCastDevices(): Promise<CastDeviceInfo[]> {
    return isTauri ? invoke("discover_cast_devices") : Promise.resolve([]);
  },
  startCast(device: CastDeviceInfo, mediaId: string, positionSecs: number): Promise<CastStatus> {
    return invoke("start_cast", { request: { device, mediaId, positionSecs } });
  },
  castStatus(): Promise<CastStatus> {
    return isTauri
      ? invoke("get_cast_status")
      : Promise.resolve({ connected: false, deviceName: null, mediaId: null, state: "disconnected", currentTime: 0, duration: 0, volume: 0, muted: false, idleReason: null });
  },
  loadNextCast(mediaId: string, positionSecs = 0): Promise<CastStatus> {
    return invoke("load_next_cast", { mediaId, positionSecs });
  },
  setCastPlaying(playing: boolean): Promise<CastStatus> {
    return invoke("set_cast_playing", { playing });
  },
  seekCast(positionSecs: number): Promise<CastStatus> {
    return invoke("seek_cast", { positionSecs });
  },
  setCastVolume(volume: number): Promise<CastStatus> {
    return invoke("set_cast_volume", { volume });
  },
  setCastMuted(muted: boolean): Promise<CastStatus> {
    return invoke("set_cast_muted", { muted });
  },
  stopCast(): Promise<CastStatus> {
    return invoke("stop_cast");
  },
  mediaUrl(path: string): string {
    return isTauri ? convertFileSrc(path) : path;
  },
  listen<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
    return isTauri ? listen<T>(event, ({ payload }) => handler(payload)) : Promise.resolve(() => undefined);
  },
  isTauri,
};
