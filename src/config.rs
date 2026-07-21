//! FSB4 format constants and flag definitions

pub const FSB4_MAGIC: &[u8; 4] = b"FSB4";
pub const FSB4_VERSION: u32 = 0x00040000;
pub const FSB4_HEADER_SIZE: usize = 48;
pub const FSB4_ENTRY_SIZE: usize = 80;

pub const PLAY_MODE: u32 = 0x80000240;
pub const BANK_VOLUME: u16 = 255;
pub const PLAYBACK_PRIORITY: u16 = 128;
pub const PAN: u16 = 128;

pub const SAMPLE_RATE: u32 = 44100;
pub const MPEG_ALIGNMENT: u32 = 32;

// --- FMOD_FSB_HEADER Flags ---

#[allow(dead_code)]
pub const FMOD_FSB_SOURCE_FORMAT: u32 = 0x00000001;
#[allow(dead_code)]
pub const FMOD_FSB_SOURCE_BASICHEADERS: u32 = 0x00000002;
#[allow(dead_code)]
pub const FMOD_FSB_SOURCE_ENCRYPTED: u32 = 0x00000004;
#[allow(dead_code)]
pub const FMOD_FSB_SOURCE_BIGENDIANPCM: u32 = 0x00000008;
#[allow(dead_code)]
pub const FMOD_FSB_SOURCE_NOTINTERLEAVED: u32 = 0x00000010;
pub const FMOD_FSB_SOURCE_MPEG_PADDED: u32 = 0x00000020;
#[allow(dead_code)]
pub const FMOD_FSB_SOURCE_MPEG_PADDED4: u32 = 0x00000040;

#[allow(dead_code)]
pub fn decode_header_flags(flags: u32) -> Vec<&'static str> {
    let mut names = Vec::new();
    let table: &[(u32, &str)] = &[
        (FMOD_FSB_SOURCE_FORMAT, "FORMAT"),
        (FMOD_FSB_SOURCE_BASICHEADERS, "BASICHEADERS"),
        (FMOD_FSB_SOURCE_ENCRYPTED, "ENCRYPTED"),
        (FMOD_FSB_SOURCE_BIGENDIANPCM, "BIGENDIANPCM"),
        (FMOD_FSB_SOURCE_NOTINTERLEAVED, "NOTINTERLEAVED"),
        (FMOD_FSB_SOURCE_MPEG_PADDED, "MPEG_PADDED"),
        (FMOD_FSB_SOURCE_MPEG_PADDED4, "MPEG_PADDED4"),
    ];
    for &(bit, name) in table {
        if flags & bit != 0 {
            names.push(name);
        }
    }
    names
}

// --- FSOUND_FLAGS ---

pub const FSOUND_LOOP_OFF: u32 = 0x00000001;
pub const FSOUND_LOOP_NORMAL: u32 = 0x00000002;
pub const FSOUND_LOOP_BIDI: u32 = 0x00000004;
pub const FSOUND_8BITS: u32 = 0x00000008;
pub const FSOUND_16BITS: u32 = 0x00000010;
pub const FSOUND_MONO: u32 = 0x00000020;
pub const FSOUND_STEREO: u32 = 0x00000040;
pub const FSOUND_UNSIGNED: u32 = 0x00000080;
pub const FSOUND_SIGNED: u32 = 0x00000100;
pub const FSOUND_MPEG: u32 = 0x00000200;
pub const FSOUND_CHANNELMODE_ALLMONO: u32 = 0x00000400;
pub const FSOUND_CHANNELMODE_ALLSTEREO: u32 = 0x00000800;
pub const FSOUND_HW3D: u32 = 0x00001000;
pub const FSOUND_2D: u32 = 0x00002000;
pub const FSOUND_SYNCPOINTS_NONAMES: u32 = 0x00004000;
pub const FSOUND_DUPLICATE: u32 = 0x00008000;
pub const FSOUND_CHANNELMODE_PROTOOLS: u32 = 0x00010000;
pub const FSOUND_MPEGACCURATE: u32 = 0x00020000;
pub const FSOUND_MPEG_LAYER2: u32 = 0x00040000;
pub const FSOUND_HW2D: u32 = 0x00080000;
pub const FSOUND_3D: u32 = 0x00100000;
pub const FSOUND_32BITS: u32 = 0x00200000;
pub const FSOUND_IMAADPCM: u32 = 0x00400000;
pub const FSOUND_VAG: u32 = 0x00800000;
pub const FSOUND_XMA: u32 = 0x01000000;
pub const FSOUND_GCADPCM: u32 = 0x02000000;
pub const FSOUND_MULTICHANNEL: u32 = 0x04000000;
pub const FSOUND_OGG: u32 = 0x08000000;
pub const FSOUND_MPEG_LAYER3: u32 = 0x10000000;
pub const FSOUND_IMAADPCMSTEREO: u32 = 0x20000000;
pub const FSOUND_IGNORETAGS: u32 = 0x40000000;
pub const FSOUND_SYNCPOINTS: u32 = 0x80000000;

pub fn decode_play_mode(flags: u32) -> Vec<&'static str> {
    let mut names = Vec::new();
    let table: &[(u32, &str)] = &[
        (FSOUND_LOOP_OFF, "LOOP_OFF"),
        (FSOUND_LOOP_NORMAL, "LOOP_NORMAL"),
        (FSOUND_LOOP_BIDI, "LOOP_BIDI"),
        (FSOUND_8BITS, "8BITS"),
        (FSOUND_16BITS, "16BITS"),
        (FSOUND_MONO, "MONO"),
        (FSOUND_STEREO, "STEREO"),
        (FSOUND_UNSIGNED, "UNSIGNED"),
        (FSOUND_SIGNED, "SIGNED"),
        (FSOUND_MPEG, "MPEG"),
        (FSOUND_CHANNELMODE_ALLMONO, "CHANNELMODE_ALLMONO"),
        (FSOUND_CHANNELMODE_ALLSTEREO, "CHANNELMODE_ALLSTEREO"),
        (FSOUND_HW3D, "HW3D"),
        (FSOUND_2D, "2D"),
        (FSOUND_SYNCPOINTS_NONAMES, "SYNCPOINTS_NONAMES"),
        (FSOUND_DUPLICATE, "DUPLICATE"),
        (FSOUND_CHANNELMODE_PROTOOLS, "CHANNELMODE_PROTOOLS"),
        (FSOUND_MPEGACCURATE, "MPEGACCURATE"),
        (FSOUND_MPEG_LAYER2, "MPEG_LAYER2"),
        (FSOUND_HW2D, "HW2D"),
        (FSOUND_3D, "3D"),
        (FSOUND_32BITS, "32BITS"),
        (FSOUND_IMAADPCM, "IMAADPCM"),
        (FSOUND_VAG, "VAG"),
        (FSOUND_XMA, "XMA"),
        (FSOUND_GCADPCM, "GCADPCM"),
        (FSOUND_MULTICHANNEL, "MULTICHANNEL"),
        (FSOUND_OGG, "OGG"),
        (FSOUND_MPEG_LAYER3, "MPEG_LAYER3"),
        (FSOUND_IMAADPCMSTEREO, "IMAADPCMSTEREO"),
        (FSOUND_IGNORETAGS, "IGNORETAGS"),
        (FSOUND_SYNCPOINTS, "SYNCPOINTS"),
    ];
    for &(bit, name) in table {
        if flags & bit != 0 {
            names.push(name);
        }
    }
    names
}
