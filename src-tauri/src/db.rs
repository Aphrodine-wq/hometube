use crate::models::{AppSettings, DownloadJob, MediaItem, ProgressUpdate, RemovedItem};
use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const SCHEMA_VERSION: i64 = 2;

const MIGRATION_1: &str = "
CREATE TABLE IF NOT EXISTS media (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    creator TEXT NOT NULL,
    description TEXT,
    source_url TEXT,
    duration_secs REAL NOT NULL DEFAULT 0,
    added_at INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    video_codec TEXT,
    audio_codec TEXT,
    container TEXT NOT NULL,
    artwork_path TEXT,
    playable INTEGER NOT NULL DEFAULT 0,
    favorite INTEGER NOT NULL DEFAULT 0,
    progress_secs REAL NOT NULL DEFAULT 0,
    watched INTEGER NOT NULL DEFAULT 0,
    conversion_path TEXT
);
CREATE INDEX IF NOT EXISTS media_creator_idx ON media(creator);
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS download_jobs (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    quality TEXT NOT NULL,
    status TEXT NOT NULL,
    progress REAL NOT NULL DEFAULT 0,
    current_title TEXT,
    item_index INTEGER NOT NULL DEFAULT 0,
    item_count INTEGER NOT NULL DEFAULT 0,
    completed_count INTEGER NOT NULL DEFAULT 0,
    speed TEXT,
    eta TEXT,
    error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS download_jobs_status_idx
    ON download_jobs(status, created_at);
";

const MIGRATION_2: &str = "
CREATE TABLE IF NOT EXISTS managed_downloads (
    media_id TEXT NOT NULL,
    extractor_id TEXT NOT NULL,
    source_url TEXT,
    canonical_path TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL,
    blake3 TEXT NOT NULL,
    job_id TEXT,
    imported_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS managed_downloads_media_idx
    ON managed_downloads(media_id);
CREATE TABLE IF NOT EXISTS removed_items (
    history_id INTEGER PRIMARY KEY AUTOINCREMENT,
    media_id TEXT NOT NULL,
    title TEXT NOT NULL,
    creator TEXT NOT NULL,
    original_path TEXT NOT NULL,
    source_url TEXT,
    size_bytes INTEGER NOT NULL,
    managed_by_hometube INTEGER NOT NULL,
    full_blake3 TEXT,
    extractor_id TEXT,
    job_id TEXT,
    imported_at INTEGER,
    favorite INTEGER NOT NULL,
    progress_secs REAL NOT NULL,
    watched INTEGER NOT NULL,
    artwork_path TEXT,
    conversion_path TEXT,
    removed_at INTEGER NOT NULL,
    restored_at INTEGER
);
CREATE INDEX IF NOT EXISTS removed_items_path_idx
    ON removed_items(original_path, restored_at, removed_at);
CREATE TABLE IF NOT EXISTS removal_operations (
    token TEXT PRIMARY KEY,
    scope TEXT NOT NULL,
    confirmation_phrase TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    consumed_at INTEGER
);
CREATE TABLE IF NOT EXISTS removal_operation_items (
    token TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    media_id TEXT NOT NULL,
    title TEXT NOT NULL,
    creator TEXT NOT NULL,
    primary_path TEXT NOT NULL,
    source_url TEXT,
    size_bytes INTEGER NOT NULL,
    expected_modified_at INTEGER NOT NULL,
    artwork_path TEXT,
    conversion_path TEXT,
    managed_by_hometube INTEGER NOT NULL,
    full_blake3 TEXT,
    extractor_id TEXT,
    provenance_source_url TEXT,
    job_id TEXT,
    imported_at INTEGER,
    favorite INTEGER NOT NULL,
    progress_secs REAL NOT NULL,
    watched INTEGER NOT NULL,
    PRIMARY KEY(token, ordinal)
);
";

#[derive(Debug, Clone)]
pub struct ManagedDownload {
    pub media_id: String,
    pub extractor_id: String,
    pub source_url: Option<String>,
    pub canonical_path: String,
    pub size_bytes: u64,
    pub blake3: String,
    pub job_id: Option<String>,
    pub imported_at: i64,
}

#[derive(Debug, Clone)]
pub struct RemovalPlanItemRecord {
    pub media_id: String,
    pub title: String,
    pub creator: String,
    pub primary_path: String,
    pub source_url: Option<String>,
    pub size_bytes: u64,
    pub expected_modified_at: i64,
    pub artwork_path: Option<String>,
    pub conversion_path: Option<String>,
    pub managed_by_hometube: bool,
    pub full_blake3: Option<String>,
    pub extractor_id: Option<String>,
    pub provenance_source_url: Option<String>,
    pub job_id: Option<String>,
    pub imported_at: Option<i64>,
    pub favorite: bool,
    pub progress_secs: f64,
    pub watched: bool,
}

#[derive(Debug, Clone)]
pub struct RemovalPlanRecord {
    pub token: String,
    pub scope: String,
    pub confirmation_phrase: String,
    pub expires_at: i64,
    pub items: Vec<RemovalPlanItemRecord>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let mut conn =
            Connection::open(path).with_context(|| format!("Could not open {}", path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn load_settings(&self, defaults: AppSettings) -> Result<AppSettings> {
        let mut values = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT key, value FROM settings")?;
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (key, value) = row?;
            values.insert(key, value);
        }
        Ok(AppSettings {
            library_path: take_valid(
                &mut values,
                "library_path",
                defaults.library_path,
                valid_nonempty,
            ),
            ffmpeg_path: take_valid(
                &mut values,
                "ffmpeg_path",
                defaults.ffmpeg_path,
                valid_command,
            ),
            ffprobe_path: take_valid(
                &mut values,
                "ffprobe_path",
                defaults.ffprobe_path,
                valid_command,
            ),
            yt_dlp_path: take_valid(
                &mut values,
                "yt_dlp_path",
                defaults.yt_dlp_path,
                valid_command,
            ),
            js_runtime_path: take_valid(
                &mut values,
                "js_runtime_path",
                defaults.js_runtime_path,
                valid_command,
            ),
            conversion_quality: take_valid(
                &mut values,
                "conversion_quality",
                defaults.conversion_quality,
                |value| matches!(value, "compact" | "balanced" | "quality"),
            ),
            theme_preference: take_valid(
                &mut values,
                "theme_preference",
                defaults.theme_preference,
                |value| matches!(value, "system" | "dark" | "light"),
            ),
            accent_preference: take_valid(
                &mut values,
                "accent_preference",
                defaults.accent_preference,
                |value| matches!(value, "cinema" | "amber" | "teal" | "blue" | "violet"),
            ),
            density_preference: take_valid(
                &mut values,
                "density_preference",
                defaults.density_preference,
                |value| matches!(value, "comfortable" | "compact"),
            ),
            text_size_preference: take_valid(
                &mut values,
                "text_size_preference",
                defaults.text_size_preference,
                |value| matches!(value, "small" | "standard" | "large"),
            ),
            reduced_motion: values
                .remove("reduced_motion")
                .and_then(|value| parse_bool(&value))
                .unwrap_or(defaults.reduced_motion),
            library_volume_id: values
                .remove("library_volume_id")
                .or(defaults.library_volume_id),
            youtube_cookies_browser: values
                .remove("youtube_cookies_browser")
                .or(defaults.youtube_cookies_browser),
            youtube_cookies_file: values
                .remove("youtube_cookies_file")
                .or(defaults.youtube_cookies_file),
        })
    }

    pub fn save_settings(&mut self, settings: &AppSettings) -> Result<()> {
        let tx = self.conn.transaction()?;
        let reduced_motion = settings.reduced_motion.to_string();
        let values = [
            ("library_path", settings.library_path.as_str()),
            ("ffmpeg_path", settings.ffmpeg_path.as_str()),
            ("ffprobe_path", settings.ffprobe_path.as_str()),
            ("yt_dlp_path", settings.yt_dlp_path.as_str()),
            ("js_runtime_path", settings.js_runtime_path.as_str()),
            ("conversion_quality", settings.conversion_quality.as_str()),
            ("theme_preference", settings.theme_preference.as_str()),
            ("accent_preference", settings.accent_preference.as_str()),
            ("density_preference", settings.density_preference.as_str()),
            (
                "text_size_preference",
                settings.text_size_preference.as_str(),
            ),
            ("reduced_motion", reduced_motion.as_str()),
        ];
        for (key, value) in values {
            tx.execute(
                "INSERT INTO settings(key, value) VALUES(?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![key, value],
            )?;
        }
        match settings.library_volume_id.as_deref() {
            Some(value) => {
                tx.execute(
                    "INSERT INTO settings(key, value) VALUES('library_volume_id', ?1)
                     ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    [value],
                )?;
            }
            None => {
                tx.execute("DELETE FROM settings WHERE key='library_volume_id'", [])?;
            }
        }
        for key in ["youtube_cookies_browser", "youtube_cookies_file"] {
            let value = if key == "youtube_cookies_browser" {
                settings.youtube_cookies_browser.as_deref()
            } else {
                settings.youtube_cookies_file.as_deref()
            };
            match value {
                Some(value) => {
                    tx.execute(
                        &format!(
                            "INSERT INTO settings(key, value) VALUES('{key}', ?1)
                             ON CONFLICT(key) DO UPDATE SET value=excluded.value"
                        ),
                        [value],
                    )?;
                }
                None => {
                    tx.execute(
                        &format!("DELETE FROM settings WHERE key='{key}'"),
                        [],
                    )?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn replace_scan(
        &mut self,
        scanned: &[MediaItem],
        _library_path: &Path,
    ) -> Result<Vec<MediaItem>> {
        let tx = self.conn.transaction()?;
        for scanned_item in scanned {
            let mut item = scanned_item.clone();
            if let Ok(canonical) = Path::new(&item.path).canonicalize() {
                item.path = canonical.to_string_lossy().into_owned();
            }
            let restored = restored_state(&tx, &item.path)?;
            if let Some(state) = restored.as_ref() {
                item.favorite = state.favorite;
                item.progress_secs = state.progress_secs;
                item.watched = state.watched;
            }
            tx.execute(
                "INSERT INTO media (
                    id,path,title,creator,description,source_url,duration_secs,added_at,modified_at,
                    size_bytes,video_codec,audio_codec,container,artwork_path,playable,
                    favorite,progress_secs,watched,conversion_path
                 ) VALUES (
                    ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19
                 )
                 ON CONFLICT(id) DO UPDATE SET
                    path=excluded.path,title=excluded.title,creator=excluded.creator,
                    description=excluded.description,source_url=excluded.source_url,
                    duration_secs=excluded.duration_secs,modified_at=excluded.modified_at,
                    size_bytes=excluded.size_bytes,video_codec=excluded.video_codec,
                    audio_codec=excluded.audio_codec,container=excluded.container,
                    artwork_path=excluded.artwork_path,playable=excluded.playable",
                params![
                    item.id,
                    item.path,
                    item.title,
                    item.creator,
                    item.description,
                    item.source_url,
                    item.duration_secs,
                    item.added_at,
                    item.modified_at,
                    item.size_bytes as i64,
                    item.video_codec,
                    item.audio_codec,
                    item.container,
                    item.artwork_path,
                    item.playable as i32,
                    item.favorite as i32,
                    item.progress_secs,
                    item.watched as i32,
                    item.conversion_path,
                ],
            )?;
            tx.execute(
                "UPDATE managed_downloads SET media_id=?1 WHERE canonical_path=?2",
                params![item.id, item.path],
            )?;
            if let Some(state) = restored {
                tx.execute(
                    "UPDATE removed_items SET restored_at=?2 WHERE history_id=?1",
                    params![state.history_id, crate::downloads::unix_time()],
                )?;
                if state.managed_by_hometube
                    && state.full_blake3.as_deref().is_some_and(|expected| {
                        full_file_hash(Path::new(&item.path))
                            .ok()
                            .as_deref()
                            .is_some_and(|actual| actual == expected)
                    })
                {
                    tx.execute(
                        "INSERT INTO managed_downloads (
                            media_id,extractor_id,source_url,canonical_path,size_bytes,blake3,
                            job_id,imported_at
                         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
                         ON CONFLICT(canonical_path) DO UPDATE SET
                            media_id=excluded.media_id,extractor_id=excluded.extractor_id,
                            source_url=excluded.source_url,size_bytes=excluded.size_bytes,
                            blake3=excluded.blake3,job_id=excluded.job_id,
                            imported_at=excluded.imported_at",
                        params![
                            item.id,
                            state.extractor_id.unwrap_or_else(|| "legacy".into()),
                            state.source_url,
                            item.path,
                            item.size_bytes as i64,
                            state.full_blake3,
                            state.job_id,
                            state.imported_at.unwrap_or(item.added_at),
                        ],
                    )?;
                }
            }
        }
        let ids: Vec<&str> = scanned.iter().map(|item| item.id.as_str()).collect();
        let mut stmt = tx.prepare("SELECT id, path FROM media")?;
        let stale: Vec<String> = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .filter_map(std::result::Result::ok)
            .filter(|(id, path)| !ids.contains(&id.as_str()) || !Path::new(path).exists())
            .map(|(id, _)| id)
            .collect();
        drop(stmt);
        for id in stale {
            tx.execute("DELETE FROM media WHERE id=?1", [id])?;
        }
        tx.commit()?;
        self.list_media()
    }

    pub fn list_media(&self) -> Result<Vec<MediaItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id,m.path,m.title,m.creator,m.description,m.source_url,m.duration_secs,
                    m.added_at,m.modified_at,m.size_bytes,m.video_codec,m.audio_codec,m.container,
                    m.artwork_path,m.playable,m.favorite,m.progress_secs,m.watched,
                    m.conversion_path,
                    EXISTS(SELECT 1 FROM managed_downloads d WHERE d.canonical_path=m.path)
             FROM media m ORDER BY m.added_at DESC, m.title COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], media_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn library_totals(&self) -> Result<(u64, u64)> {
        let (count, bytes): (i64, i64) = self.conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0) FROM media",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok((count.max(0) as u64, bytes.max(0) as u64))
    }

    pub fn get_media(&self, id: &str) -> Result<Option<MediaItem>> {
        self.conn
            .query_row(
                "SELECT m.id,m.path,m.title,m.creator,m.description,m.source_url,m.duration_secs,
                        m.added_at,m.modified_at,m.size_bytes,m.video_codec,m.audio_codec,
                        m.container,m.artwork_path,m.playable,m.favorite,m.progress_secs,m.watched,
                        m.conversion_path,
                        EXISTS(SELECT 1 FROM managed_downloads d WHERE d.canonical_path=m.path)
                 FROM media m WHERE m.id=?1",
                [id],
                media_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn set_favorite(&self, id: &str, favorite: bool) -> Result<bool> {
        Ok(self.conn.execute(
            "UPDATE media SET favorite=?2 WHERE id=?1",
            params![id, favorite as i32],
        )? == 1)
    }

    pub fn set_watched(&self, id: &str, watched: bool) -> Result<bool> {
        Ok(self.conn.execute(
            "UPDATE media SET watched=?2, progress_secs=CASE WHEN ?2=1 THEN 0 ELSE progress_secs END
             WHERE id=?1",
            params![id, watched as i32],
        )? == 1)
    }

    pub fn save_progress(&self, update: &ProgressUpdate) -> Result<()> {
        let watched =
            update.duration_secs > 0.0 && update.position_secs / update.duration_secs >= 0.9;
        let position = if watched {
            0.0
        } else {
            update.position_secs.max(0.0)
        };
        self.conn.execute(
            "UPDATE media SET progress_secs=?2, watched=?3 WHERE id=?1",
            params![update.media_id, position, watched as i32],
        )?;
        Ok(())
    }

    pub fn set_conversion(&self, id: &str, path: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE media SET conversion_path=?2, playable=1 WHERE id=?1",
            params![id, path],
        )?;
        Ok(())
    }

    pub fn register_managed_download(&self, record: &ManagedDownload) -> Result<()> {
        self.conn.execute(
            "INSERT INTO managed_downloads (
                media_id,extractor_id,source_url,canonical_path,size_bytes,blake3,job_id,imported_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(canonical_path) DO UPDATE SET
                media_id=excluded.media_id,extractor_id=excluded.extractor_id,
                source_url=excluded.source_url,size_bytes=excluded.size_bytes,
                blake3=excluded.blake3,job_id=excluded.job_id,imported_at=excluded.imported_at",
            params![
                record.media_id,
                record.extractor_id,
                record.source_url,
                record.canonical_path,
                record.size_bytes as i64,
                record.blake3,
                record.job_id,
                record.imported_at,
            ],
        )?;
        Ok(())
    }

    pub fn managed_download_for_path(&self, path: &str) -> Result<Option<ManagedDownload>> {
        self.conn
            .query_row(
                "SELECT media_id,extractor_id,source_url,canonical_path,size_bytes,blake3,
                        job_id,imported_at
                 FROM managed_downloads WHERE canonical_path=?1",
                [path],
                managed_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn create_removal_plan(&mut self, plan: &RemovalPlanRecord, created_at: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM removal_operation_items WHERE token IN (
                SELECT token FROM removal_operations WHERE expires_at<?1 OR consumed_at IS NOT NULL
             )",
            [created_at],
        )?;
        tx.execute(
            "DELETE FROM removal_operations WHERE expires_at<?1 OR consumed_at IS NOT NULL",
            [created_at],
        )?;
        tx.execute(
            "INSERT INTO removal_operations(
                token,scope,confirmation_phrase,created_at,expires_at,consumed_at
             ) VALUES(?1,?2,?3,?4,?5,NULL)",
            params![
                plan.token,
                plan.scope,
                plan.confirmation_phrase,
                created_at,
                plan.expires_at
            ],
        )?;
        for (ordinal, item) in plan.items.iter().enumerate() {
            tx.execute(
                "INSERT INTO removal_operation_items(
                    token,ordinal,media_id,title,creator,primary_path,source_url,size_bytes,
                    expected_modified_at,artwork_path,conversion_path,managed_by_hometube,
                    full_blake3,extractor_id,provenance_source_url,job_id,imported_at,
                    favorite,progress_secs,watched
                 ) VALUES(
                    ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20
                 )",
                params![
                    plan.token,
                    ordinal as i64,
                    item.media_id,
                    item.title,
                    item.creator,
                    item.primary_path,
                    item.source_url,
                    item.size_bytes as i64,
                    item.expected_modified_at,
                    item.artwork_path,
                    item.conversion_path,
                    item.managed_by_hometube as i32,
                    item.full_blake3,
                    item.extractor_id,
                    item.provenance_source_url,
                    item.job_id,
                    item.imported_at,
                    item.favorite as i32,
                    item.progress_secs,
                    item.watched as i32,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn consume_removal_plan(&mut self, token: &str, now: i64) -> Result<RemovalPlanRecord> {
        let tx = self.conn.transaction()?;
        let operation: Option<(String, String, i64, Option<i64>)> = tx
            .query_row(
                "SELECT scope,confirmation_phrase,expires_at,consumed_at
                 FROM removal_operations WHERE token=?1",
                [token],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let Some((scope, confirmation_phrase, expires_at, consumed_at)) = operation else {
            bail!("This removal preview is no longer available");
        };
        if consumed_at.is_some() {
            bail!("This removal preview was already used");
        }
        if expires_at < now {
            tx.execute(
                "UPDATE removal_operations SET consumed_at=?2 WHERE token=?1",
                params![token, now],
            )?;
            tx.commit()?;
            bail!("This removal preview expired; create a new preview");
        }
        tx.execute(
            "UPDATE removal_operations SET consumed_at=?2 WHERE token=?1 AND consumed_at IS NULL",
            params![token, now],
        )?;
        let mut stmt = tx.prepare(
            "SELECT media_id,title,creator,primary_path,source_url,size_bytes,
                    expected_modified_at,artwork_path,conversion_path,managed_by_hometube,
                    full_blake3,extractor_id,provenance_source_url,job_id,imported_at,
                    favorite,progress_secs,watched
             FROM removal_operation_items WHERE token=?1 ORDER BY ordinal",
        )?;
        let items = stmt
            .query_map([token], removal_plan_item_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        tx.commit()?;
        Ok(RemovalPlanRecord {
            token: token.into(),
            scope,
            confirmation_phrase,
            expires_at,
            items,
        })
    }

    pub fn finalize_removed(
        &mut self,
        item: &RemovalPlanItemRecord,
        removed_at: i64,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO removed_items(
                media_id,title,creator,original_path,source_url,size_bytes,managed_by_hometube,
                full_blake3,extractor_id,job_id,imported_at,favorite,progress_secs,watched,
                artwork_path,conversion_path,removed_at,restored_at
             ) VALUES(
                ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,NULL
             )",
            params![
                item.media_id,
                item.title,
                item.creator,
                item.primary_path,
                item.provenance_source_url
                    .as_deref()
                    .or(item.source_url.as_deref()),
                item.size_bytes as i64,
                item.managed_by_hometube as i32,
                item.full_blake3,
                item.extractor_id,
                item.job_id,
                item.imported_at,
                item.favorite as i32,
                item.progress_secs,
                item.watched as i32,
                item.artwork_path,
                item.conversion_path,
                removed_at,
            ],
        )?;
        tx.execute("DELETE FROM media WHERE id=?1", [&item.media_id])?;
        tx.execute(
            "DELETE FROM managed_downloads WHERE canonical_path=?1",
            [&item.primary_path],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn recently_removed(&self) -> Result<Vec<RemovedItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT history_id,media_id,title,creator,original_path,source_url,size_bytes,
                    managed_by_hometube,removed_at,restored_at
             FROM removed_items ORDER BY removed_at DESC, history_id DESC LIMIT 100",
        )?;
        let rows = stmt.query_map([], |row| {
            let history_id: i64 = row.get(0)?;
            let restored_at: Option<i64> = row.get(9)?;
            Ok(RemovedItem {
                id: format!("removed-{history_id}"),
                media_id: row.get(1)?,
                title: row.get(2)?,
                creator: row.get(3)?,
                original_path: row.get(4)?,
                source_url: row.get(5)?,
                size_bytes: row.get::<_, i64>(6)?.max(0) as u64,
                managed_by_home_tube: row.get::<_, i32>(7)? != 0,
                removed_at: row.get(8)?,
                restored_at,
                status: if restored_at.is_some() {
                    "restored"
                } else {
                    "removed"
                }
                .into(),
                error: None,
                space_recovery_note: "Empty the system Trash to free disk space.".into(),
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn insert_download(&self, job: &DownloadJob) -> Result<()> {
        self.conn.execute(
            "INSERT INTO download_jobs (
                id,url,quality,status,progress,current_title,item_index,item_count,
                completed_count,speed,eta,error,created_at,updated_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                job.id,
                job.url,
                job.quality,
                job.status,
                job.progress,
                job.current_title,
                job.item_index,
                job.item_count,
                job.completed_count,
                job.speed,
                job.eta,
                job.error,
                job.created_at,
                job.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_downloads(&self) -> Result<Vec<DownloadJob>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,url,quality,status,progress,current_title,item_index,item_count,
                    completed_count,speed,eta,error,created_at,updated_at
             FROM download_jobs ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], download_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn get_download(&self, id: &str) -> Result<Option<DownloadJob>> {
        self.conn
            .query_row(
                "SELECT id,url,quality,status,progress,current_title,item_index,item_count,
                        completed_count,speed,eta,error,created_at,updated_at
                 FROM download_jobs WHERE id=?1",
                [id],
                download_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn next_queued_download(&self) -> Result<Option<DownloadJob>> {
        self.conn
            .query_row(
                "SELECT id,url,quality,status,progress,current_title,item_index,item_count,
                        completed_count,speed,eta,error,created_at,updated_at
                 FROM download_jobs WHERE status='queued' ORDER BY created_at LIMIT 1",
                [],
                download_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn update_download(&self, job: &DownloadJob) -> Result<()> {
        self.conn.execute(
            "UPDATE download_jobs SET
                status=?2,progress=?3,current_title=?4,item_index=?5,item_count=?6,
                completed_count=?7,speed=?8,eta=?9,error=?10,updated_at=?11
             WHERE id=?1",
            params![
                job.id,
                job.status,
                job.progress,
                job.current_title,
                job.item_index,
                job.item_count,
                job.completed_count,
                job.speed,
                job.eta,
                job.error,
                job.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn recover_downloads(&self, updated_at: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE download_jobs SET status='queued', error=NULL, updated_at=?1
             WHERE status IN ('extracting','downloading','processing')",
            [updated_at],
        )?;
        Ok(())
    }

    pub fn clear_finished_downloads(&self) -> Result<()> {
        self.conn.execute(
            "DELETE FROM download_jobs WHERE status IN ('complete','failed','cancelled')",
            [],
        )?;
        Ok(())
    }
}

fn run_migrations(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if current > SCHEMA_VERSION {
        bail!(
            "This HomeTube database uses schema version {current}, but this app supports {SCHEMA_VERSION}"
        );
    }
    let migrations = [(1_i64, MIGRATION_1), (2_i64, MIGRATION_2)];
    for (version, sql) in migrations {
        if version <= current {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)
            .with_context(|| format!("Database migration {version} failed"))?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
    }
    Ok(())
}

fn take_valid(
    values: &mut HashMap<String, String>,
    key: &str,
    default: String,
    valid: impl Fn(&str) -> bool,
) -> String {
    values
        .remove(key)
        .filter(|value| valid(value))
        .unwrap_or(default)
}

fn valid_nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_command(value: &str) -> bool {
    valid_nonempty(value) && !value.chars().any(char::is_control)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

pub fn full_file_hash(path: &Path) -> Result<String> {
    let mut file = File::open(path)
        .with_context(|| format!("Could not open {} for verification", path.display()))?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn download_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DownloadJob> {
    Ok(DownloadJob {
        id: row.get(0)?,
        url: row.get(1)?,
        quality: row.get(2)?,
        status: row.get(3)?,
        progress: row.get(4)?,
        current_title: row.get(5)?,
        item_index: row.get(6)?,
        item_count: row.get(7)?,
        completed_count: row.get(8)?,
        speed: row.get(9)?,
        eta: row.get(10)?,
        error: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn media_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MediaItem> {
    Ok(MediaItem {
        id: row.get(0)?,
        path: row.get(1)?,
        title: row.get(2)?,
        creator: row.get(3)?,
        description: row.get(4)?,
        source_url: row.get(5)?,
        duration_secs: row.get(6)?,
        added_at: row.get(7)?,
        modified_at: row.get(8)?,
        size_bytes: row.get::<_, i64>(9)?.max(0) as u64,
        video_codec: row.get(10)?,
        audio_codec: row.get(11)?,
        container: row.get(12)?,
        artwork_path: row.get(13)?,
        playable: row.get::<_, i32>(14)? != 0,
        favorite: row.get::<_, i32>(15)? != 0,
        progress_secs: row.get(16)?,
        watched: row.get::<_, i32>(17)? != 0,
        conversion_path: row.get(18)?,
        managed_by_home_tube: row.get::<_, i32>(19)? != 0,
    })
}

fn managed_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ManagedDownload> {
    Ok(ManagedDownload {
        media_id: row.get(0)?,
        extractor_id: row.get(1)?,
        source_url: row.get(2)?,
        canonical_path: row.get(3)?,
        size_bytes: row.get::<_, i64>(4)?.max(0) as u64,
        blake3: row.get(5)?,
        job_id: row.get(6)?,
        imported_at: row.get(7)?,
    })
}

fn removal_plan_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RemovalPlanItemRecord> {
    Ok(RemovalPlanItemRecord {
        media_id: row.get(0)?,
        title: row.get(1)?,
        creator: row.get(2)?,
        primary_path: row.get(3)?,
        source_url: row.get(4)?,
        size_bytes: row.get::<_, i64>(5)?.max(0) as u64,
        expected_modified_at: row.get(6)?,
        artwork_path: row.get(7)?,
        conversion_path: row.get(8)?,
        managed_by_hometube: row.get::<_, i32>(9)? != 0,
        full_blake3: row.get(10)?,
        extractor_id: row.get(11)?,
        provenance_source_url: row.get(12)?,
        job_id: row.get(13)?,
        imported_at: row.get(14)?,
        favorite: row.get::<_, i32>(15)? != 0,
        progress_secs: row.get(16)?,
        watched: row.get::<_, i32>(17)? != 0,
    })
}

struct RestoredState {
    history_id: i64,
    source_url: Option<String>,
    managed_by_hometube: bool,
    full_blake3: Option<String>,
    extractor_id: Option<String>,
    job_id: Option<String>,
    imported_at: Option<i64>,
    favorite: bool,
    progress_secs: f64,
    watched: bool,
}

fn restored_state(tx: &Transaction<'_>, path: &str) -> Result<Option<RestoredState>> {
    tx.query_row(
        "SELECT history_id,source_url,managed_by_hometube,full_blake3,extractor_id,job_id,
                imported_at,favorite,progress_secs,watched
         FROM removed_items
         WHERE original_path=?1 AND restored_at IS NULL
         ORDER BY removed_at DESC,history_id DESC LIMIT 1",
        [path],
        |row| {
            Ok(RestoredState {
                history_id: row.get(0)?,
                source_url: row.get(1)?,
                managed_by_hometube: row.get::<_, i32>(2)? != 0,
                full_blake3: row.get(3)?,
                extractor_id: row.get(4)?,
                job_id: row.get(5)?,
                imported_at: row.get(6)?,
                favorite: row.get::<_, i32>(7)? != 0,
                progress_secs: row.get(8)?,
                watched: row.get::<_, i32>(9)? != 0,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use tempfile::tempdir;

    fn media(path: &Path) -> MediaItem {
        MediaItem {
            id: "one".into(),
            path: path.to_string_lossy().into(),
            title: "One".into(),
            creator: "Creator".into(),
            description: None,
            source_url: None,
            duration_secs: 100.0,
            added_at: 1,
            modified_at: 1,
            size_bytes: 1,
            video_codec: Some("h264".into()),
            audio_codec: Some("aac".into()),
            container: "mp4".into(),
            artwork_path: None,
            playable: true,
            favorite: false,
            progress_secs: 0.0,
            watched: false,
            conversion_path: None,
            managed_by_home_tube: false,
        }
    }

    #[test]
    fn progress_marks_items_watched_at_ninety_percent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("one.mp4");
        std::fs::write(&path, b"x").unwrap();
        let mut db = Database::open(&dir.path().join("test.sqlite")).unwrap();
        db.replace_scan(&[media(&path)], dir.path()).unwrap();
        db.save_progress(&ProgressUpdate {
            media_id: "one".into(),
            position_secs: 91.0,
            duration_secs: 100.0,
        })
        .unwrap();
        let saved = db.get_media("one").unwrap().unwrap();
        assert!(saved.watched);
        assert_eq!(saved.progress_secs, 0.0);
    }

    #[test]
    fn migrations_preserve_existing_rows_and_advance_user_version() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(MIGRATION_1).unwrap();
        conn.execute(
            "INSERT INTO settings(key,value) VALUES('theme_preference','dark')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO download_jobs(
                id,url,quality,status,progress,item_index,item_count,completed_count,
                created_at,updated_at
             ) VALUES('job','https://youtu.be/a','best','complete',1,1,1,1,1,1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO media(
                id,path,title,creator,duration_secs,added_at,modified_at,size_bytes,container
             ) VALUES('media','/tmp/preserved.mp4','Preserved','Creator',1,1,1,1,'mp4')",
            [],
        )
        .unwrap();
        drop(conn);

        let db = Database::open(&path).unwrap();
        let version: i64 = db
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(db.list_downloads().unwrap().len(), 1);
        assert_eq!(db.list_media().unwrap().len(), 1);
        assert_eq!(
            db.load_settings(config::defaults())
                .unwrap()
                .theme_preference,
            "dark"
        );
    }

    #[test]
    fn settings_round_trip_all_accessibility_preferences() {
        let dir = tempdir().unwrap();
        let mut db = Database::open(&dir.path().join("settings.sqlite")).unwrap();
        let mut settings = config::defaults();
        settings.accent_preference = "violet".into();
        settings.density_preference = "compact".into();
        settings.text_size_preference = "large".into();
        settings.reduced_motion = true;
        db.save_settings(&settings).unwrap();
        let loaded = db.load_settings(config::defaults()).unwrap();
        assert_eq!(loaded.accent_preference, "violet");
        assert_eq!(loaded.density_preference, "compact");
        assert_eq!(loaded.text_size_preference, "large");
        assert!(loaded.reduced_motion);
    }

    #[test]
    fn invalid_persisted_preferences_fall_back_to_safe_defaults() {
        let dir = tempdir().unwrap();
        let db = Database::open(&dir.path().join("invalid-settings.sqlite")).unwrap();
        db.conn
            .execute(
                "INSERT INTO settings(key,value) VALUES('accent_preference','unknown')",
                [],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO settings(key,value) VALUES('reduced_motion','sometimes')",
                [],
            )
            .unwrap();
        let loaded = db.load_settings(config::defaults()).unwrap();
        assert_eq!(loaded.accent_preference, "cinema");
        assert!(!loaded.reduced_motion);
    }

    #[test]
    fn a_scanned_external_restore_recovers_state_and_verified_provenance() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("restored.mp4");
        std::fs::write(&path, b"restored video").unwrap();
        let canonical = path.canonicalize().unwrap();
        let mut db = Database::open(&dir.path().join("restore.sqlite")).unwrap();
        let mut original = media(&canonical);
        original.favorite = true;
        original.progress_secs = 23.0;
        db.replace_scan(&[original.clone()], dir.path()).unwrap();
        let hash = full_file_hash(&canonical).unwrap();
        db.register_managed_download(&ManagedDownload {
            media_id: original.id.clone(),
            extractor_id: "youtube-id".into(),
            source_url: Some("https://youtu.be/youtube-id".into()),
            canonical_path: canonical.to_string_lossy().into_owned(),
            size_bytes: original.size_bytes,
            blake3: hash.clone(),
            job_id: Some("job".into()),
            imported_at: 1,
        })
        .unwrap();
        db.finalize_removed(
            &RemovalPlanItemRecord {
                media_id: original.id.clone(),
                title: original.title.clone(),
                creator: original.creator.clone(),
                primary_path: canonical.to_string_lossy().into_owned(),
                source_url: original.source_url.clone(),
                size_bytes: original.size_bytes,
                expected_modified_at: 0,
                artwork_path: None,
                conversion_path: None,
                managed_by_hometube: true,
                full_blake3: Some(hash),
                extractor_id: Some("youtube-id".into()),
                provenance_source_url: Some("https://youtu.be/youtube-id".into()),
                job_id: Some("job".into()),
                imported_at: Some(1),
                favorite: true,
                progress_secs: 23.0,
                watched: false,
            },
            10,
        )
        .unwrap();
        assert!(db.get_media("one").unwrap().is_none());
        assert_eq!(db.recently_removed().unwrap()[0].status, "removed");

        db.replace_scan(&[media(&canonical)], dir.path()).unwrap();
        let restored = db.get_media("one").unwrap().unwrap();
        assert!(restored.favorite);
        assert_eq!(restored.progress_secs, 23.0);
        assert!(restored.managed_by_home_tube);
        assert_eq!(db.recently_removed().unwrap()[0].status, "restored");
    }

    #[test]
    fn interrupted_downloads_return_to_the_queue() {
        let dir = tempdir().unwrap();
        let db = Database::open(&dir.path().join("downloads.sqlite")).unwrap();
        let mut job = DownloadJob {
            id: "download-one".into(),
            url: "https://youtu.be/example".into(),
            quality: "1080p".into(),
            status: "downloading".into(),
            progress: 0.4,
            current_title: Some("Example".into()),
            item_index: 1,
            item_count: 2,
            completed_count: 0,
            speed: Some("2MiB/s".into()),
            eta: Some("00:10".into()),
            error: Some("old error".into()),
            created_at: 1,
            updated_at: 1,
        };
        db.insert_download(&job).unwrap();
        db.recover_downloads(2).unwrap();
        job = db.get_download(&job.id).unwrap().unwrap();
        assert_eq!(job.status, "queued");
        assert_eq!(job.progress, 0.4);
        assert_eq!(job.updated_at, 2);
        assert!(job.error.is_none());
    }
}
