use crate::models::{AppSettings, StorageStatus};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const WARNING_PERCENT: f64 = 15.0;
const PROTECTION_PERCENT: f64 = 5.0;

pub fn volume_id(path: &Path) -> Result<String> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Could not inspect storage at {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!("unix-device:{}", metadata.dev()))
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        Ok(path.to_string_lossy().into_owned())
    }
}

pub fn status(settings: &AppSettings, media_count: u64, library_bytes: u64) -> StorageStatus {
    let path = Path::new(&settings.library_path);
    let unavailable = |message: String| StorageStatus {
        path: settings.library_path.clone(),
        available: false,
        total_bytes: 0,
        available_bytes: 0,
        used_bytes: 0,
        free_percent: 0.0,
        library_bytes,
        media_count,
        estimated_additional_items: None,
        warning: true,
        protection_active: true,
        volume_id: None,
        message: Some(message),
    };
    if !path.is_dir() {
        return unavailable("Library storage is unavailable. Mount the storage node or choose another folder in Settings".into());
    }
    let current_volume = match volume_id(path) {
        Ok(id) => id,
        Err(error) => return unavailable(error.to_string()),
    };
    if settings
        .library_volume_id
        .as_deref()
        .is_some_and(|expected| expected != current_volume)
    {
        return unavailable("The library path is on a different volume than the one you selected. Mount the expected storage node before writing media".into());
    }
    let stats = match fs2::statvfs(path) {
        Ok(stats) => stats,
        Err(error) => return unavailable(format!("Could not read storage capacity: {error}")),
    };
    let total_bytes = stats.total_space();
    let available_bytes = stats.available_space();
    let used_bytes = total_bytes.saturating_sub(available_bytes);
    let free_percent = if total_bytes > 0 {
        available_bytes as f64 / total_bytes as f64 * 100.0
    } else {
        0.0
    };
    let average = (media_count > 0).then(|| library_bytes / media_count.max(1));
    let estimated_additional_items = average
        .filter(|value| *value > 0)
        .map(|value| available_bytes / value);
    let protection_active = free_percent < PROTECTION_PERCENT;
    let warning = free_percent < WARNING_PERCENT;
    let message = if protection_active {
        Some("Storage protection is active below 5% free. New downloads and conversions are paused; existing media is never deleted".into())
    } else if warning {
        Some(
            "Library storage is below 15% free. Consider adding or moving to a storage node soon"
                .into(),
        )
    } else {
        None
    };
    StorageStatus {
        path: settings.library_path.clone(),
        available: true,
        total_bytes,
        available_bytes,
        used_bytes,
        free_percent,
        library_bytes,
        media_count,
        estimated_additional_items,
        warning,
        protection_active,
        volume_id: Some(current_volume),
        message,
    }
}

pub fn ensure_writes_allowed(status: &StorageStatus) -> Result<()> {
    if !status.available {
        anyhow::bail!(status
            .message
            .clone()
            .unwrap_or_else(|| "Library storage is unavailable".into()));
    }
    if status.protection_active {
        anyhow::bail!(status
            .message
            .clone()
            .unwrap_or_else(|| "Library storage protection is active".into()));
    }
    Ok(())
}
