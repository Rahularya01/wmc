use std::path::PathBuf;

/// Classification of a media file by type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCategory {
    Images,
    Videos,
    Audio,
    Documents,
}

impl MediaCategory {
    /// Human-readable label for the category.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Images => "Images",
            Self::Videos => "Videos",
            Self::Audio => "Audio",
            Self::Documents => "Documents",
        }
    }
}

/// A single media file on disk with its resolved size.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaEntry {
    pub path: PathBuf,
    pub size: u64,
}

/// Aggregate stats for one media category (Images / Videos / Audio / Documents).
#[derive(Debug, Clone, PartialEq)]
pub struct CategorySummary {
    pub label: &'static str,
    pub file_count: usize,
    pub total_size: u64,
}

/// Named container for the four category summaries.
#[derive(Debug, Clone, PartialEq)]
pub struct CategorySummaries {
    pub images: CategorySummary,
    pub videos: CategorySummary,
    pub audio: CategorySummary,
    pub documents: CategorySummary,
}

impl CategorySummaries {
    /// Iterate over the four summaries in display order.
    pub fn iter(&self) -> impl Iterator<Item = &CategorySummary> {
        [&self.images, &self.videos, &self.audio, &self.documents].into_iter()
    }
}

/// Per-contact media attribution produced by querying the WhatsApp DB.
#[derive(Debug, Clone, PartialEq)]
pub struct ContactBreakdown {
    pub label: String,
    pub file_count: usize,
    pub total_size: u64,
    pub files: Vec<MediaEntry>,
}

/// Full result of a media directory scan.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanReport {
    pub categories: CategorySummaries,
    pub contact_breakdown: Vec<ContactBreakdown>,
    /// All media files sorted by descending size.
    pub files: Vec<MediaEntry>,
    pub total_files: usize,
    pub total_size: u64,
}

/// Outcome returned by [`crate::media::cleaner::clean_media`].
#[derive(Debug, Clone, PartialEq)]
pub struct CleanOutcome {
    pub deleted_files: usize,
    pub total_files: usize,
    pub freed_bytes: u64,
    pub errors: usize,
    pub repaired_orphans: usize,
    pub db_updated: bool,
    pub db_errors: usize,
}
