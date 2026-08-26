use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};

const LEGACY_DIR_NAME: &str = "app.reeldeck.desktop";
const LEGACY_DB_NAME: &str = "reeldeck.sqlite3";
const CURRENT_DB_NAME: &str = "hometube.sqlite3";

pub fn migrate_legacy_data(current_dir: &Path) -> Result<()> {
    let parent = current_dir
        .parent()
        .context("HomeTube data directory has no parent")?;
    let legacy_dir = parent.join(LEGACY_DIR_NAME);
    let mut moved_directory = false;
    if !current_dir.exists() && legacy_dir.is_dir() {
        fs::rename(&legacy_dir, current_dir).with_context(|| {
            format!(
                "Could not migrate {} to {}",
                legacy_dir.display(),
                current_dir.display()
            )
        })?;
        moved_directory = true;
    }
    if !current_dir.exists() {
        return Ok(());
    }

    let legacy_db = current_dir.join(LEGACY_DB_NAME);
    let current_db = current_dir.join(CURRENT_DB_NAME);
    if !moved_directory && !legacy_db.exists() {
        return Ok(());
    }
    if !current_db.exists() && legacy_db.exists() {
        let legacy_connection = Connection::open(&legacy_db)
            .context("Could not open the ReelDeck database for migration")?;
        legacy_connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .context("Could not checkpoint the ReelDeck database before migration")?;
        drop(legacy_connection);
        if let Err(error) = fs::rename(&legacy_db, &current_db) {
            rollback_directory(moved_directory, current_dir, &legacy_dir);
            return Err(error).context("Could not rename the ReelDeck database for HomeTube");
        }
    }
    if !current_db.exists() {
        return Ok(());
    }

    let backup = current_dir.join("reeldeck.sqlite3.migration-backup");
    if !backup.exists() {
        fs::copy(&current_db, &backup).context("Could not create the migration database backup")?;
    }
    let old_prefix = legacy_dir.to_string_lossy().into_owned();
    let new_prefix = current_dir.to_string_lossy().into_owned();
    let result = migrate_database(&current_db, &old_prefix, &new_prefix);
    if let Err(error) = result {
        let _ = fs::copy(&backup, &current_db);
        if moved_directory {
            let _ = fs::rename(&current_db, current_dir.join(LEGACY_DB_NAME));
            rollback_directory(true, current_dir, &legacy_dir);
        }
        return Err(error).context("HomeTube could not finish the ReelDeck data migration");
    }
    Ok(())
}

fn migrate_database(path: &Path, old_prefix: &str, new_prefix: &str) -> Result<()> {
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    let tx = conn.transaction()?;
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )?;
    let already_done: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key='identity_migration' AND value='hometube-v1')",
        [],
        |row| row.get(0),
    )?;
    if !already_done {
        tx.execute(
            "UPDATE media SET artwork_path=REPLACE(artwork_path, ?1, ?2) WHERE artwork_path LIKE ?3",
            params![old_prefix, new_prefix, format!("{old_prefix}%")],
        )?;
        tx.execute(
            "UPDATE media SET conversion_path=REPLACE(conversion_path, ?1, ?2) WHERE conversion_path LIKE ?3",
            params![old_prefix, new_prefix, format!("{old_prefix}%")],
        )?;
        tx.execute(
            "INSERT INTO settings(key, value) VALUES('identity_migration', 'hometube-v1')
             ON CONFLICT(key) DO UPDATE SET value='hometube-v1'",
            [],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn rollback_directory(moved: bool, current: &Path, legacy: &PathBuf) {
    if moved && current.exists() && !legacy.exists() {
        let _ = fs::rename(current, legacy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migrates_legacy_directory_database_and_internal_paths() {
        let root = tempdir().unwrap();
        let legacy = root.path().join(LEGACY_DIR_NAME);
        fs::create_dir_all(&legacy).unwrap();
        let old_db = legacy.join(LEGACY_DB_NAME);
        let connection = Connection::open(&old_db).unwrap();
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL;
                 CREATE TABLE media (artwork_path TEXT, conversion_path TEXT);
                 CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            )
            .unwrap();
        let old_prefix = legacy.to_string_lossy().into_owned();
        connection
            .execute(
                "INSERT INTO media(artwork_path, conversion_path) VALUES(?1, ?2)",
                params![
                    format!("{old_prefix}/artwork/one.jpg"),
                    format!("{old_prefix}/converted/one.mp4")
                ],
            )
            .unwrap();
        drop(connection);

        let current = root.path().join("app.hometube.desktop");
        migrate_legacy_data(&current).unwrap();

        assert!(current.is_dir());
        assert!(!legacy.exists());
        assert!(current.join(CURRENT_DB_NAME).is_file());
        assert!(current.join("reeldeck.sqlite3.migration-backup").is_file());
        let migrated = Connection::open(current.join(CURRENT_DB_NAME)).unwrap();
        let (artwork, conversion): (String, String) = migrated
            .query_row(
                "SELECT artwork_path, conversion_path FROM media",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!(artwork.starts_with(current.to_string_lossy().as_ref()));
        assert!(conversion.starts_with(current.to_string_lossy().as_ref()));
    }

    #[test]
    fn leaves_an_existing_hometube_database_alone() {
        let root = tempdir().unwrap();
        let current = root.path().join("app.hometube.desktop");
        fs::create_dir_all(&current).unwrap();
        Connection::open(current.join(CURRENT_DB_NAME)).unwrap();

        migrate_legacy_data(&current).unwrap();

        assert!(!current.join("reeldeck.sqlite3.migration-backup").exists());
    }
}
