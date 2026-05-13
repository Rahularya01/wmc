use humansize::{BINARY, format_size};

/// Human-readable byte size formatter.
pub fn format_bytes(bytes: u64) -> String {
    format_size(bytes, BINARY)
}
