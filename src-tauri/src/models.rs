use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub id: String,
    pub path: String,
    pub title: String,
    pub creator: String,
    pub description: Option<String>,
    pub source_url: Option<String>,
    pub duration_secs: f64,
    pub added_at: i64,
    pub modified_at: i64,
    pub size_bytes: u64,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub container: String,
    pub artwork_path: Option<String>,
    pub playable: bool,
    pub favorite: bool,
    pub progress_secs: f64,
    pub watched: bool,
    pub conversion_path: Option<String>,
    pub managed_by_home_tube: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    pub media: Vec<MediaItem>,
    pub library_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub library_path: String,
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub yt_dlp_path: String,
    pub js_runtime_path: String,
    pub conversion_quality: String,
    pub theme_preference: String,
    pub accent_preference: String,
    pub density_preference: String,
    pub text_size_preference: String,
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default)]
    pub library_volume_id: Option<String>,
    /// Optional browser whose YouTube cookies are passed to yt-dlp
    /// (`--cookies-from-browser`). Empty when unused.
    #[serde(default)]
    pub youtube_cookies_browser: Option<String>,
    /// Optional exported cookies.txt passed to yt-dlp (`--cookies`).
    #[serde(default)]
    pub youtube_cookies_file: Option<String>,
}

impl AppSettings {
    /// Extra yt-dlp arguments that carry the configured YouTube session.
    pub fn youtube_auth_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        if let Some(browser) = self.youtube_cookies_browser.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
            args.push("--cookies-from-browser".to_owned());
            args.push(browser.to_owned());
        }
        if let Some(file) = self.youtube_cookies_file.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
            args.push("--cookies".to_owned());
            args.push(file.to_owned());
        }
        args
    }

    pub fn normalized(mut self) -> Self {
        let trim_optional = |value: Option<String>| -> Option<String> {
            value.map(|inner| inner.trim().to_owned()).filter(|inner| !inner.is_empty())
        };
        self.youtube_cookies_browser = trim_optional(self.youtube_cookies_browser)
            .filter(|browser| matches!(browser.as_str(), "firefox" | "chrome" | "chromium" | "brave" | "edge" | "opera" | "safari" | "vivaldi" | "whale" | "librewolf"));
        self.youtube_cookies_file = trim_optional(self.youtube_cookies_file);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearancePreferences {
    pub theme_preference: String,
    pub accent_preference: String,
    pub density_preference: String,
    pub text_size_preference: String,
    pub reduced_motion: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatus {
    pub path: String,
    pub available: bool,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub free_percent: f64,
    pub library_bytes: u64,
    pub media_count: u64,
    pub estimated_additional_items: Option<u64>,
    pub warning: bool,
    pub protection_active: bool,
    pub volume_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    pub key: String,
    pub label: String,
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub required: bool,
    pub hint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatus {
    pub ready: bool,
    pub dependencies: Vec<DependencyStatus>,
    pub settings: AppSettings,
    pub storage: StorageStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionJob {
    pub media_id: String,
    pub status: String,
    pub progress: f64,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub url: String,
    pub quality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJob {
    pub id: String,
    pub url: String,
    pub quality: String,
    pub status: String,
    pub progress: f64,
    pub current_title: Option<String>,
    pub item_index: u32,
    pub item_count: u32,
    pub completed_count: u32,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadQueueSnapshot {
    pub jobs: Vec<DownloadJob>,
    pub active_job_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressUpdate {
    pub media_id: String,
    pub position_secs: f64,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadPreview {
    pub kind: String,
    pub title: String,
    pub channel: Option<String>,
    pub thumbnail_url: Option<String>,
    pub item_count: Option<u32>,
    pub duration_secs: Option<f64>,
    pub source_max_height: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyCandidate {
    pub media_id: String,
    pub title: String,
    pub creator: String,
    pub path: String,
    pub source_url: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaOperationFailure {
    pub media_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkMediaResult {
    pub succeeded_ids: Vec<String>,
    pub failures: Vec<MediaOperationFailure>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovalPreviewItem {
    pub media_id: String,
    pub title: String,
    pub creator: String,
    pub path: String,
    pub size_bytes: u64,
    pub managed_by_home_tube: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovalPreview {
    pub token: String,
    pub scope: String,
    pub items: Vec<RemovalPreviewItem>,
    pub item_count: u32,
    pub total_bytes: u64,
    pub confirmation_phrase: String,
    pub eligible_count: u32,
    pub excluded_count: u32,
    pub required_phrase: String,
    pub expires_at: i64,
    pub space_recovery_note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovalResult {
    pub succeeded_ids: Vec<String>,
    pub failures: Vec<MediaOperationFailure>,
    pub moved_bytes: u64,
    pub space_recovery_note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovedItem {
    pub id: String,
    pub media_id: String,
    pub title: String,
    pub creator: String,
    pub original_path: String,
    pub source_url: Option<String>,
    pub size_bytes: u64,
    pub managed_by_home_tube: bool,
    pub removed_at: i64,
    pub restored_at: Option<i64>,
    pub status: String,
    pub error: Option<String>,
    pub space_recovery_note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YoutubeSearchItem {
    pub video_id: String,
    pub url: String,
    pub title: String,
    pub channel: String,
    pub duration_secs: Option<u64>,
    pub thumbnail_url: Option<String>,
}
