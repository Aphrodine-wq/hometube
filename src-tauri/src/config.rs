use crate::models::AppSettings;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub fn defaults() -> AppSettings {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    AppSettings {
        library_path: home.join("Videos").to_string_lossy().into_owned(),
        ffmpeg_path: "ffmpeg".into(),
        ffprobe_path: "ffprobe".into(),
        yt_dlp_path: "yt-dlp".into(),
        js_runtime_path: "node".into(),
        conversion_quality: "balanced".into(),
        theme_preference: "system".into(),
        accent_preference: "cinema".into(),
        density_preference: "comfortable".into(),
        text_size_preference: "standard".into(),
        reduced_motion: false,
        library_volume_id: None,
        youtube_cookies_browser: None,
        youtube_cookies_file: None,
    }
}

pub fn validate_library(path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path);
    let metadata = fs::metadata(&path).with_context(|| {
        format!(
            "Could not open {}. If this is a storage node, make sure it is mounted",
            path.display()
        )
    })?;
    if !metadata.is_dir() {
        anyhow::bail!("{} is not a folder", path.display());
    }
    let canonical = path
        .canonicalize()
        .with_context(|| format!("Could not open {}", path.display()))?;
    let probe = canonical.join(".hometube-write-check");
    fs::write(&probe, b"HomeTube storage check")
        .with_context(|| format!("{} is not writable", canonical.display()))?;
    fs::remove_file(&probe).with_context(|| {
        format!(
            "Could not finish the storage check in {}",
            canonical.display()
        )
    })?;
    Ok(canonical)
}
