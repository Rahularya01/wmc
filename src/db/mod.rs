use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::media::types::{ContactBreakdown, MediaEntry};

// ── Path helpers ─────────────────────────────────────────────────────────────

/// Derives the `ChatStorage.sqlite` path from the media directory.
///
/// WhatsApp lays out its container like:
/// ```text
/// <container>/
///   Message/
///     Media/          ← `media_path`
///     ChatStorage.sqlite
/// ```
pub fn get_db_path(media_path: &Path) -> Option<PathBuf> {
    let message_dir = media_path.parent()?;
    let container_dir = message_dir.parent()?;
    Some(container_dir.join("ChatStorage.sqlite"))
}

/// Returns the relative path string that WhatsApp stores in `ZMEDIALOCALPATH`.
///
/// Paths are stored relative to the `Message/` directory (the parent of the
/// media directory).
pub fn relative_db_path(media_path: &Path, file_path: &Path) -> Option<String> {
    let message_dir = media_path.parent()?;
    file_path
        .strip_prefix(message_dir)
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

// ── Contact lookup ────────────────────────────────────────────────────────────

/// Returns the best display label for a contact: partner name → JID prefix →
/// "Unknown".
fn resolve_contact_label(partner_name: Option<&str>, jid: Option<&str>) -> String {
    if let Some(name) = partner_name {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Some(jid) = jid {
        let stripped = jid.split('@').next().unwrap_or(jid).trim();
        if !stripped.is_empty() {
            return stripped.to_string();
        }
    }
    "Unknown".to_string()
}

/// Queries the WhatsApp database to build a per-contact breakdown for the
/// given set of media files. Returns `None` when the database is unreachable.
pub fn get_contact_breakdown(target: &Path, files: &[MediaEntry]) -> Option<Vec<ContactBreakdown>> {
    let db_path = get_db_path(target)?;
    if !db_path.exists() {
        return None;
    }

    let conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;

    // Build a map from relative DB path → (partner_name, jid).
    let mut stmt = conn
        .prepare(
            "SELECT mi.ZMEDIALOCALPATH, cs.ZPARTNERNAME, cs.ZCONTACTJID \
             FROM ZWAMEDIAITEM mi \
             JOIN ZWAMESSAGE msg ON msg.Z_PK = mi.ZMESSAGE \
             JOIN ZWACHATSESSION cs ON cs.Z_PK = msg.ZCHATSESSION \
             WHERE mi.ZMEDIALOCALPATH IS NOT NULL",
        )
        .ok()?;

    let mut path_to_contact: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .ok()?;

    for row in rows.filter_map(|r| r.ok()) {
        path_to_contact.insert(row.0, (row.1, row.2));
    }

    // Tally files per contact.
    let mut contact_map: HashMap<String, ContactBreakdown> = HashMap::new();
    let mut other_count = 0usize;
    let mut other_size = 0u64;
    let mut other_files: Vec<MediaEntry> = Vec::new();

    for entry in files {
        let label = relative_db_path(target, &entry.path)
            .as_deref()
            .and_then(|rel| path_to_contact.get(rel))
            .map(|(name, jid)| resolve_contact_label(name.as_deref(), jid.as_deref()));

        match label {
            Some(value) => {
                let contact = contact_map
                    .entry(value.clone())
                    .or_insert(ContactBreakdown {
                        label: value,
                        file_count: 0,
                        total_size: 0,
                        files: Vec::new(),
                    });
                contact.file_count += 1;
                contact.total_size += entry.size;
                contact.files.push(entry.clone());
            }
            None => {
                other_count += 1;
                other_size += entry.size;
                other_files.push(entry.clone());
            }
        }
    }

    let mut result: Vec<ContactBreakdown> = contact_map.into_values().collect();
    result.sort_unstable_by(|l, r| r.total_size.cmp(&l.total_size));

    if other_count > 0 {
        result.push(ContactBreakdown {
            label: "Other".to_string(),
            file_count: other_count,
            total_size: other_size,
            files: other_files,
        });
    }

    Some(result)
}
