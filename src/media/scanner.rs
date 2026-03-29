use std::fs;
use std::io;
use std::path::Path;

use crate::config::{AUDIO_EXTENSIONS, IMAGE_EXTENSIONS, VIDEO_EXTENSIONS};
use crate::db;

use super::types::{CategorySummary, MediaEntry, ScanReport};

/// Maps a file path to its media category string, or `None` if not a known
/// media extension.
pub fn file_category(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        Some("Images")
    } else if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        Some("Videos")
    } else if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
        Some("Audio")
    } else {
        None
    }
}

/// Returns `true` when the path has a recognised media extension.
pub fn is_media_file(path: &Path) -> bool {
    file_category(path).is_some()
}

/// Recursively walks `dir` and appends every media file to `files`.
pub fn collect_files(dir: &Path, files: &mut Vec<MediaEntry>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            collect_files(&path, files)?;
        } else if meta.is_file() && is_media_file(&path) {
            files.push(MediaEntry {
                path,
                size: meta.len(),
            });
        }
    }
    Ok(())
}

/// Scans `target` and returns a full [`ScanReport`] including per-contact
/// attribution (when the WhatsApp database is reachable).
pub fn scan_media(target: &Path) -> io::Result<ScanReport> {
    let mut files = Vec::new();
    collect_files(target, &mut files)?;

    // Sort largest-first; ties broken by path for determinism.
    files.sort_unstable_by(|left, right| {
        right
            .size
            .cmp(&left.size)
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut images = CategorySummary {
        label: "Images",
        file_count: 0,
        total_size: 0,
    };
    let mut videos = CategorySummary {
        label: "Videos",
        file_count: 0,
        total_size: 0,
    };
    let mut audio = CategorySummary {
        label: "Audio",
        file_count: 0,
        total_size: 0,
    };

    for entry in &files {
        match file_category(&entry.path) {
            Some("Images") => {
                images.file_count += 1;
                images.total_size += entry.size;
            }
            Some("Videos") => {
                videos.file_count += 1;
                videos.total_size += entry.size;
            }
            Some("Audio") => {
                audio.file_count += 1;
                audio.total_size += entry.size;
            }
            _ => {}
        }
    }

    let total_size = files.iter().map(|e| e.size).sum();
    let total_files = files.len();
    let contact_breakdown = db::get_contact_breakdown(target, &files).unwrap_or_default();

    Ok(ScanReport {
        categories: [images, videos, audio],
        contact_breakdown,
        files,
        total_files,
        total_size,
    })
}
