mod casting;
mod config;
mod conversion;
mod db;
mod downloads;
mod media_server;
mod migration;
mod models;
mod removal;
mod scanner;
mod storage;

use crate::db::Database;
use crate::models::{
    AppSettings, AppearancePreferences, BootstrapStatus, BulkMediaResult, ConversionJob,
    DependencyStatus, DownloadJob, DownloadPreview, DownloadQueueSnapshot, DownloadRequest,
    LegacyCandidate, LibrarySnapshot, MediaOperationFailure, ProgressUpdate, RemovalPreview,
    RemovalResult, RemovedItem, StorageStatus,
};
use anyhow::{Context, Result};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::{Mutex, RwLock};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
pub(crate) struct MediaOperations {
    conversions: HashSet<String>,
    removals: HashSet<String>,
}

pub(crate) struct AppState {
    pub(crate) db: Mutex<Database>,
    pub(crate) settings: RwLock<AppSettings>,
    watcher: Mutex<Option<RecommendedWatcher>>,
    artwork_dir: PathBuf,
    conversion_dir: PathBuf,
    pub(crate) download_dir: PathBuf,
    pub(crate) downloads: downloads::DownloadRuntime,
    casting: casting::CastRuntime,
    media_server: media_server::MediaServer,
    media_operations: Mutex<MediaOperations>,
}

impl AppState {
    fn initialize(app: &AppHandle) -> Result<Self> {
        // Reserve the single-instance Cast port before touching legacy data. A running
        // ReelDeck process owns this port, so migration cannot race its SQLite WAL.
        let media_server = media_server::MediaServer::start()?;
        let data_dir = app.path().app_local_data_dir()?;
        migration::migrate_legacy_data(&data_dir)?;
        fs::create_dir_all(&data_dir)?;
        let artwork_dir = data_dir.join("artwork");
        let conversion_dir = data_dir.join("converted");
        let download_dir = data_dir.join("downloads");
        fs::create_dir_all(&artwork_dir)?;
        fs::create_dir_all(&conversion_dir)?;
        fs::create_dir_all(&download_dir)?;
        let database = Database::open(&data_dir.join("hometube.sqlite3"))?;
        let settings = database.load_settings(config::defaults())?;
        if Path::new(&settings.library_path).is_dir() {
            app.asset_protocol_scope()
                .allow_directory(&settings.library_path, true)?;
        }
        app.asset_protocol_scope()
            .allow_directory(&artwork_dir, true)?;
        app.asset_protocol_scope()
            .allow_directory(&conversion_dir, true)?;
        Ok(Self {
            db: Mutex::new(database),
            settings: RwLock::new(settings),
            watcher: Mutex::new(None),
            artwork_dir,
            conversion_dir,
            download_dir,
            downloads: downloads::DownloadRuntime::default(),
            casting: casting::CastRuntime::default(),
            media_server,
            media_operations: Mutex::new(MediaOperations::default()),
        })
    }

    fn restart_watcher(&self, app: &AppHandle) -> Result<()> {
        let library = PathBuf::from(&self.settings.read().library_path);
        if !library.is_dir() {
            *self.watcher.lock() = None;
            return Ok(());
        }
        let handle = app.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if event.is_ok() {
                    let _ = handle.emit("library-dirty", ());
                }
            })?;
        watcher.watch(&library, RecursiveMode::Recursive)?;
        *self.watcher.lock() = Some(watcher);
        Ok(())
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        self.downloads.shutdown();
    }
}

#[tauri::command]
fn get_bootstrap_status(state: State<'_, AppState>) -> BootstrapStatus {
    let settings = state.settings.read().clone();
    let mut dependencies = vec![check_dependency(
        "ytDlp",
        "yt-dlp",
        &settings.yt_dlp_path,
        true,
        "Install yt-dlp and ensure it is on PATH.",
    )];
    dependencies.push(check_dependency(
        "ffmpeg",
        "FFmpeg",
        &settings.ffmpeg_path,
        true,
        "Install ffmpeg for downloads, artwork, and conversion.",
    ));
    dependencies.push(check_dependency(
        "ffprobe",
        "FFprobe",
        &settings.ffprobe_path,
        true,
        "Install ffprobe, normally provided by ffmpeg.",
    ));
    dependencies.push(check_dependency(
        "jsRuntime",
        "JavaScript runtime",
        &settings.js_runtime_path,
        false,
        "Install Node.js or Deno for full YouTube support.",
    ));
    let ready = dependencies
        .iter()
        .filter(|dep| dep.required)
        .all(|dep| dep.available);
    BootstrapStatus {
        ready,
        dependencies,
        settings,
        storage: storage_status(&state),
    }
}

#[tauri::command]
fn get_storage_status(state: State<'_, AppState>) -> StorageStatus {
    storage_status(&state)
}

#[tauri::command]
fn get_library(state: State<'_, AppState>) -> Result<LibrarySnapshot, String> {
    let settings = state.settings.read().clone();
    let media = state.db.lock().list_media().map_err(err_string)?;
    Ok(LibrarySnapshot {
        media,
        library_path: settings.library_path,
    })
}

#[tauri::command]
async fn rescan_library(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LibrarySnapshot, String> {
    let settings = state.settings.read().clone();
    let library = PathBuf::from(&settings.library_path);
    if !library.is_dir() {
        return Err(format!(
            "Library storage is unavailable at {}. Mount the storage node or choose another folder in Settings",
            library.display()
        ));
    }
    let ffprobe = settings.ffprobe_path.clone();
    let ffmpeg = settings.ffmpeg_path.clone();
    let artwork = state.artwork_dir.clone();
    let scan_path = library.clone();
    let scanned = tauri::async_runtime::spawn_blocking(move || {
        scanner::scan_library(&scan_path, &ffprobe, &ffmpeg, &artwork)
    })
    .await
    .map_err(err_string)?
    .map_err(err_string)?;
    let media = state
        .db
        .lock()
        .replace_scan(&scanned, &library)
        .map_err(err_string)?;
    let _ = app.emit("library-updated", media.len());
    Ok(LibrarySnapshot {
        media,
        library_path: settings.library_path,
    })
}

#[tauri::command]
fn set_favorite(
    state: State<'_, AppState>,
    media_id: String,
    favorite: bool,
) -> Result<(), String> {
    let updated = state
        .db
        .lock()
        .set_favorite(&media_id, favorite)
        .map_err(err_string)?;
    updated
        .then_some(())
        .ok_or_else(|| "Media was not found".into())
}

#[tauri::command]
fn bulk_set_favorite(
    state: State<'_, AppState>,
    media_ids: Vec<String>,
    favorite: bool,
) -> BulkMediaResult {
    bulk_update(&state, media_ids, |db, id| db.set_favorite(id, favorite))
}

#[tauri::command]
fn bulk_set_watched(
    state: State<'_, AppState>,
    media_ids: Vec<String>,
    watched: bool,
) -> BulkMediaResult {
    bulk_update(&state, media_ids, |db, id| db.set_watched(id, watched))
}

#[tauri::command]
fn save_progress(state: State<'_, AppState>, update: ProgressUpdate) -> Result<(), String> {
    state.db.lock().save_progress(&update).map_err(err_string)
}

fn bulk_update(
    state: &AppState,
    media_ids: Vec<String>,
    update: impl Fn(&Database, &str) -> Result<bool>,
) -> BulkMediaResult {
    let mut succeeded_ids = Vec::new();
    let mut failures = Vec::new();
    let mut seen = HashSet::new();
    let db = state.db.lock();
    for media_id in media_ids {
        if !seen.insert(media_id.clone()) {
            continue;
        }
        match update(&db, &media_id) {
            Ok(true) => succeeded_ids.push(media_id),
            Ok(false) => failures.push(MediaOperationFailure {
                media_id,
                error: "Media was not found".into(),
            }),
            Err(error) => failures.push(MediaOperationFailure {
                media_id,
                error: error.to_string(),
            }),
        }
    }
    BulkMediaResult {
        succeeded_ids,
        failures,
    }
}

#[tauri::command]
async fn preview_download(
    state: State<'_, AppState>,
    url: String,
) -> Result<DownloadPreview, String> {
    let settings = state.settings.read().clone();
    tauri::async_runtime::spawn_blocking(move || {
        downloads::preview_download(&settings.yt_dlp_path, &settings.js_runtime_path, &url)
    })
    .await
    .map_err(err_string)?
    .map_err(err_string)
}

#[tauri::command]
fn enqueue_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: DownloadRequest,
) -> Result<DownloadJob, String> {
    storage::ensure_writes_allowed(&storage_status(&state)).map_err(err_string)?;
    let job = downloads::create_job(request).map_err(err_string)?;
    state.db.lock().insert_download(&job).map_err(err_string)?;
    let _ = app.emit("download-progress", job.clone());
    downloads::kick_worker(app);
    Ok(job)
}

#[tauri::command]
fn get_download_queue(state: State<'_, AppState>) -> Result<DownloadQueueSnapshot, String> {
    Ok(DownloadQueueSnapshot {
        jobs: state.db.lock().list_downloads().map_err(err_string)?,
        active_job_id: state.downloads.active_job_id(),
    })
}

#[tauri::command]
fn cancel_download(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> Result<(), String> {
    let mut job = state
        .db
        .lock()
        .get_download(&job_id)
        .map_err(err_string)?
        .context("Download job was not found")
        .map_err(err_string)?;
    if downloads::is_finished_status(&job.status) {
        return Ok(());
    }
    job.status = "cancelled".into();
    job.speed = None;
    job.eta = None;
    job.updated_at = downloads::unix_time();
    state.db.lock().update_download(&job).map_err(err_string)?;
    state.downloads.cancel(&job_id);
    let _ = app.emit("download-progress", job);
    Ok(())
}

#[tauri::command]
fn retry_download(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> Result<DownloadJob, String> {
    storage::ensure_writes_allowed(&storage_status(&state)).map_err(err_string)?;
    if state.downloads.active_job_id().as_deref() == Some(&job_id) {
        return Err("Wait for the cancelled process to stop before retrying".into());
    }
    let mut job = state
        .db
        .lock()
        .get_download(&job_id)
        .map_err(err_string)?
        .context("Download job was not found")
        .map_err(err_string)?;
    if !downloads::is_finished_status(&job.status) {
        return Err("Only finished downloads can be retried".into());
    }
    job.status = "queued".into();
    job.progress = if job.item_count > 0 {
        job.completed_count as f64 / job.item_count as f64
    } else {
        0.0
    };
    job.error = None;
    job.speed = None;
    job.eta = None;
    job.updated_at = downloads::unix_time();
    state.db.lock().update_download(&job).map_err(err_string)?;
    let _ = app.emit("download-progress", job.clone());
    downloads::kick_worker(app);
    Ok(job)
}

#[tauri::command]
fn clear_finished_downloads(state: State<'_, AppState>) -> Result<(), String> {
    state
        .db
        .lock()
        .clear_finished_downloads()
        .map_err(err_string)
}

#[tauri::command]
fn list_legacy_candidates(state: State<'_, AppState>) -> Result<Vec<LegacyCandidate>, String> {
    let library = PathBuf::from(&state.settings.read().library_path)
        .canonicalize()
        .map_err(err_string)?;
    let media = state.db.lock().list_media().map_err(err_string)?;
    Ok(media
        .into_iter()
        .filter(|item| !item.managed_by_home_tube)
        .filter_map(|item| {
            let path = PathBuf::from(&item.path);
            let canonical = path.canonicalize().ok()?;
            if !canonical.starts_with(&library)
                || fs::symlink_metadata(&path)
                    .ok()
                    .is_some_and(|metadata| metadata.file_type().is_symlink())
            {
                return None;
            }
            let source_extractor_id = item
                .source_url
                .as_deref()
                .filter(|url| downloads::validate_youtube_url(url).is_ok())
                .and_then(extractor_id_from_url);
            let filename_id = legacy_extractor_id_from_filename(&canonical).is_some();
            if source_extractor_id.is_none() && !filename_id {
                return None;
            }
            let reason = if source_extractor_id.is_some() {
                "YouTube metadata exists, but no verified HomeTube import record was found"
            } else {
                "The filename looks like an older HomeTube download, but ownership is unverified"
            };
            Some(LegacyCandidate {
                media_id: item.id,
                title: item.title,
                creator: item.creator,
                path: canonical.to_string_lossy().into_owned(),
                source_url: item.source_url,
                reason: reason.into(),
            })
        })
        .collect())
}

#[tauri::command]
async fn adopt_legacy_downloads(app: AppHandle, media_ids: Vec<String>) -> BulkMediaResult {
    let failed_ids = media_ids.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        adopt_legacy_downloads_inner(&state, media_ids)
    })
    .await
    .unwrap_or_else(|error| BulkMediaResult {
        succeeded_ids: Vec::new(),
        failures: failed_ids
            .into_iter()
            .map(|media_id| MediaOperationFailure {
                media_id,
                error: format!("Legacy verification could not finish: {error}"),
            })
            .collect(),
    })
}

fn adopt_legacy_downloads_inner(state: &AppState, media_ids: Vec<String>) -> BulkMediaResult {
    let library = match PathBuf::from(&state.settings.read().library_path).canonicalize() {
        Ok(path) => path,
        Err(error) => {
            return BulkMediaResult {
                succeeded_ids: Vec::new(),
                failures: media_ids
                    .into_iter()
                    .map(|media_id| MediaOperationFailure {
                        media_id,
                        error: format!("The library is unavailable: {error}"),
                    })
                    .collect(),
            };
        }
    };
    let mut succeeded_ids = Vec::new();
    let mut failures = Vec::new();
    let mut seen = HashSet::new();
    for media_id in media_ids {
        if !seen.insert(media_id.clone()) {
            continue;
        }
        let result = (|| -> Result<()> {
            let item = state
                .db
                .lock()
                .get_media(&media_id)?
                .context("Media was not found")?;
            if item.managed_by_home_tube {
                anyhow::bail!("This item is already managed by HomeTube");
            }
            let raw_path = PathBuf::from(&item.path);
            let metadata = fs::symlink_metadata(&raw_path)?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                anyhow::bail!("Legacy candidates must be regular files, not symbolic links");
            }
            let path = raw_path.canonicalize()?;
            if !path.starts_with(&library) || path == library {
                anyhow::bail!("The selected file is outside the current library");
            }
            let source_url = item
                .source_url
                .filter(|url| downloads::validate_youtube_url(url).is_ok());
            let extractor_id = source_url
                .as_deref()
                .and_then(extractor_id_from_url)
                .or_else(|| legacy_extractor_id_from_filename(&path))
                .context("The selected file no longer has a recognizable YouTube ID")?;
            let metadata = fs::metadata(&path)?;
            let record = crate::db::ManagedDownload {
                media_id: item.id,
                extractor_id,
                source_url,
                canonical_path: path.to_string_lossy().into_owned(),
                size_bytes: metadata.len(),
                blake3: crate::db::full_file_hash(&path)?,
                job_id: None,
                imported_at: downloads::unix_time(),
            };
            state.db.lock().register_managed_download(&record)
        })();
        match result {
            Ok(()) => succeeded_ids.push(media_id),
            Err(error) => failures.push(MediaOperationFailure {
                media_id,
                error: error.to_string(),
            }),
        }
    }
    BulkMediaResult {
        succeeded_ids,
        failures,
    }
}

fn extractor_id_from_url(raw: &str) -> Option<String> {
    let url = url::Url::parse(raw).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    let id = if host == "youtu.be" {
        url.path_segments()?
            .find(|part| !part.is_empty())?
            .to_owned()
    } else {
        url.query_pairs()
            .find(|(key, _)| key == "v")
            .map(|(_, value)| value.into_owned())?
    };
    valid_extractor_id(&id).then_some(id)
}

fn legacy_extractor_id_from_filename(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let start = stem.rfind('[')? + 1;
    let id = stem.get(start..)?.strip_suffix(']')?;
    valid_extractor_id(id).then(|| id.to_owned())
}

fn valid_extractor_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

#[tauri::command]
fn preview_remove_media(
    state: State<'_, AppState>,
    media_ids: Vec<String>,
) -> Result<RemovalPreview, String> {
    removal::preview_selected(&state, media_ids).map_err(err_string)
}

#[tauri::command]
fn preview_remove_all_hometube(state: State<'_, AppState>) -> Result<RemovalPreview, String> {
    removal::preview_all_managed(&state).map_err(err_string)
}

#[tauri::command]
async fn execute_removal(
    app: AppHandle,
    token: String,
    confirmation: String,
) -> Result<RemovalResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        removal::execute(&state, token, confirmation)
    })
    .await
    .map_err(err_string)?
    .map_err(err_string)
}

#[tauri::command]
fn get_recently_removed(state: State<'_, AppState>) -> Result<Vec<RemovedItem>, String> {
    removal::recently_removed(&state).map_err(err_string)
}

#[tauri::command]
fn open_system_trash() -> Result<(), String> {
    removal::open_system_trash().map_err(err_string)
}

#[tauri::command]
fn open_media_folder(state: State<'_, AppState>, media_id: String) -> Result<(), String> {
    removal::open_media_folder(&state, &media_id).map_err(err_string)
}

#[tauri::command]
fn get_playback_url(state: State<'_, AppState>, media_id: String) -> Result<String, String> {
    let item = state
        .db
        .lock()
        .get_media(&media_id)
        .map_err(err_string)?
        .context("Media was not found")
        .map_err(err_string)?;
    let path = item
        .conversion_path
        .as_deref()
        .filter(|path| Path::new(path).is_file())
        .unwrap_or(&item.path);
    state
        .media_server
        .url_for(&media_id, Path::new(path))
        .map_err(err_string)
}

#[tauri::command]
async fn discover_cast_devices(
    state: State<'_, AppState>,
) -> Result<Vec<casting::CastDeviceInfo>, String> {
    state.casting.discover().await.map_err(err_string)
}

#[tauri::command]
async fn start_cast(
    state: State<'_, AppState>,
    request: casting::CastStartRequest,
) -> Result<casting::CastStatus, String> {
    let item = state
        .db
        .lock()
        .get_media(&request.media_id)
        .map_err(err_string)?
        .context("Media was not found")
        .map_err(err_string)?;
    casting::validate_device(&request.device).map_err(err_string)?;
    let path = item
        .conversion_path
        .as_deref()
        .filter(|path| Path::new(path).is_file())
        .map(PathBuf::from)
        .or_else(|| cast_compatible(&item).then(|| PathBuf::from(&item.path)))
        .context("This video needs a compatible H.264/AAC copy before it can be cast")
        .map_err(err_string)?;
    let url = state
        .media_server
        .url_for_cast(&request.media_id, &path, &request.device.ip)
        .map_err(err_string)?;
    state
        .casting
        .start(
            &request,
            &url,
            &item.title,
            &item.creator,
            item.duration_secs,
        )
        .await
        .map_err(err_string)
}

#[tauri::command]
async fn load_next_cast(
    state: State<'_, AppState>,
    media_id: String,
    position_secs: f64,
) -> Result<casting::CastStatus, String> {
    let item = state
        .db
        .lock()
        .get_media(&media_id)
        .map_err(err_string)?
        .context("Media was not found")
        .map_err(err_string)?;
    let path = item
        .conversion_path
        .as_deref()
        .filter(|path| Path::new(path).is_file())
        .map(PathBuf::from)
        .or_else(|| cast_compatible(&item).then(|| PathBuf::from(&item.path)))
        .context("The next video needs a compatible H.264/AAC copy before it can be cast")
        .map_err(err_string)?;
    let status = state.casting.status().await.map_err(err_string)?;
    let device_ip = state
        .media_server
        .last_cast_device_ip()
        .context("No Cast destination is available")
        .map_err(err_string)?;
    let url = state
        .media_server
        .url_for_cast(&media_id, &path, &device_ip)
        .map_err(err_string)?;
    if !status.connected {
        return Err("No Cast device is connected".into());
    }
    state
        .casting
        .load_next(
            &media_id,
            &url,
            &item.title,
            &item.creator,
            item.duration_secs,
            position_secs,
        )
        .await
        .map_err(err_string)
}

#[tauri::command]
async fn get_cast_status(state: State<'_, AppState>) -> Result<casting::CastStatus, String> {
    state.casting.status().await.map_err(err_string)
}

#[tauri::command]
async fn set_cast_playing(
    state: State<'_, AppState>,
    playing: bool,
) -> Result<casting::CastStatus, String> {
    state.casting.set_playing(playing).await.map_err(err_string)
}

#[tauri::command]
async fn seek_cast(
    state: State<'_, AppState>,
    position_secs: f64,
) -> Result<casting::CastStatus, String> {
    state.casting.seek(position_secs).await.map_err(err_string)
}

#[tauri::command]
async fn set_cast_volume(
    state: State<'_, AppState>,
    volume: f32,
) -> Result<casting::CastStatus, String> {
    state.casting.set_volume(volume).await.map_err(err_string)
}

#[tauri::command]
async fn set_cast_muted(
    state: State<'_, AppState>,
    muted: bool,
) -> Result<casting::CastStatus, String> {
    state.casting.set_muted(muted).await.map_err(err_string)
}

#[tauri::command]
async fn stop_cast(state: State<'_, AppState>) -> Result<casting::CastStatus, String> {
    state.casting.stop().await.map_err(err_string)
}

fn cast_compatible(item: &crate::models::MediaItem) -> bool {
    matches!(item.container.as_str(), "mp4" | "m4v")
        && item.video_codec.as_deref() == Some("h264")
        && matches!(item.audio_codec.as_deref(), None | Some("aac" | "mp3"))
}

fn validate_appearance(preferences: &AppearancePreferences) -> Result<(), String> {
    if !matches!(
        preferences.theme_preference.as_str(),
        "system" | "dark" | "light"
    ) {
        return Err("Theme must be System, Dark, or Light".into());
    }
    if !matches!(
        preferences.accent_preference.as_str(),
        "cinema" | "amber" | "teal" | "blue" | "violet"
    ) {
        return Err("Accent must be Cinema, Amber, Teal, Blue, or Violet".into());
    }
    if !matches!(
        preferences.density_preference.as_str(),
        "comfortable" | "compact"
    ) {
        return Err("Density must be Comfortable or Compact".into());
    }
    if !matches!(
        preferences.text_size_preference.as_str(),
        "small" | "standard" | "large"
    ) {
        return Err("Text size must be Small, Standard, or Large".into());
    }
    Ok(())
}

#[tauri::command]
fn save_appearance(
    state: State<'_, AppState>,
    preferences: AppearancePreferences,
) -> Result<AppSettings, String> {
    validate_appearance(&preferences)?;
    let mut settings = state.settings.read().clone();
    settings.theme_preference = preferences.theme_preference;
    settings.accent_preference = preferences.accent_preference;
    settings.density_preference = preferences.density_preference;
    settings.text_size_preference = preferences.text_size_preference;
    settings.reduced_motion = preferences.reduced_motion;
    state
        .db
        .lock()
        .save_settings(&settings)
        .map_err(err_string)?;
    *state.settings.write() = settings.clone();
    Ok(settings)
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    mut settings: AppSettings,
) -> Result<BootstrapStatus, String> {
    validate_command_setting(&mut settings.ffmpeg_path, "FFmpeg")?;
    validate_command_setting(&mut settings.ffprobe_path, "FFprobe")?;
    validate_command_setting(&mut settings.yt_dlp_path, "yt-dlp")?;
    validate_command_setting(&mut settings.js_runtime_path, "JavaScript runtime")?;
    if !matches!(
        settings.conversion_quality.as_str(),
        "compact" | "balanced" | "quality"
    ) {
        return Err("Conversion quality must be Compact, Balanced, or Quality".into());
    }
    validate_appearance(&AppearancePreferences {
        theme_preference: settings.theme_preference.clone(),
        accent_preference: settings.accent_preference.clone(),
        density_preference: settings.density_preference.clone(),
        text_size_preference: settings.text_size_preference.clone(),
        reduced_motion: settings.reduced_motion,
    })?;
    let library = config::validate_library(&settings.library_path).map_err(err_string)?;
    settings.library_path = library.to_string_lossy().into_owned();
    settings.library_volume_id = Some(storage::volume_id(&library).map_err(err_string)?);
    app.asset_protocol_scope()
        .allow_directory(&library, true)
        .map_err(err_string)?;
    state
        .db
        .lock()
        .save_settings(&settings)
        .map_err(err_string)?;
    *state.settings.write() = settings;
    state.restart_watcher(&app).map_err(err_string)?;
    Ok(get_bootstrap_status(state))
}

fn validate_command_setting(value: &mut String, label: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(format!(
            "{label} command cannot be empty or contain control characters"
        ));
    }
    *value = trimmed.to_owned();
    Ok(())
}

#[tauri::command]
fn start_conversion(
    app: AppHandle,
    state: State<'_, AppState>,
    media_id: String,
) -> Result<ConversionJob, String> {
    storage::ensure_writes_allowed(&storage_status(&state)).map_err(err_string)?;
    {
        let mut operations = state.media_operations.lock();
        if operations.removals.contains(&media_id) {
            return Err("This media item is being moved to Trash".into());
        }
        if !operations.conversions.insert(media_id.clone()) {
            return Err("This media item is already being converted".into());
        }
    }
    let item = state
        .db
        .lock()
        .get_media(&media_id)
        .map_err(|error| {
            state.media_operations.lock().conversions.remove(&media_id);
            err_string(error)
        })?
        .context("Media was not found")
        .map_err(|error| {
            state.media_operations.lock().conversions.remove(&media_id);
            err_string(error)
        })?;
    let settings = state.settings.read().clone();
    let conversion_dir = state.conversion_dir.clone();
    let initial = ConversionJob {
        media_id: media_id.clone(),
        status: "queued".into(),
        progress: 0.0,
        output_path: None,
        error: None,
    };
    let _ = app.emit("conversion-progress", initial.clone());
    let active_media_id = media_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = conversion::convert(
            &app,
            &settings.ffmpeg_path,
            Path::new(&item.path),
            &conversion_dir,
            &media_id,
            item.duration_secs,
            &settings.conversion_quality,
        );
        let state = app.state::<AppState>();
        match result {
            Ok(path) => {
                let value = path.to_string_lossy().into_owned();
                let _ = app.asset_protocol_scope().allow_file(&path);
                let _ = state.db.lock().set_conversion(&media_id, &value);
                let _ = app.emit(
                    "conversion-progress",
                    ConversionJob {
                        media_id,
                        status: "complete".into(),
                        progress: 1.0,
                        output_path: Some(value),
                        error: None,
                    },
                );
            }
            Err(error) => {
                let _ = app.emit(
                    "conversion-progress",
                    ConversionJob {
                        media_id,
                        status: "failed".into(),
                        progress: 0.0,
                        output_path: None,
                        error: Some(error.to_string()),
                    },
                );
            }
        }
        state
            .media_operations
            .lock()
            .conversions
            .remove(&active_media_id);
    });
    Ok(initial)
}

fn check_dependency(
    key: &str,
    label: &str,
    command: &str,
    required: bool,
    hint: &str,
) -> DependencyStatus {
    let path = resolve_command(command);
    let version = path.as_ref().and_then(|resolved| {
        Command::new(resolved)
            .arg("--version")
            .stdin(Stdio::null())
            .output()
            .ok()
            .and_then(|out| {
                let text = if out.stdout.is_empty() {
                    out.stderr
                } else {
                    out.stdout
                };
                String::from_utf8(text)
                    .ok()
                    .and_then(|value| value.lines().next().map(str::to_owned))
            })
    });
    DependencyStatus {
        key: key.into(),
        label: label.into(),
        available: path.is_some(),
        path: path.map(|value| value.to_string_lossy().into_owned()),
        version,
        required,
        hint: hint.into(),
    }
}

fn resolve_command(command: &str) -> Option<PathBuf> {
    let direct = PathBuf::from(command);
    if direct.components().count() > 1 {
        return direct.is_file().then_some(direct);
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(command))
            .find(|path| path.is_file())
    })
}

fn err_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn storage_status(state: &AppState) -> StorageStatus {
    let settings = state.settings.read().clone();
    let (count, bytes) = state.db.lock().library_totals().unwrap_or_default();
    storage::status(&settings, count, bytes)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::initialize(app.handle())?;
            state.restart_watcher(app.handle())?;
            state.db.lock().recover_downloads(downloads::unix_time())?;
            app.manage(state);
            downloads::kick_worker(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_bootstrap_status,
            get_storage_status,
            get_library,
            rescan_library,
            set_favorite,
            bulk_set_favorite,
            bulk_set_watched,
            save_progress,
            preview_download,
            enqueue_download,
            get_download_queue,
            cancel_download,
            retry_download,
            clear_finished_downloads,
            list_legacy_candidates,
            adopt_legacy_downloads,
            preview_remove_media,
            preview_remove_all_hometube,
            execute_removal,
            get_recently_removed,
            open_system_trash,
            open_media_folder,
            get_playback_url,
            discover_cast_devices,
            start_cast,
            load_next_cast,
            get_cast_status,
            set_cast_playing,
            seek_cast,
            set_cast_volume,
            set_cast_muted,
            stop_cast,
            save_appearance,
            save_settings,
            start_conversion,
        ])
        .run(tauri::generate_context!())
        .expect("error while running HomeTube");
}
