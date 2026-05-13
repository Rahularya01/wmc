use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::Path;

use crate::config::{AUDIO_EXTENSIONS, IGNORED_EXTENSIONS, IMAGE_EXTENSIONS, VIDEO_EXTENSIONS};
use crate::db;

use super::types::{CategorySummaries, CategorySummary, MediaCategory, MediaEntry, ScanReport};

/// Extracts the lowercase extension from a file path, if any.
fn extension_lowercase(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

/// Maps a file path to its media category.
pub fn file_category(path: &Path) -> MediaCategory {
    let ext = extension_lowercase(path).unwrap_or_default();
    if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        MediaCategory::Images
    } else if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        MediaCategory::Videos
    } else if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
        MediaCategory::Audio
    } else {
        MediaCategory::Documents
    }
}

fn is_ignored(path: &Path) -> bool {
    let ext = extension_lowercase(path).unwrap_or_default();
    IGNORED_EXTENSIONS.contains(&ext.as_str())
}

/// Iteratively walks `dir` and returns every media file.
///
/// Symbolic links are skipped to prevent directory traversal outside the
/// target and infinite recursion from symlink cycles.
pub fn collect_files(dir: &Path) -> io::Result<Vec<MediaEntry>> {
    let mut files = Vec::new();
    let mut dirs = VecDeque::new();
    dirs.push_back(dir.to_path_buf());

    while let Some(current) = dirs.pop_front() {
        for entry in fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            let meta = fs::symlink_metadata(&path)?;

            if meta.is_symlink() {
                continue;
            }

            if meta.is_dir() {
                dirs.push_back(path);
            } else if meta.is_file() && !is_ignored(&path) {
                files.push(MediaEntry {
                    path,
                    size: meta.len(),
                });
            }
        }
    }
    Ok(files)
}

/// Scans `target` and returns a full [`ScanReport`] including per-contact
/// attribution (when the WhatsApp database is reachable).
pub fn scan_media(target: &Path) -> io::Result<ScanReport> {
    let mut files = collect_files(target)?;

    // Sort largest-first; ties broken by path for determinism.
    files.sort_unstable_by(|left, right| {
        right
            .size
            .cmp(&left.size)
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut categories = CategorySummaries {
        images: CategorySummary {
            label: MediaCategory::Images.label(),
            file_count: 0,
            total_size: 0,
        },
        videos: CategorySummary {
            label: MediaCategory::Videos.label(),
            file_count: 0,
            total_size: 0,
        },
        audio: CategorySummary {
            label: MediaCategory::Audio.label(),
            file_count: 0,
            total_size: 0,
        },
        documents: CategorySummary {
            label: MediaCategory::Documents.label(),
            file_count: 0,
            total_size: 0,
        },
    };

    for entry in &files {
        match file_category(&entry.path) {
            MediaCategory::Images => {
                categories.images.file_count += 1;
                categories.images.total_size += entry.size;
            }
            MediaCategory::Videos => {
                categories.videos.file_count += 1;
                categories.videos.total_size += entry.size;
            }
            MediaCategory::Audio => {
                categories.audio.file_count += 1;
                categories.audio.total_size += entry.size;
            }
            MediaCategory::Documents => {
                categories.documents.file_count += 1;
                categories.documents.total_size += entry.size;
            }
        }
    }

    let total_size = categories.images.total_size
        + categories.videos.total_size
        + categories.audio.total_size
        + categories.documents.total_size;
    let total_files = files.len();
    let contact_breakdown = db::get_contact_breakdown(target, &files).unwrap_or_default();

    Ok(ScanReport {
        categories,
        contact_breakdown,
        files,
        total_files,
        total_size,
    })
}
