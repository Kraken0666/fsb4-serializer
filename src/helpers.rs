//! functions for calculations and data parsing

/// Round up to the next multiple of alignment
pub fn align_up(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}

/// Calculate total PCM samples from duration
pub fn calculate_pcm_sample_len(duration: f64, sample_rate: u32, num_channels: u16) -> u32 {
    (duration * sample_rate as f64 * num_channels as f64 * 2.0) as u32
}

/// Parse MP3 frame header bytes into usable fields
pub fn parse_mp3_frame_header(header: u32) -> Option<Mp3FrameInfo> {
    let sync = (header >> 21) & 0x7FF;
    if sync != 0x7FF {
        return None;
    }

    let version = (header >> 19) & 0x03;
    let layer = (header >> 17) & 0x03;
    let bitrate_idx = (header >> 12) & 0x0F;
    let sample_rate_idx = (header >> 10) & 0x03;
    let padding = (header >> 9) & 0x01;
    let channel_bits = (header >> 6) & 0x03;

    // Skip invalid or reserved values
    if version == 1 || layer == 0 || bitrate_idx == 0 || bitrate_idx == 15 || sample_rate_idx == 3 {
        return None;
    }

    Some(Mp3FrameInfo {
        version,
        layer,
        bitrate_idx,
        sample_rate_idx,
        padding,
        channel_bits,
    })
}

/// Parsed MP3 frame header data
#[derive(Debug, Clone, Copy)]
pub struct Mp3FrameInfo {
    pub version: u32,
    pub layer: u32,
    pub bitrate_idx: u32,
    pub sample_rate_idx: u32,
    pub padding: u32,
    pub channel_bits: u32,
}

impl Mp3FrameInfo {
    /// Get number of audio channels (mono or stereo)
    pub fn num_channels(&self) -> u16 {
        if self.channel_bits == 3 { 1 } else { 2 }
    }

    /// Look up sample rate from MPEG version table
    pub fn sample_rate(&self) -> u32 {
        let rates = [
            [11025, 12000, 8000, 0],
            [0, 0, 0, 0],
            [22050, 24000, 16000, 0],
            [44100, 48000, 32000, 0],
        ];
        rates[self.version as usize][self.sample_rate_idx as usize]
    }

    /// Look up bitrate from MPEG version table
    pub fn bitrate(&self) -> u32 {
        let bitrates = [
            [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0],
            [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0],
        ];
        bitrates[self.version as usize][self.bitrate_idx as usize]
    }

    /// Calculate frame size in bytes based on bitrate and sample rate
    pub fn frame_size(&self) -> u32 {
        let sr = self.sample_rate();
        let br = self.bitrate();
        if sr == 0 || br == 0 {
            return 0;
        }

        match self.layer {
            1 => (144 * br * 1000 / sr) + self.padding,
            2 => (144 * br * 1000 / sr) + self.padding,
            3 => ((12 * br * 1000 / sr) + self.padding) * 4,
            _ => 0,
        }
    }

    /// Get number of samples per frame for this MPEG version
    pub fn samples_per_frame(&self) -> u32 {
        match self.layer {
            1 => match self.version {
                3 => 1152,
                2 => 1152,
                _ => 576,
            },
            2 => 1152,
            3 => 384,
            _ => 0,
        }
    }
}

/// Read a 4-byte big-endian value from a byte slice
pub fn read_be_u32(data: &[u8], offset: usize) -> u32 {
    ((data[offset] as u32) << 24)
        | ((data[offset + 1] as u32) << 16)
        | ((data[offset + 2] as u32) << 8)
        | (data[offset + 3] as u32)
}
