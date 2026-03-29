use std::path::PathBuf;

pub const MAX_CONTACTS: usize = 8;
pub const MAX_FILES: usize = 10;

/// File extensions that are WhatsApp internal files and should never be deleted.
pub const IGNORED_EXTENSIONS: &[&str] = &["thumb"];

pub const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "heif", "tiff", "tif",
];

pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "3gp", "m4v", "webm",
];

pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "aac", "m4a", "ogg", "wav", "flac", "opus", "amr"];

/// Default WhatsApp media directory on macOS.
pub fn default_media_path() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME env var not set");
    PathBuf::from(home)
        .join("Library/Group Containers/group.net.whatsapp.WhatsApp.shared/Message/Media")
}

/// Path to the WhatsApp disk-cache plist that must be cleared after cleaning.
pub fn media_cache_plist_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(
        "Library/Containers/net.whatsapp.WhatsApp/Data/tmp/MediaCache/diskcacherepository.plist",
    ))
}
