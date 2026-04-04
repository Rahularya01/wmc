use std::fs;
use std::io;
use std::path::Path;

use crate::config::{AUDIO_EXTENSIONS, IGNORED_EXTENSIONS, IMAGE_EXTENSIONS, VIDEO_EXTENSIONS};
use crate::db;

use super::types::{CategorySummary, MediaEntry, ScanReport};

/// Maps a file path to its media category string.
pub fn file_category(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        "Images"
    } else if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        "Videos"
    } else if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
        "Audio"
    } else {
        "Documents"
    }
}

fn is_ignored(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    IGNORED_EXTENSIONS.contains(&ext.as_str())
}

/// Recursively walks `dir` and appends every media file to `files`.
pub fn collect_files(dir: &Path, files: &mut Vec<MediaEntry>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            collect_files(&path, files)?;
        } else if meta.is_file() && !is_ignored(&path) {
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
    let mut documents = CategorySummary {
        label: "Documents",
        file_count: 0,
        total_size: 0,
    };

    for entry in &files {
        match file_category(&entry.path) {
            "Images" => {
                images.file_count += 1;
                images.total_size += entry.size;
            }
            "Videos" => {
                videos.file_count += 1;
                videos.total_size += entry.size;
            }
            "Audio" => {
                audio.file_count += 1;
                audio.total_size += entry.size;
            }
            _ => {
                documents.file_count += 1;
                documents.total_size += entry.size;
            }
        }
    }

    let total_size = images.total_size + videos.total_size + audio.total_size + documents.total_size;
    let total_files = files.len();
    let contact_breakdown = db::get_contact_breakdown(target, &files).unwrap_or_default();

    Ok(ScanReport {
        categories: [images, videos, audio, documents],
        contact_breakdown,
        files,
        total_files,
        total_size,
    })
}
