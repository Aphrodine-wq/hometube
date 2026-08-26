use crate::db::{full_file_hash, ManagedDownload, RemovalPlanItemRecord, RemovalPlanRecord};
use crate::downloads;
use crate::models::{
    MediaOperationFailure, RemovalPreview, RemovalPreviewItem, RemovalResult, RemovedItem,
};
use crate::AppState;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECS: i64 = 5 * 60;
const SELECTED_CONFIRMATION: &str = "MOVE TO TRASH";
const ALL_CONFIRMATION: &str = "MOVE ALL TO TRASH";
const SPACE_RECOVERY_NOTE: &str =
    "Items were moved to the system Trash. Empty the system Trash to free disk space.";

pub trait TrashAdapter {
    fn move_to_trash(&self, path: &Path) -> Result<()>;
}

struct SystemTrash;

impl TrashAdapter for SystemTrash {
    fn move_to_trash(&self, path: &Path) -> Result<()> {
        trash::delete(path)
            .map_err(anyhow::Error::from)
            .with_context(|| format!("The system Trash refused {}", path.display()))
    }
}

pub fn preview_selected(state: &AppState, media_ids: Vec<String>) -> Result<RemovalPreview> {
    if media_ids.is_empty() {
        anyhow::bail!("Select at least one item to remove");
    }
    let mut seen = HashSet::new();
    let db = state.db.lock();
    let mut items = Vec::new();
    for media_id in media_ids {
        if !seen.insert(media_id.clone()) {
            continue;
        }
        let item = db
            .get_media(&media_id)?
            .with_context(|| format!("Media item {media_id} was not found"))?;
        items.push(item);
    }
    drop(db);
    create_preview(state, "selected", SELECTED_CONFIRMATION, items, false, 0)
}

pub fn preview_all_managed(state: &AppState) -> Result<RemovalPreview> {
    if state.downloads.active_job_id().is_some() {
        anyhow::bail!("Wait for the active download to finish before removing all HomeTube items");
    }
    let all_items = state.db.lock().list_media()?;
    let excluded_count = all_items
        .iter()
        .filter(|item| !item.managed_by_home_tube)
        .count()
        .min(u32::MAX as usize) as u32;
    let items = all_items
        .into_iter()
        .filter(|item| item.managed_by_home_tube)
        .collect::<Vec<_>>();
    if items.is_empty() {
        anyhow::bail!("There are no verified HomeTube-managed downloads to remove");
    }
    create_preview(
        state,
        "allManaged",
        ALL_CONFIRMATION,
        items,
        true,
        excluded_count,
    )
}

fn create_preview(
    state: &AppState,
    scope: &str,
    confirmation: &str,
    items: Vec<crate::models::MediaItem>,
    require_hash: bool,
    excluded_count: u32,
) -> Result<RemovalPreview> {
    let library = canonical_directory(Path::new(&state.settings.read().library_path), "library")?;
    let now = downloads::unix_time();
    let expires_at = now + PREVIEW_TTL_SECS;
    let token = removal_token(scope, now);
    let mut records = Vec::with_capacity(items.len());
    let mut preview_items = Vec::with_capacity(items.len());
    let db = state.db.lock();
    for item in items {
        ensure_not_busy(state, &item.id)?;
        let path = validate_file_in_root(Path::new(&item.path), &library, "media file")?;
        let metadata = fs::metadata(&path)?;
        let managed = db.managed_download_for_path(&path.to_string_lossy())?;
        if require_hash && managed.is_none() {
            anyhow::bail!(
                "{} no longer has verified HomeTube provenance; refresh the library",
                item.title
            );
        }
        let expected_hash = managed.as_ref().map(|record| record.blake3.clone());
        records.push(plan_item(
            &item,
            &path,
            &metadata,
            managed.as_ref(),
            expected_hash,
        )?);
        preview_items.push(RemovalPreviewItem {
            media_id: item.id,
            title: item.title,
            creator: item.creator,
            path: path.to_string_lossy().into_owned(),
            size_bytes: metadata.len(),
            managed_by_home_tube: managed.is_some(),
        });
    }
    drop(db);
    let plan = RemovalPlanRecord {
        token: token.clone(),
        scope: scope.into(),
        confirmation_phrase: confirmation.into(),
        expires_at,
        items: records,
    };
    state.db.lock().create_removal_plan(&plan, now)?;
    let total_bytes = preview_items.iter().map(|item| item.size_bytes).sum();
    Ok(RemovalPreview {
        token,
        scope: scope.into(),
        item_count: preview_items.len().min(u32::MAX as usize) as u32,
        items: preview_items,
        total_bytes,
        confirmation_phrase: confirmation.into(),
        eligible_count: plan.items.len().min(u32::MAX as usize) as u32,
        excluded_count,
        required_phrase: confirmation.into(),
        expires_at,
        space_recovery_note: SPACE_RECOVERY_NOTE.into(),
    })
}

fn plan_item(
    item: &crate::models::MediaItem,
    path: &Path,
    metadata: &fs::Metadata,
    managed: Option<&ManagedDownload>,
    full_blake3: Option<String>,
) -> Result<RemovalPlanItemRecord> {
    Ok(RemovalPlanItemRecord {
        media_id: item.id.clone(),
        title: item.title.clone(),
        creator: item.creator.clone(),
        primary_path: path.to_string_lossy().into_owned(),
        source_url: item.source_url.clone(),
        size_bytes: metadata.len(),
        expected_modified_at: modified_secs(metadata)?,
        artwork_path: item.artwork_path.clone(),
        conversion_path: item.conversion_path.clone(),
        managed_by_hometube: managed.is_some(),
        full_blake3,
        extractor_id: managed.map(|record| record.extractor_id.clone()),
        provenance_source_url: managed.and_then(|record| record.source_url.clone()),
        job_id: managed.and_then(|record| record.job_id.clone()),
        imported_at: managed.map(|record| record.imported_at),
        favorite: item.favorite,
        progress_secs: item.progress_secs,
        watched: item.watched,
    })
}

pub fn execute(state: &AppState, token: String, confirmation: String) -> Result<RemovalResult> {
    execute_with_adapter(state, token, confirmation, &SystemTrash)
}

fn execute_with_adapter(
    state: &AppState,
    token: String,
    confirmation: String,
    adapter: &dyn TrashAdapter,
) -> Result<RemovalResult> {
    let now = downloads::unix_time();
    let plan = state.db.lock().consume_removal_plan(token.trim(), now)?;
    if confirmation.trim() != plan.confirmation_phrase {
        anyhow::bail!(
            "Confirmation did not match. Type {} exactly",
            plan.confirmation_phrase
        );
    }
    if plan.scope == "allManaged" && state.downloads.active_job_id().is_some() {
        anyhow::bail!("A download became active; create a new removal preview after it finishes");
    }
    let library = canonical_directory(Path::new(&state.settings.read().library_path), "library")?;
    let artwork_root = canonical_directory(&state.artwork_dir, "artwork folder")?;
    let conversion_root = canonical_directory(&state.conversion_dir, "conversion folder")?;
    let mut succeeded_ids = Vec::new();
    let mut failures = Vec::new();
    let mut moved_bytes = 0_u64;
    let mut reserved = HashSet::new();
    {
        let mut operations = state.media_operations.lock();
        for item in &plan.items {
            if !operations.conversions.contains(&item.media_id)
                && operations.removals.insert(item.media_id.clone())
            {
                reserved.insert(item.media_id.clone());
            }
        }
    }

    for item in plan.items {
        if !reserved.contains(&item.media_id) {
            failures.push(MediaOperationFailure {
                media_id: item.media_id.clone(),
                error: "Another conversion or removal is using this item".into(),
            });
            continue;
        }
        if state.casting.active_media_id().as_deref() == Some(&item.media_id) {
            failures.push(MediaOperationFailure {
                media_id: item.media_id.clone(),
                error: "Stop casting this item before moving it to Trash".into(),
            });
            continue;
        }
        let current = match state.db.lock().get_media(&item.media_id) {
            Ok(Some(current)) => current,
            Ok(None) => {
                failures.push(MediaOperationFailure {
                    media_id: item.media_id.clone(),
                    error: "The media item is no longer in the library".into(),
                });
                continue;
            }
            Err(error) => {
                failures.push(failure(&item.media_id, error));
                continue;
            }
        };
        let current_path =
            match validate_file_in_root(Path::new(&current.path), &library, "media file") {
                Ok(path) => path,
                Err(error) => {
                    failures.push(failure(&item.media_id, error));
                    continue;
                }
            };
        if current_path.to_string_lossy() != item.primary_path {
            failures.push(MediaOperationFailure {
                media_id: item.media_id.clone(),
                error: "The media path changed after the preview".into(),
            });
            continue;
        }
        let metadata = match fs::metadata(&current_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                failures.push(failure(&item.media_id, error));
                continue;
            }
        };
        let modified = modified_secs(&metadata).unwrap_or(i64::MIN);
        if metadata.len() != item.size_bytes || modified != item.expected_modified_at {
            failures.push(MediaOperationFailure {
                media_id: item.media_id.clone(),
                error: "The media file changed after the preview".into(),
            });
            continue;
        }
        if plan.scope == "allManaged" {
            let verified = item
                .full_blake3
                .as_deref()
                .map(|expected| {
                    full_file_hash(&current_path)
                        .map(|actual| actual == expected)
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            if !verified {
                failures.push(MediaOperationFailure {
                    media_id: item.media_id.clone(),
                    error: "The media content no longer matches HomeTube provenance".into(),
                });
                continue;
            }
        }

        state.media_server.unregister(&item.media_id);
        if let Err(error) = adapter.move_to_trash(&current_path) {
            failures.push(failure(&item.media_id, error));
            continue;
        }
        moved_bytes = moved_bytes.saturating_add(item.size_bytes);
        succeeded_ids.push(item.media_id.clone());

        move_sidecar(
            adapter,
            item.artwork_path.as_deref(),
            &artwork_root,
            "artwork",
            &item.media_id,
            &mut failures,
        );
        move_sidecar(
            adapter,
            item.conversion_path.as_deref(),
            &conversion_root,
            "conversion",
            &item.media_id,
            &mut failures,
        );
        if let Err(error) = state
            .db
            .lock()
            .finalize_removed(&item, downloads::unix_time())
        {
            failures.push(MediaOperationFailure {
                media_id: item.media_id.clone(),
                error: format!(
                    "The file was moved to Trash, but HomeTube could not update its history: {error}"
                ),
            });
        }
    }
    {
        let mut operations = state.media_operations.lock();
        for media_id in &reserved {
            operations.removals.remove(media_id);
        }
    }
    Ok(RemovalResult {
        succeeded_ids,
        failures,
        moved_bytes,
        space_recovery_note: SPACE_RECOVERY_NOTE.into(),
    })
}

fn move_sidecar(
    adapter: &dyn TrashAdapter,
    raw_path: Option<&str>,
    root: &Path,
    label: &str,
    media_id: &str,
    failures: &mut Vec<MediaOperationFailure>,
) {
    let Some(raw_path) = raw_path else { return };
    if !Path::new(raw_path).exists() {
        return;
    }
    let path = match validate_file_in_root(Path::new(raw_path), root, label) {
        Ok(path) => path,
        Err(error) => {
            failures.push(failure(media_id, error));
            return;
        }
    };
    if let Err(error) = adapter.move_to_trash(&path) {
        failures.push(MediaOperationFailure {
            media_id: media_id.into(),
            error: format!("The video was moved, but its {label} was not: {error}"),
        });
    }
}

fn ensure_not_busy(state: &AppState, media_id: &str) -> Result<()> {
    let operations = state.media_operations.lock();
    if operations.conversions.contains(media_id) {
        anyhow::bail!("This item is currently being converted");
    }
    if operations.removals.contains(media_id) {
        anyhow::bail!("This item is already being moved to Trash");
    }
    drop(operations);
    if state.casting.active_media_id().as_deref() == Some(media_id) {
        anyhow::bail!("Stop casting this item before moving it to Trash");
    }
    Ok(())
}

pub fn recently_removed(state: &AppState) -> Result<Vec<RemovedItem>> {
    state.db.lock().recently_removed()
}

pub fn open_system_trash() -> Result<()> {
    let attempts: [(&str, &[&str]); 5] = [
        ("kioclient6", &["exec", "trash:/"]),
        ("kioclient5", &["exec", "trash:/"]),
        ("kioclient", &["exec", "trash:/"]),
        ("gio", &["open", "trash:///"]),
        ("xdg-open", &["trash:///"]),
    ];
    for (program, args) in attempts {
        match Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => return Ok(()),
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).with_context(|| format!("Could not start {program}")),
        }
    }
    anyhow::bail!("No KDE, GIO, or xdg-open Trash handler is installed")
}

pub fn open_media_folder(state: &AppState, media_id: &str) -> Result<()> {
    let item = state
        .db
        .lock()
        .get_media(media_id)?
        .context("Media was not found")?;
    let library = canonical_directory(Path::new(&state.settings.read().library_path), "library")?;
    let path = validate_file_in_root(Path::new(&item.path), &library, "media file")?;
    let folder = path
        .parent()
        .context("The media path has no parent folder")?;
    Command::new("xdg-open")
        .arg(folder)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Could not open the media folder")?;
    Ok(())
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("Could not inspect the {label} at {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        anyhow::bail!("The configured {label} must be a real directory, not a symbolic link");
    }
    path.canonicalize()
        .with_context(|| format!("Could not resolve the {label} at {}", path.display()))
}

fn validate_file_in_root(path: &Path, canonical_root: &Path, label: &str) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("Could not inspect {label} {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        anyhow::bail!("Refusing to remove a symbolic-link {label}");
    }
    if !metadata.is_file() {
        anyhow::bail!("The {label} is not a regular file");
    }
    let canonical = path
        .canonicalize()
        .with_context(|| format!("Could not resolve {label} {}", path.display()))?;
    if !canonical.starts_with(canonical_root) || canonical == canonical_root {
        anyhow::bail!("Refusing to remove a {label} outside its managed folder");
    }
    Ok(canonical)
}

fn modified_secs(metadata: &fs::Metadata) -> Result<i64> {
    Ok(metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .min(i64::MAX as u64) as i64)
}

fn removal_token(scope: &str, now: i64) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let input = format!("{scope}:{now}:{nonce}:{}", std::process::id());
    blake3::hash(input.as_bytes()).to_hex()[..32].to_owned()
}

fn failure(media_id: &str, error: impl std::fmt::Display) -> MediaOperationFailure {
    MediaOperationFailure {
        media_id: media_id.into(),
        error: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use tempfile::tempdir;

    #[derive(Default)]
    struct RecordingTrash {
        moved: Mutex<Vec<PathBuf>>,
    }

    impl TrashAdapter for RecordingTrash {
        fn move_to_trash(&self, path: &Path) -> Result<()> {
            self.moved.lock().push(path.to_owned());
            Ok(())
        }
    }

    #[test]
    fn injected_adapter_never_uses_the_real_system_trash() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("video.mp4");
        fs::write(&file, b"video").unwrap();
        let adapter = RecordingTrash::default();
        adapter.move_to_trash(&file).unwrap();
        assert_eq!(adapter.moved.lock().as_slice(), &[file.clone()]);
        assert!(file.exists());
    }

    #[test]
    fn validates_regular_files_inside_the_canonical_root() {
        let dir = tempdir().unwrap();
        let root = canonical_directory(dir.path(), "test root").unwrap();
        let inside = root.join("inside.mp4");
        fs::write(&inside, b"video").unwrap();
        assert_eq!(
            validate_file_in_root(&inside, &root, "test").unwrap(),
            inside
        );
        let outside_dir = tempdir().unwrap();
        let outside = outside_dir.path().join("outside.mp4");
        fs::write(&outside, b"video").unwrap();
        assert!(validate_file_in_root(&outside, &root, "test").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symbolic_links_even_when_the_target_is_inside() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();
        let root = canonical_directory(dir.path(), "test root").unwrap();
        let target = root.join("target.mp4");
        let link = root.join("link.mp4");
        fs::write(&target, b"video").unwrap();
        symlink(&target, &link).unwrap();
        assert!(validate_file_in_root(&link, &root, "test").is_err());
    }

    #[test]
    fn all_and_selected_confirmations_are_deliberately_different() {
        assert_eq!(SELECTED_CONFIRMATION, "MOVE TO TRASH");
        assert_eq!(ALL_CONFIRMATION, "MOVE ALL TO TRASH");
        assert!(PREVIEW_TTL_SECS <= 5 * 60);
    }
}
