//! FSB4 format constants

pub const FSB4_MAGIC: &[u8; 4] = b"FSB4";
pub const FSB4_VERSION: u32 = 0x00040000;
pub const FSB4_HEADER_SIZE: usize = 48;
pub const FSB4_ENTRY_SIZE: usize = 80;

pub const PLAY_MODE: u32 = 0x80000240;
pub const BANK_VOLUME: u16 = 255;
pub const PLAYBACK_PRIORITY: u16 = 128;
pub const PAN: u16 = 128;

pub const SAMPLE_RATE: u32 = 48000;
pub const MPEG_ALIGNMENT: u32 = 32;
