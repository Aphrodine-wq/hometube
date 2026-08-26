use crate::models::MediaItem;
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "webm", "mov", "m4v", "avi"];
const TEMP_EXTENSIONS: &[&str] = &["part", "ytdl", "temp", "tmp"];

pub fn scan_library(
    library: &Path,
    ffprobe: &str,
    ffmpeg: &str,
    artwork_dir: &Path,
) -> Result<Vec<MediaItem>> {
    fs::create_dir_all(artwork_dir)?;
    let mut items = Vec::new();
    for entry in WalkDir::new(library)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !entry.file_type().is_file() || !is_video(path) {
            continue;
        }
        if let Ok(item) = inspect(path, ffprobe, ffmpeg, artwork_dir) {
            items.push(item);
        }
    }
    items.sort_by(|a, b| {
        b.added_at
            .cmp(&a.added_at)
            .then_with(|| a.title.cmp(&b.title))
    });
    Ok(items)
}

fn is_video(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if TEMP_EXTENSIONS.contains(&extension.as_str()) {
        return false;
    }
    VIDEO_EXTENSIONS.contains(&extension.as_str())
        && !path
            .file_name()
            .and_then(|v| v.to_str())
            .is_some_and(|name| name.starts_with('.'))
}

fn inspect(path: &Path, ffprobe: &str, ffmpeg: &str, artwork_dir: &Path) -> Result<MediaItem> {
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .with_context(|| format!("Could not inspect {}", path.display()))?;
    if !output.status.success() {
        anyhow::bail!("ffprobe rejected {}", path.display());
    }
    let probe: Value = serde_json::from_slice(&output.stdout)?;
    let format = probe.get("format").unwrap_or(&Value::Null);
    let tags = format.get("tags").unwrap_or(&Value::Null);
    let tag = |names: &[&str]| -> Option<String> {
        names.iter().find_map(|name| {
            tags.get(*name)
                .or_else(|| tags.get(name.to_ascii_uppercase()))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToOwned::to_owned)
        })
    };
    let source_url = tag(&["purl", "comment", "url"])
        .filter(|value| value.starts_with("http://") || value.starts_with("https://"));
    let id = stable_id(path, source_url.as_deref())?;
    let metadata = fs::metadata(path)?;
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default();
    let added_at = metadata
        .created()
        .or_else(|_| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(modified_at);
    let streams = probe
        .get("streams")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let video_codec = codec_for(&streams, "video");
    let audio_codec = codec_for(&streams, "audio");
    let container = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("video")
        .to_ascii_lowercase();
    let playable = is_likely_playable(&container, video_codec.as_deref(), audio_codec.as_deref());
    let artwork_path = ensure_artwork(path, &id, ffmpeg, artwork_dir)
        .map(|value| value.to_string_lossy().into_owned());

    Ok(MediaItem {
        id,
        path: path.to_string_lossy().into_owned(),
        title: tag(&["title"]).unwrap_or_else(|| display_title(path)),
        creator: tag(&["artist", "album_artist", "uploader", "author"])
            .or_else(|| {
                path.parent()
                    .and_then(Path::file_name)
                    .and_then(|value| value.to_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_else(|| "Unknown creator".into()),
        description: tag(&["synopsis", "description"]),
        source_url,
        duration_secs: format
            .get("duration")
            .and_then(Value::as_str)
            .and_then(|v| v.parse().ok())
            .unwrap_or_default(),
        added_at,
        modified_at,
        size_bytes: metadata.len(),
        video_codec,
        audio_codec,
        container,
        artwork_path,
        playable,
        favorite: false,
        progress_secs: 0.0,
        watched: false,
        conversion_path: None,
        managed_by_home_tube: false,
    })
}

fn codec_for(streams: &[Value], kind: &str) -> Option<String> {
    streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(Value::as_str) == Some(kind))
        .and_then(|stream| stream.get("codec_name"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn is_likely_playable(container: &str, video: Option<&str>, audio: Option<&str>) -> bool {
    let video_ok = matches!(video, Some("h264" | "vp8" | "vp9" | "av1"));
    let audio_ok = audio.is_none() || matches!(audio, Some("aac" | "mp3" | "opus" | "vorbis"));
    matches!(container, "mp4" | "m4v" | "webm") && video_ok && audio_ok
}

fn stable_id(path: &Path, source_url: Option<&str>) -> Result<String> {
    if let Some(url) = source_url {
        return Ok(blake3::hash(url.trim().as_bytes()).to_hex()[..20].to_owned());
    }
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut hasher = blake3::Hasher::new();
    hasher.update(&size.to_le_bytes());
    let mut buffer = vec![0u8; (size.min(1024 * 1024)) as usize];
    file.read_exact(&mut buffer)?;
    hasher.update(&buffer);
    if size > 1024 * 1024 {
        file.seek(SeekFrom::End(-(1024 * 1024)))?;
        file.read_exact(&mut buffer)?;
        hasher.update(&buffer);
    }
    Ok(hasher.finalize().to_hex()[..20].to_owned())
}

fn display_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Untitled")
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn ensure_artwork(path: &Path, id: &str, ffmpeg: &str, artwork_dir: &Path) -> Option<PathBuf> {
    let target = artwork_dir.join(format!("{id}.jpg"));
    if target.exists() {
        return Some(target);
    }
    let status = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-ss", "5"])
        .arg("-i")
        .arg(path)
        .args(["-frames:v", "1", "-vf", "scale=960:-2", "-q:v", "3", "-y"])
        .arg(&target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    status.success().then_some(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_supported_and_partial_files() {
        assert!(is_video(Path::new("movie.mp4")));
        assert!(is_video(Path::new("show.WEBM")));
        assert!(!is_video(Path::new("movie.mp4.part")));
        assert!(!is_video(Path::new("song.mp3")));
        assert!(!is_video(Path::new(".hidden.mp4")));
    }

    #[test]
    fn classifies_common_webview_codecs() {
        assert!(is_likely_playable("mp4", Some("h264"), Some("aac")));
        assert!(is_likely_playable("webm", Some("vp9"), Some("opus")));
        assert!(!is_likely_playable("mkv", Some("hevc"), Some("aac")));
    }
}
