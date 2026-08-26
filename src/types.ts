export type View = "home" | "library" | "favorites" | "discover" | "removed" | "settings";

export interface MediaItem {
  id: string;
  path: string;
  title: string;
  creator: string;
  description: string | null;
  sourceUrl: string | null;
  durationSecs: number;
  addedAt: number;
  modifiedAt: number;
  sizeBytes: number;
  videoCodec: string | null;
  audioCodec: string | null;
  container: string;
  artworkPath: string | null;
  playable: boolean;
  favorite: boolean;
  progressSecs: number;
  watched: boolean;
  conversionPath: string | null;
  managedByHomeTube: boolean;
}

export interface LibrarySnapshot {
  media: MediaItem[];
  libraryPath: string;
}

export interface AppSettings {
  libraryPath: string;
  ffmpegPath: string;
  ffprobePath: string;
  ytDlpPath: string;
  jsRuntimePath: string;
  conversionQuality: "compact" | "balanced" | "quality";
  themePreference: "system" | "dark" | "light";
  accentPreference: AccentPreference;
  densityPreference: DensityPreference;
  textSizePreference: TextSizePreference;
  reducedMotion: boolean;
  libraryVolumeId: string | null;
}

export type AccentPreference = "cinema" | "amber" | "teal" | "blue" | "violet";
export type DensityPreference = "comfortable" | "compact";
export type TextSizePreference = "small" | "standard" | "large";

export interface AppearancePreferences {
  themePreference: AppSettings["themePreference"];
  accentPreference: AccentPreference;
  densityPreference: DensityPreference;
  textSizePreference: TextSizePreference;
  reducedMotion: boolean;
}

export interface StorageStatus {
  path: string;
  available: boolean;
  totalBytes: number;
  availableBytes: number;
  usedBytes: number;
  freePercent: number;
  libraryBytes: number;
  mediaCount: number;
  estimatedAdditionalItems: number | null;
  warning: boolean;
  protectionActive: boolean;
  volumeId: string | null;
  message: string | null;
}

export interface DependencyStatus {
  key: string;
  label: string;
  available: boolean;
  path: string | null;
  version: string | null;
  required: boolean;
  hint: string;
}

export interface BootstrapStatus {
  ready: boolean;
  dependencies: DependencyStatus[];
  settings: AppSettings;
  storage: StorageStatus;
}

export interface ConversionJob {
  mediaId: string;
  status: "queued" | "running" | "complete" | "failed";
  progress: number;
  outputPath: string | null;
  error: string | null;
}

export interface CastDeviceInfo {
  id: string;
  name: string;
  model: string | null;
  ip: string;
  port: number;
  available: boolean;
}

export interface CastStatus {
  connected: boolean;
  deviceName: string | null;
  mediaId: string | null;
  state: "disconnected" | "connected" | "idle" | "playing" | "paused" | "buffering";
  currentTime: number;
  duration: number;
  volume: number;
  muted: boolean;
  idleReason: "finished" | "cancelled" | "interrupted" | "error" | "unknown" | null;
}

export type DownloadQuality = "720p" | "1080p" | "best";
export type DownloadStatus = "queued" | "extracting" | "downloading" | "processing" | "complete" | "failed" | "cancelled";

export interface DownloadRequest {
  url: string;
  quality: DownloadQuality;
}

export interface DownloadJob {
  id: string;
  url: string;
  quality: DownloadQuality;
  status: DownloadStatus;
  progress: number;
  currentTitle: string | null;
  itemIndex: number;
  itemCount: number;
  completedCount: number;
  speed: string | null;
  eta: string | null;
  error: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface DownloadQueueSnapshot {
  jobs: DownloadJob[];
  activeJobId: string | null;
}

export interface DownloadPreview {
  kind: "video" | "playlist";
  title: string;
  channel: string | null;
  thumbnailUrl: string | null;
  itemCount: number | null;
  durationSecs: number | null;
  sourceMaxHeight: number | null;
}

export interface LegacyCandidate {
  mediaId: string;
  title: string;
  creator: string;
  path: string;
  sourceUrl: string | null;
  reason: string;
}

export interface BulkFailure {
  mediaId: string;
  error: string;
}

export interface BulkMediaResult {
  succeededIds: string[];
  failures: BulkFailure[];
}

export interface RemovalPreviewItem {
  mediaId: string;
  title: string;
  creator: string;
  path: string;
  sizeBytes: number;
  managedByHomeTube: boolean;
}

export interface RemovalPreview {
  token: string;
  items: RemovalPreviewItem[];
  totalBytes: number;
  eligibleCount: number;
  excludedCount: number;
  requiredPhrase: string;
}

export interface RemovalResult extends BulkMediaResult {}

export interface RemovedItem {
  id: string;
  mediaId: string;
  title: string;
  creator: string;
  originalPath: string;
  sizeBytes: number;
  removedAt: number;
  status: "removed" | "restored" | "missing" | "failed";
  error: string | null;
}
