use crate::db::{full_file_hash, ManagedDownload};
use crate::models::{DownloadJob, DownloadPreview, DownloadRequest};
use crate::AppState;
use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use url::Url;
use walkdir::WalkDir;

const FINISHED_STATUSES: &[&str] = &["complete", "failed", "cancelled"];
const PREVIEW_TIMEOUT: Duration = Duration::from_secs(20);
const PREVIEW_STDOUT_LIMIT: usize = 2 * 1024 * 1024;
const PREVIEW_STDERR_LIMIT: usize = 64 * 1024;

struct ActiveDownload {
    job_id: String,
    child: Arc<Mutex<Child>>,
}

#[derive(Default)]
pub struct DownloadRuntime {
    worker_running: AtomicBool,
    active: Mutex<Option<ActiveDownload>>,
}

impl DownloadRuntime {
    pub fn active_job_id(&self) -> Option<String> {
        self.active
            .lock()
            .as_ref()
            .map(|active| active.job_id.clone())
    }

    pub fn cancel(&self, job_id: &str) {
        let active = self.active.lock();
        if let Some(active) = active.as_ref().filter(|active| active.job_id == job_id) {
            let _ = active.child.lock().kill();
        }
    }

    pub fn shutdown(&self) {
        if let Some(active) = self.active.lock().as_ref() {
            let _ = active.child.lock().kill();
        }
    }
}

pub fn create_job(request: DownloadRequest) -> Result<DownloadJob> {
    validate_youtube_url(&request.url)?;
    if !matches!(request.quality.as_str(), "720p" | "1080p" | "best") {
        anyhow::bail!("Quality must be 720p, 1080p, or best");
    }
    let now = unix_time();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let seed = format!("{}:{}:{nonce}", request.url, std::process::id());
    Ok(DownloadJob {
        id: blake3::hash(seed.as_bytes()).to_hex()[..20].to_owned(),
        url: request.url.trim().to_owned(),
        quality: request.quality,
        status: "queued".into(),
        progress: 0.0,
        current_title: None,
        item_index: 0,
        item_count: 0,
        completed_count: 0,
        speed: None,
        eta: None,
        error: None,
        created_at: now,
        updated_at: now,
    })
}

pub fn validate_youtube_url(raw: &str) -> Result<()> {
    let url = Url::parse(raw.trim()).context("Enter a complete YouTube URL")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        anyhow::bail!("Only public HTTP or HTTPS YouTube links are supported");
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let allowed = host == "youtu.be"
        || host == "youtube.com"
        || host.ends_with(".youtube.com")
        || host == "youtube-nocookie.com"
        || host.ends_with(".youtube-nocookie.com");
    if !allowed {
        anyhow::bail!("Only YouTube and youtu.be links are supported");
    }
    Ok(())
}

pub fn preview_download(
    yt_dlp: &str,
    js_runtime: &str,
    raw_url: &str,
    youtube_auth_args: &[String],
) -> Result<DownloadPreview> {
    validate_youtube_url(raw_url)?;
    let mut command = Command::new(yt_dlp);
    command
        .args([
            "--dump-single-json",
            "--skip-download",
            "--no-warnings",
            "--no-colors",
            "--yes-playlist",
            "--playlist-end",
            "500",
        ])
        .args(youtube_auth_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(runtime) = javascript_runtime(js_runtime) {
        command.args(["--js-runtimes", &runtime]);
    }
    command.arg(raw_url.trim());
    let output = run_bounded_command(command, PREVIEW_TIMEOUT)
        .with_context(|| format!("Could not preview this URL with {yt_dlp}"))?;
    let value: Value =
        serde_json::from_slice(&output).context("yt-dlp returned invalid preview metadata")?;
    parse_download_preview(&value)
}

pub(crate) fn run_bounded_command(mut command: Command, timeout: Duration) -> Result<Vec<u8>> {
    let mut child = command
        .spawn()
        .context("The configured yt-dlp command could not be started")?;
    let stdout = child
        .stdout
        .take()
        .context("yt-dlp stdout was unavailable")?;
    let stderr = child
        .stderr
        .take()
        .context("yt-dlp stderr was unavailable")?;
    let stdout_reader = std::thread::spawn(move || read_bounded(stdout, PREVIEW_STDOUT_LIMIT));
    let stderr_reader = std::thread::spawn(move || read_bounded(stderr, PREVIEW_STDERR_LIMIT));
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            drop(stdout_reader);
            drop(stderr_reader);
            anyhow::bail!(
                "yt-dlp did not finish the preview within {} seconds",
                timeout.as_secs()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let (stdout, stdout_overflow) = stdout_reader
        .join()
        .map_err(|_| anyhow::anyhow!("Could not read yt-dlp preview output"))??;
    let (stderr, _) = stderr_reader
        .join()
        .map_err(|_| anyhow::anyhow!("Could not read yt-dlp preview errors"))??;
    if stdout_overflow {
        anyhow::bail!("yt-dlp returned too much preview metadata");
    }
    if !status.success() {
        let detail = String::from_utf8_lossy(&stderr);
        anyhow::bail!("{}", friendly_error(&detail));
    }
    Ok(stdout)
}

fn read_bounded(mut reader: impl Read, limit: usize) -> std::io::Result<(Vec<u8>, bool)> {
    let mut kept = Vec::with_capacity(limit.min(64 * 1024));
    let mut buffer = [0_u8; 16 * 1024];
    let mut overflow = false;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(kept.len());
        kept.extend_from_slice(&buffer[..read.min(remaining)]);
        overflow |= read > remaining;
    }
    Ok((kept, overflow))
}

fn parse_download_preview(value: &Value) -> Result<DownloadPreview> {
    let entries = value.get("entries").and_then(Value::as_array);
    let playlist =
        value.get("_type").and_then(Value::as_str) == Some("playlist") || entries.is_some();
    let title = value_string(value, &["title", "playlist_title"])
        .context("yt-dlp preview metadata did not include a title")?;
    let channel = value_string(value, &["channel", "uploader", "creator"]);
    let thumbnail_url = preview_thumbnail(value);
    let item_count = playlist.then(|| {
        value
            .get("playlist_count")
            .or_else(|| value.get("n_entries"))
            .and_then(Value::as_u64)
            .unwrap_or_else(|| {
                entries
                    .map(|items| items.iter().filter(|item| !item.is_null()).count() as u64)
                    .unwrap_or_default()
            })
            .min(u32::MAX as u64) as u32
    });
    let duration_secs = (!playlist)
        .then(|| value.get("duration").and_then(Value::as_f64))
        .flatten();
    let source_max_height = (!playlist)
        .then(|| {
            value
                .get("formats")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|format| format.get("height").and_then(Value::as_u64))
                .max()
                .map(|height| height.min(u32::MAX as u64) as u32)
        })
        .flatten();
    Ok(DownloadPreview {
        kind: if playlist { "playlist" } else { "video" }.into(),
        title,
        channel,
        thumbnail_url,
        item_count,
        duration_secs,
        source_max_height,
    })
}

fn preview_thumbnail(value: &Value) -> Option<String> {
    value_string(value, &["thumbnail"])
        .filter(|url| is_allowed_thumbnail_url(url))
        .or_else(|| {
            value
                .get("thumbnails")
                .and_then(Value::as_array)?
                .iter()
                .rev()
                .filter_map(|thumbnail| thumbnail.get("url").and_then(Value::as_str))
                .find(|url| is_allowed_thumbnail_url(url))
                .map(ToOwned::to_owned)
        })
}

fn is_allowed_thumbnail_url(raw: &str) -> bool {
    let Ok(url) = Url::parse(raw) else {
        return false;
    };
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(
            url.host_str().map(str::to_ascii_lowercase).as_deref(),
            Some("i.ytimg.com" | "img.youtube.com" | "yt3.ggpht.com")
        )
}

pub fn kick_worker(app: AppHandle) {
    let state = app.state::<AppState>();
    if state.downloads.worker_running.swap(true, Ordering::AcqRel) {
        return;
    }
    tauri::async_runtime::spawn_blocking(move || {
        worker_loop(&app);
        let state = app.state::<AppState>();
        state
            .downloads
            .worker_running
            .store(false, Ordering::Release);
        let has_more = state
            .db
            .lock()
            .next_queued_download()
            .ok()
            .flatten()
            .is_some();
        if has_more {
            kick_worker(app.clone());
        }
    });
}

fn worker_loop(app: &AppHandle) {
    loop {
        let next = app
            .state::<AppState>()
            .db
            .lock()
            .next_queued_download()
            .ok()
            .flatten();
        let Some(mut job) = next else { break };
        if let Err(error) = run_job(app, &mut job) {
            let current = app
                .state::<AppState>()
                .db
                .lock()
                .get_download(&job.id)
                .ok()
                .flatten();
            if current
                .as_ref()
                .is_some_and(|value| value.status == "cancelled")
            {
                continue;
            }
            job.status = "failed".into();
            job.error = Some(friendly_error(&error.to_string()));
            job.speed = None;
            job.eta = None;
            job.updated_at = unix_time();
            update_job(app, &job);
        }
    }
}

fn run_job(app: &AppHandle, job: &mut DownloadJob) -> Result<()> {
    let state = app.state::<AppState>();
    let settings = state.settings.read().clone();
    let library = PathBuf::from(&settings.library_path);
    let staging = state.download_dir.join(&job.id);
    fs::create_dir_all(&staging)?;
    recover_staged(
        app,
        job,
        &staging,
        &library,
        &settings.ffprobe_path,
        &settings.ffmpeg_path,
    )?;

    job.status = "extracting".into();
    job.error = None;
    job.updated_at = unix_time();
    update_job(app, job);

    let archive = state.download_dir.join("youtube-download-archive.txt");
    let output = staging.join("%(id)s.%(ext)s");
    let mut command = Command::new(&settings.yt_dlp_path);
    command
        .args([
            "--newline",
            "--no-colors",
            "--yes-playlist",
            "--continue",
            "--ignore-errors",
            "--embed-metadata",
            "--write-info-json",
            "--merge-output-format",
            "mp4",
            "--download-archive",
        ])
        .arg(&archive)
        .args(["--output"])
        .arg(&output)
        .args(["--format", format_selector(&job.quality)])
        .args([
            "--progress-template",
            "download:HOMETUBE_PROGRESS:%(progress._percent_str)s\t%(progress._speed_str)s\t%(progress._eta_str)s\t%(info.playlist_index)s\t%(info.playlist_count)s\t%(info.title)s",
            "--print",
            "after_move:HOMETUBE_COMPLETE:%(id)s",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(runtime) = javascript_runtime(&settings.js_runtime_path) {
        command.args(["--js-runtimes", &runtime]);
    }
    command.args(settings.youtube_auth_args());
    command.arg(&job.url);

    let mut child = command
        .spawn()
        .with_context(|| format!("Could not start {}", settings.yt_dlp_path))?;
    let stdout = child
        .stdout
        .take()
        .context("yt-dlp did not expose progress output")?;
    let stderr = child
        .stderr
        .take()
        .context("yt-dlp did not expose error output")?;
    let errors = Arc::new(Mutex::new(VecDeque::<String>::with_capacity(12)));
    let error_lines = Arc::clone(&errors);
    let error_reader = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let mut lines = error_lines.lock();
            if lines.len() == 12 {
                lines.pop_front();
            }
            lines.push_back(line);
        }
    });
    let child = Arc::new(Mutex::new(child));
    *state.downloads.active.lock() = Some(ActiveDownload {
        job_id: job.id.clone(),
        child: Arc::clone(&child),
    });

    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if let Some(progress) = line.strip_prefix("HOMETUBE_PROGRESS:") {
            apply_progress(job, progress);
            update_job(app, job);
        } else if let Some(raw_id) = line.strip_prefix("HOMETUBE_COMPLETE:") {
            let id = raw_id.trim();
            let info = read_staged_info(&staging, id)?;
            job.status = "processing".into();
            job.current_title = value_string(&info, &["title"]);
            job.speed = None;
            job.eta = None;
            job.updated_at = unix_time();
            update_job(app, job);
            import_and_register(
                app,
                &job.id,
                &info,
                &staging,
                &library,
                &settings.ffprobe_path,
                &settings.ffmpeg_path,
            )?;
            job.completed_count = job.completed_count.saturating_add(1);
            job.item_count = job.item_count.max(job.completed_count);
            job.progress = job.completed_count as f64 / job.item_count.max(1) as f64;
            job.status = "downloading".into();
            job.updated_at = unix_time();
            update_job(app, job);
            let _ = app.emit("library-dirty", ());
        }
    }
    let status = child.lock().wait()?;
    let _ = error_reader.join();
    let mut active = state.downloads.active.lock();
    if active
        .as_ref()
        .is_some_and(|active| active.job_id == job.id)
    {
        *active = None;
    }
    drop(active);

    let current = state.db.lock().get_download(&job.id)?;
    if current
        .as_ref()
        .is_some_and(|value| value.status == "cancelled")
    {
        return Ok(());
    }
    let error_text = errors.lock().iter().cloned().collect::<Vec<_>>().join("\n");
    if !status.success() || error_text.contains("ERROR:") {
        anyhow::bail!(if error_text.is_empty() {
            format!("yt-dlp exited with {status}")
        } else {
            error_text
        });
    }
    job.status = "complete".into();
    job.progress = 1.0;
    job.speed = None;
    job.eta = None;
    job.updated_at = unix_time();
    update_job(app, job);
    Ok(())
}

fn apply_progress(job: &mut DownloadJob, line: &str) {
    let mut fields = line.splitn(6, '\t');
    let item_progress = fields
        .next()
        .unwrap_or_default()
        .trim()
        .trim_end_matches('%')
        .parse::<f64>()
        .unwrap_or_default()
        / 100.0;
    job.speed = clean_field(fields.next());
    job.eta = clean_field(fields.next());
    job.item_index = parse_count(fields.next()).unwrap_or(job.completed_count + 1);
    job.item_count = parse_count(fields.next()).unwrap_or(job.item_count.max(1));
    job.current_title = clean_field(fields.next());
    job.status = "downloading".into();
    job.progress = ((job.completed_count as f64 + item_progress) / job.item_count.max(1) as f64)
        .clamp(0.0, 0.99);
    job.updated_at = unix_time();
}

fn clean_field(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "NA" && *value != "N/A")
        .map(ToOwned::to_owned)
}

fn parse_count(value: Option<&str>) -> Option<u32> {
    clean_field(value)?.parse().ok()
}

fn recover_staged(
    app: &AppHandle,
    job: &mut DownloadJob,
    staging: &Path,
    library: &Path,
    ffprobe: &str,
    ffmpeg: &str,
) -> Result<()> {
    let manifests: Vec<PathBuf> = WalkDir::new(staging)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| path.to_string_lossy().ends_with(".info.json"))
        .collect();
    for manifest in manifests {
        let info: Value = serde_json::from_slice(&fs::read(&manifest)?)?;
        let id = value_string(&info, &["id"]).unwrap_or_default();
        let source = find_staged_video(staging, &id);
        if source.is_none() {
            continue;
        }
        import_and_register(app, &job.id, &info, staging, library, ffprobe, ffmpeg)?;
        job.completed_count = job.completed_count.saturating_add(1);
        job.updated_at = unix_time();
        update_job(app, job);
        let _ = app.emit("library-dirty", ());
    }
    Ok(())
}

fn read_staged_info(staging: &Path, id: &str) -> Result<Value> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        anyhow::bail!("yt-dlp returned an invalid YouTube item ID");
    }
    let manifest = staging.join(format!("{id}.info.json"));
    serde_json::from_slice(
        &fs::read(&manifest).with_context(|| {
            format!("yt-dlp finished {id}, but its metadata file was not created")
        })?,
    )
    .context("yt-dlp created an invalid metadata file")
}

struct PendingImport {
    target: PathBuf,
    staged_source: PathBuf,
    extractor_id: String,
    provenance: ManagedDownload,
}

fn import_and_register(
    app: &AppHandle,
    job_id: &str,
    info: &Value,
    staging: &Path,
    library: &Path,
    ffprobe: &str,
    ffmpeg: &str,
) -> Result<PathBuf> {
    let pending = import_item(info, staging, library, ffprobe, ffmpeg, job_id)?;
    let registration = app
        .state::<AppState>()
        .db
        .lock()
        .register_managed_download(&pending.provenance);
    if let Err(error) = registration {
        let _ = fs::remove_file(&pending.target);
        return Err(error).context(
            "The download was placed, but HomeTube could not verify its ownership; the staged copy was kept",
        );
    }
    let _ = fs::remove_file(&pending.staged_source);
    remove_manifest(staging, &pending.extractor_id);
    Ok(pending.target)
}

fn import_item(
    info: &Value,
    staging: &Path,
    library: &Path,
    ffprobe: &str,
    ffmpeg: &str,
    job_id: &str,
) -> Result<PendingImport> {
    let id = value_string(info, &["id"]).context("Downloaded item has no YouTube ID")?;
    let source = value_string(info, &["filepath"])
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| find_staged_video(staging, &id))
        .context("yt-dlp completed without producing a video file")?;
    let canonical_staging = staging.canonicalize()?;
    let canonical_source = source.canonicalize()?;
    if !canonical_source.starts_with(&canonical_staging) {
        anyhow::bail!("yt-dlp returned a file outside its staging directory");
    }
    let channel = sanitize_component(
        &value_string(info, &["channel", "uploader", "creator"])
            .unwrap_or_else(|| "Unknown creator".into()),
        80,
    );
    let title = sanitize_component(
        &value_string(info, &["title"]).unwrap_or_else(|| "Untitled".into()),
        160,
    );
    let target_dir = library.join(channel);
    fs::create_dir_all(&target_dir)?;
    let target = target_dir.join(format!("{title} [{id}].mp4"));
    if target.exists() {
        anyhow::bail!(
            "A file already exists at {}. HomeTube will not adopt or replace it automatically",
            target.display()
        );
    }
    let temporary = target_dir.join(format!(".{id}.hometube-importing"));
    if let Ok(metadata) = fs::symlink_metadata(&temporary) {
        if metadata.file_type().is_symlink() {
            anyhow::bail!("The temporary import path is a symbolic link");
        }
        fs::remove_file(&temporary).with_context(|| {
            format!(
                "Could not clear an incomplete import at {}",
                temporary.display()
            )
        })?;
    }
    let compatible = canonical_source
        .extension()
        .and_then(|value| value.to_str())
        == Some("mp4")
        && is_compatible_mp4(&canonical_source, ffprobe)?;
    if compatible {
        fs::copy(&canonical_source, &temporary)?;
    } else {
        let output = Command::new(ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(&canonical_source)
            .args([
                "-map",
                "0:v:0",
                "-map",
                "0:a?",
                "-map_metadata",
                "0",
                "-c:v",
                "libx264",
                "-preset",
                "medium",
                "-crf",
                "22",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-b:a",
                "192k",
                "-movflags",
                "+faststart",
                "-f",
                "mp4",
            ])
            .arg(&temporary)
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("Could not start {ffmpeg}"))?;
        if !output.status.success() {
            let _ = fs::remove_file(&temporary);
            anyhow::bail!(
                "FFmpeg could not create a compatible MP4: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    fs::hard_link(&temporary, &target).with_context(|| {
        if target.exists() {
            format!(
                "A file appeared at {} while importing. HomeTube did not replace it",
                target.display()
            )
        } else {
            format!("Could not atomically place {}", target.display())
        }
    })?;
    if let Err(error) = fs::remove_file(&temporary) {
        let _ = fs::remove_file(&target);
        return Err(error).context("Could not finish atomic download placement");
    }
    let verified = (|| -> Result<PendingImport> {
        let canonical_target = target
            .canonicalize()
            .with_context(|| format!("Could not verify {}", target.display()))?;
        let canonical_library = library.canonicalize()?;
        if !canonical_target.starts_with(&canonical_library) {
            anyhow::bail!("The final download path escaped the configured library");
        }
        let metadata = fs::metadata(&canonical_target)?;
        let source_url = value_string(info, &["webpage_url", "original_url"])
            .filter(|url| validate_youtube_url(url).is_ok())
            .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={id}"));
        let media_id = blake3::hash(source_url.trim().as_bytes()).to_hex()[..20].to_owned();
        let provenance = ManagedDownload {
            media_id,
            extractor_id: id.clone(),
            source_url: Some(source_url),
            canonical_path: canonical_target.to_string_lossy().into_owned(),
            size_bytes: metadata.len(),
            blake3: full_file_hash(&canonical_target)?,
            job_id: Some(job_id.into()),
            imported_at: unix_time(),
        };
        Ok(PendingImport {
            target: canonical_target,
            staged_source: canonical_source,
            extractor_id: id,
            provenance,
        })
    })();
    if verified.is_err() {
        let _ = fs::remove_file(&target);
    }
    verified
}

fn is_compatible_mp4(path: &Path, ffprobe: &str) -> Result<bool> {
    let output = Command::new(ffprobe)
        .args(["-v", "quiet", "-print_format", "json", "-show_streams"])
        .arg(path)
        .output()?;
    if !output.status.success() {
        return Ok(false);
    }
    let probe: Value = serde_json::from_slice(&output.stdout)?;
    let streams = probe.get("streams").and_then(Value::as_array);
    let video = streams.and_then(|streams| {
        streams.iter().find(|stream| {
            stream.get("codec_type").and_then(Value::as_str) == Some("video")
                && stream
                    .pointer("/disposition/attached_pic")
                    .and_then(Value::as_i64)
                    != Some(1)
        })
    });
    let audio = streams.and_then(|streams| {
        streams
            .iter()
            .find(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("audio"))
    });
    Ok(video
        .and_then(|stream| stream.get("codec_name"))
        .and_then(Value::as_str)
        == Some("h264")
        && audio
            .and_then(|stream| stream.get("codec_name"))
            .and_then(Value::as_str)
            .is_none_or(|codec| codec == "aac"))
}

fn find_staged_video(staging: &Path, id: &str) -> Option<PathBuf> {
    ["mp4", "mkv", "webm", "mov", "m4v"]
        .into_iter()
        .map(|extension| staging.join(format!("{id}.{extension}")))
        .find(|path| path.is_file())
}

fn remove_manifest(staging: &Path, id: &str) {
    let _ = fs::remove_file(staging.join(format!("{id}.info.json")));
}

fn value_string(info: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        info.get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn sanitize_component(value: &str, max_chars: usize) -> String {
    let cleaned: String = value
        .chars()
        .map(|character| match character {
            '/' | '\\' | '\0' => ' ',
            character if character.is_control() => ' ',
            character => character,
        })
        .collect();
    let shortened: String = cleaned.chars().take(max_chars).collect();
    let value = shortened.split_whitespace().collect::<Vec<_>>().join(" ");
    let value = value.trim_matches(['.', ' ']);
    if value.is_empty() {
        "Untitled".into()
    } else {
        value.into()
    }
}

fn format_selector(quality: &str) -> &'static str {
    match quality {
        "720p" => "bv[vcodec^=avc1][height<=720]+ba[acodec^=mp4a]/b[vcodec^=avc1][acodec^=mp4a][height<=720]/bv*[height<=720]+ba/b[height<=720]",
        "best" => "bv[vcodec^=avc1]+ba[acodec^=mp4a]/b[vcodec^=avc1][acodec^=mp4a]/bv*+ba/b",
        _ => "bv[vcodec^=avc1][height<=1080]+ba[acodec^=mp4a]/b[vcodec^=avc1][acodec^=mp4a][height<=1080]/bv*[height<=1080]+ba/b[height<=1080]",
    }
}

pub(crate) fn javascript_runtime(command: &str) -> Option<String> {
    let path = Path::new(command.trim());
    let executable = path.file_name()?.to_str()?.to_ascii_lowercase();
    let runtime = if executable.starts_with("node") {
        "node"
    } else if executable.starts_with("deno") {
        "deno"
    } else if executable.starts_with("quickjs") || executable == "qjs" {
        "quickjs"
    } else if executable.starts_with("bun") {
        "bun"
    } else {
        return None;
    };
    if path.components().count() > 1 {
        Some(format!("{runtime}:{}", path.display()))
    } else {
        Some(runtime.into())
    }
}

fn update_job(app: &AppHandle, job: &DownloadJob) {
    let state = app.state::<AppState>();
    let database = state.db.lock();
    let cancelled = database
        .get_download(&job.id)
        .ok()
        .flatten()
        .is_some_and(|current| current.status == "cancelled");
    if cancelled && job.status != "cancelled" {
        return;
    }
    let _ = database.update_download(job);
    drop(database);
    let _ = app.emit("download-progress", job.clone());
}

fn friendly_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("sign in") || lower.contains("cookies") || lower.contains("age-restricted") {
        return "This video requires a signed-in YouTube session. HomeTube only downloads public videos.".into();
    }
    let lines = raw.lines().map(str::trim).filter(|line| {
        !line.is_empty()
            && *line != "<no Python frame>"
            && !line.starts_with("Traceback (most recent call last)")
            && !line.starts_with("File \"")
            && !line.starts_with('^')
    });
    if let Some(error) = lines.clone().rev().find(|line| line.starts_with("ERROR:")) {
        return error.trim_start_matches("ERROR:").trim().to_owned();
    }
    lines
        .rev()
        .find(|line| {
            !line.starts_with("Python runtime state:")
                && !line.starts_with("Current thread")
                && !line.starts_with("Extension modules:")
        })
        .unwrap_or("The download failed. Try Retry; if it continues, update yt-dlp in Settings.")
        .to_owned()
}

pub fn is_finished_status(status: &str) -> bool {
    FINISHED_STATUSES.contains(&status)
}

pub fn unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_youtube_urls_and_rejects_other_hosts() {
        assert!(validate_youtube_url("https://www.youtube.com/watch?v=abc").is_ok());
        assert!(validate_youtube_url("https://youtu.be/abc").is_ok());
        assert!(validate_youtube_url("https://music.youtube.com/watch?v=abc").is_ok());
        assert!(validate_youtube_url("https://example.com/watch?v=abc").is_err());
        assert!(validate_youtube_url("file:///tmp/video").is_err());
    }

    #[test]
    fn maps_quality_to_bounded_format_selectors() {
        assert!(format_selector("720p").contains("height<=720"));
        assert!(format_selector("1080p").contains("height<=1080"));
        assert!(format_selector("best").contains("vcodec^=avc1"));
        assert!(format_selector("best").ends_with("bv*+ba/b"));
    }

    #[test]
    fn sanitizes_output_path_components() {
        assert_eq!(sanitize_component(" ../A/B\nChannel ", 80), "A B Channel");
        assert_eq!(sanitize_component("...", 80), "Untitled");
    }

    #[test]
    fn parses_download_progress() {
        let mut job = create_job(DownloadRequest {
            url: "https://youtu.be/abc".into(),
            quality: "1080p".into(),
        })
        .unwrap();
        apply_progress(&mut job, "50.0%\t2.0MiB/s\t00:10\t2\t4\tExample");
        assert_eq!(job.current_title.as_deref(), Some("Example"));
        assert_eq!(job.item_index, 2);
        assert_eq!(job.item_count, 4);
        assert!((job.progress - 0.125).abs() < f64::EPSILON);
    }

    #[test]
    fn maps_javascript_runtime_commands() {
        assert_eq!(javascript_runtime("node").as_deref(), Some("node"));
        assert_eq!(
            javascript_runtime("/usr/bin/deno").as_deref(),
            Some("deno:/usr/bin/deno")
        );
        assert_eq!(javascript_runtime("python"), None);
    }

    #[test]
    fn parses_video_and_playlist_previews_with_safe_thumbnails() {
        let video = parse_download_preview(&serde_json::json!({
            "title": "Example",
            "channel": "Channel",
            "duration": 42.5,
            "thumbnail": "https://i.ytimg.com/vi/abc/hqdefault.jpg",
            "formats": [{"height": 720}, {"height": 1080}]
        }))
        .unwrap();
        assert_eq!(video.kind, "video");
        assert_eq!(video.duration_secs, Some(42.5));
        assert_eq!(video.source_max_height, Some(1080));
        assert!(video.thumbnail_url.is_some());

        let playlist = parse_download_preview(&serde_json::json!({
            "_type": "playlist",
            "title": "List",
            "thumbnail": "https://example.com/tracker.jpg",
            "entries": [{"id": "a"}, null, {"id": "b"}]
        }))
        .unwrap();
        assert_eq!(playlist.kind, "playlist");
        assert_eq!(playlist.item_count, Some(2));
        assert!(playlist.thumbnail_url.is_none());
        assert!(playlist.duration_secs.is_none());
    }

    #[test]
    fn validates_only_https_youtube_image_hosts() {
        assert!(is_allowed_thumbnail_url(
            "https://i.ytimg.com/vi/abc/hqdefault.jpg"
        ));
        assert!(is_allowed_thumbnail_url("https://yt3.ggpht.com/example"));
        assert!(!is_allowed_thumbnail_url(
            "http://i.ytimg.com/vi/abc/hqdefault.jpg"
        ));
        assert!(!is_allowed_thumbnail_url(
            "https://i.ytimg.com.evil.test/image.jpg"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn preview_processes_are_killed_at_the_deadline() {
        let started = Instant::now();
        let mut command = Command::new("sleep");
        command.arg("10");
        let result = run_bounded_command(command, Duration::from_millis(50));
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn bounded_reader_consumes_but_does_not_retain_excess_output() {
        let input = std::io::Cursor::new(vec![b'x'; 4096]);
        let (kept, overflow) = read_bounded(input, 128).unwrap();
        assert_eq!(kept.len(), 128);
        assert!(overflow);
    }

    #[test]
    fn reads_compact_completion_metadata_from_staging() {
        let staging = tempfile::tempdir().unwrap();
        fs::write(
            staging.path().join("abc-123.info.json"),
            br#"{"id":"abc-123","title":"Example"}"#,
        )
        .unwrap();
        let info = read_staged_info(staging.path(), "abc-123").unwrap();
        assert_eq!(value_string(&info, &["title"]).as_deref(), Some("Example"));
        assert!(read_staged_info(staging.path(), "../escape").is_err());
    }

    #[test]
    fn reports_a_useful_error_instead_of_python_frame_noise() {
        let raw = "Fatal Python error: could not acquire lock\nPython runtime state: finalizing\n<no Python frame>";
        assert_eq!(
            friendly_error(raw),
            "Fatal Python error: could not acquire lock"
        );
        assert_eq!(
            friendly_error("WARNING: skipped\nERROR: Video unavailable\n<no Python frame>"),
            "Video unavailable"
        );
    }

    #[cfg(unix)]
    #[test]
    fn atomically_imports_a_compatible_mp4() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let staging = root.path().join("staging");
        let library = root.path().join("library");
        fs::create_dir_all(&staging).unwrap();
        fs::create_dir_all(&library).unwrap();
        let source = staging.join("abc.mp4");
        fs::write(&source, b"video bytes").unwrap();
        let probe = root.path().join("fake-ffprobe");
        fs::write(
            &probe,
            "#!/bin/sh\nprintf '%s' '{\"streams\":[{\"codec_type\":\"video\",\"codec_name\":\"h264\"},{\"codec_type\":\"audio\",\"codec_name\":\"aac\"}]}'\n",
        )
        .unwrap();
        fs::set_permissions(&probe, fs::Permissions::from_mode(0o755)).unwrap();
        let info = serde_json::json!({
            "id": "abc",
            "title": "Case / Files",
            "channel": "Example Channel",
            "filepath": source,
        });
        let imported = import_item(
            &info,
            &staging,
            &library,
            probe.to_str().unwrap(),
            "unused-ffmpeg",
            "job-one",
        )
        .unwrap();
        assert_eq!(
            imported.target,
            library.join("Example Channel/Case Files [abc].mp4")
        );
        assert_eq!(fs::read(imported.target).unwrap(), b"video bytes");
        assert!(source.exists());
        assert_eq!(imported.provenance.size_bytes, 11);
        assert_eq!(imported.provenance.job_id.as_deref(), Some("job-one"));
    }

    #[cfg(unix)]
    #[test]
    fn refuses_to_adopt_an_existing_import_target() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let staging = root.path().join("staging");
        let library = root.path().join("library");
        fs::create_dir_all(&staging).unwrap();
        fs::create_dir_all(library.join("Channel")).unwrap();
        let source = staging.join("abc.mp4");
        fs::write(&source, b"new").unwrap();
        let existing = library.join("Channel/Title [abc].mp4");
        fs::write(&existing, b"existing").unwrap();
        let probe = root.path().join("fake-ffprobe");
        fs::write(
            &probe,
            "#!/bin/sh\nprintf '%s' '{\"streams\":[{\"codec_type\":\"video\",\"codec_name\":\"h264\"}]}'\n",
        )
        .unwrap();
        fs::set_permissions(&probe, fs::Permissions::from_mode(0o755)).unwrap();
        let info = serde_json::json!({
            "id": "abc",
            "title": "Title",
            "channel": "Channel",
            "filepath": source,
        });
        assert!(import_item(
            &info,
            &staging,
            &library,
            probe.to_str().unwrap(),
            "unused",
            "job"
        )
        .is_err());
        assert_eq!(fs::read(existing).unwrap(), b"existing");
        assert_eq!(fs::read(source).unwrap(), b"new");
    }
}
