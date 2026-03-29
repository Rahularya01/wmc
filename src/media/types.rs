use std::path::PathBuf;

/// A single media file on disk with its resolved size.
#[derive(Clone)]
pub struct MediaEntry {
    pub path: PathBuf,
    pub size: u64,
}

/// Aggregate stats for one media category (Images / Videos / Audio).
#[derive(Clone)]
pub struct CategorySummary {
    pub label: &'static str,
    pub file_count: usize,
    pub total_size: u64,
}

/// Per-contact media attribution produced by querying the WhatsApp DB.
#[derive(Clone)]
pub struct ContactBreakdown {
    pub label: String,
    pub file_count: usize,
    pub total_size: u64,
    pub files: Vec<MediaEntry>,
}

/// Full result of a media directory scan.
#[derive(Clone)]
pub struct ScanReport {
    /// Always four entries: Images, Videos, Audio, Documents (in that order).
    pub categories: [CategorySummary; 4],
    pub contact_breakdown: Vec<ContactBreakdown>,
    /// All media files sorted by descending size.
    pub files: Vec<MediaEntry>,
    pub total_files: usize,
    pub total_size: u64,
}

/// Outcome returned by [`crate::media::cleaner::clean_media`].
pub struct CleanOutcome {
    pub deleted_files: usize,
    pub total_files: usize,
    pub freed_bytes: u64,
    pub errors: usize,
    pub repaired_orphans: usize,
    pub db_updated: bool,
}
