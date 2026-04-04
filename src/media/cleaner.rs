use std::fs;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, params};

use crate::config::media_cache_plist_path;
use crate::db;

use super::types::{CleanOutcome, MediaEntry};

/// Recursively removes empty directories under `dir`.
pub fn remove_empty_dirs(dir: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.metadata()?.is_dir() {
            remove_empty_dirs(&path)?;
            let _ = fs::remove_dir(&path);
        }
    }
    Ok(())
}

/// Kills the WhatsApp process and re-launches it so that it re-reads the
/// updated database. A two-second delay is inserted between the two steps.
/// Does nothing if WhatsApp is not currently running.
pub fn restart_whatsapp() {
    // Check if WhatsApp is running before attempting to kill/relaunch it.
    let running = std::process::Command::new("pgrep")
        .args(["-x", "WhatsApp"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !running {
        return;
    }

    let _ = std::process::Command::new("pkill")
        .args(["-x", "WhatsApp"])
        .status();
    std::thread::sleep(Duration::from_secs(2));
    let _ = std::process::Command::new("open")
        .args(["-a", "WhatsApp"])
        .status();
}

/// Deletes every file in `files`, NULLs the corresponding
/// `ZMEDIALOCALPATH` rows in the WhatsApp SQLite database (wrapping all DB
/// writes in a single transaction), repairs orphaned DB records, clears the
/// media disk-cache plist, and restarts WhatsApp when the DB was successfully
/// updated.
///
/// If a WhatsApp database is found but the transaction cannot be committed,
/// the function returns early **without** deleting any files so that the DB
/// and the filesystem stay consistent.
pub fn clean_media(target: &Path, files: &[MediaEntry]) -> CleanOutcome {
    // `message_dir` is the parent of `Media/` — the same base used by
    // `db::relative_db_path` and by WhatsApp's stored paths.
    let message_dir = target.parent().unwrap_or(target);

    // Open the database for writing if it is available.
    let conn: Option<Connection> = match db::get_db_path(target) {
        Some(db_path) if db_path.exists() => match Connection::open(&db_path) {
            Ok(connection) => match connection.execute_batch("BEGIN") {
                Ok(_) => Some(connection),
                Err(_) => None,
            },
            Err(_) => None,
        },
        _ => None,
    };

    let db_present = conn.is_some();
    let mut repaired_orphans = 0usize;

    if let Some(ref connection) = conn {
        // Repair orphaned records (DB rows pointing to files that no longer exist).
        // Paths are stored relative to `message_dir`, consistent with `relative_db_path`.
        if let Ok(mut stmt) = connection.prepare(
            "SELECT Z_PK, ZMEDIALOCALPATH FROM ZWAMEDIAITEM WHERE ZMEDIALOCALPATH IS NOT NULL",
        ) {
            let orphans: Vec<(i64, String)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map(|rows| {
                    rows.filter_map(|r| r.ok())
                        .filter(|(_, rel)| !message_dir.join(rel).exists())
                        .collect()
                })
                .unwrap_or_default();

            repaired_orphans = orphans.len();
            for (pk, _) in &orphans {
                let _ = connection.execute(
                    "UPDATE ZWAMEDIAITEM SET ZMEDIALOCALPATH = NULL WHERE Z_PK = ?1",
                    params![pk],
                );
            }
        }

        // NULL the path for every file we are about to delete.
        for entry in files {
            if let Some(relative) = db::relative_db_path(target, &entry.path) {
                let _ = connection.execute(
                    "UPDATE ZWAMEDIAITEM SET ZMEDIALOCALPATH = NULL WHERE ZMEDIALOCALPATH = ?1",
                    params![relative],
                );
            }
        }
    }

    // Commit the transaction. If the DB was present but the commit fails,
    // bail out **before** touching the filesystem so the two stay in sync.
    let db_updated = conn
        .as_ref()
        .map(|c| c.execute_batch("COMMIT").is_ok())
        .unwrap_or(false);

    if db_present && !db_updated {
        return CleanOutcome {
            deleted_files: 0,
            total_files: files.len(),
            freed_bytes: 0,
            errors: 0,
            repaired_orphans,
            db_updated: false,
        };
    }

    // Delete files from disk.
    let mut freed_bytes = 0u64;
    let mut errors = 0usize;
    for entry in files {
        match fs::remove_file(&entry.path) {
            Ok(_) => freed_bytes += entry.size,
            Err(_) => errors += 1,
        }
    }

    let _ = remove_empty_dirs(target);

    // Remove the disk-cache plist so stale cache entries don't confuse WhatsApp.
    if let Some(path) = media_cache_plist_path() {
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }

    CleanOutcome {
        deleted_files: files.len().saturating_sub(errors),
        total_files: files.len(),
        freed_bytes,
        errors,
        repaired_orphans,
        db_updated,
    }
}
