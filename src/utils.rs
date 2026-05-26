//! utility functions used across modules

use std::path::Path;

/// Get filename from a path, fallback to default if invalid
pub fn extract_filename(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.mp3")
}

/// Copy filename into a fixed 30-byte array (FSB4 requirement)
pub fn filename_to_array(filename: &str) -> [u8; 30] {
    let mut arr = [0u8; 30];
    let bytes = filename.as_bytes();
    let len = bytes.len().min(30);
    arr[..len].copy_from_slice(&bytes[..len]);
    arr
}

/// Check if two bytes form an MP3 frame sync marker
pub fn is_mp3_sync(data: &[u8], offset: usize) -> bool {
    offset + 2 <= data.len()
        && data[offset] == 0xFF
        && (data[offset + 1] & 0xE0) == 0xE0
}

/// Scan forward to find the next MP3 sync marker
pub fn find_mp3_sync(data: &[u8], start: usize) -> Option<usize> {
    let mut offset = start;
    while offset + 2 <= data.len() {
        if is_mp3_sync(data, offset) {
            return Some(offset);
        }
        offset += 1;
    }
    None
}
