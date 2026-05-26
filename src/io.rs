//! I/O, data structures, and serialization

use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;

use crate::config::*;
use crate::helpers::*;
use crate::utils::*;

/// Each startpoint entry is 260 bytes: 4 for offset, 10 for name, 246 reserved
const STARTPOINT_ENTRY_SIZE: usize = 260;

/// A marker point in the audio track
#[derive(Debug, Clone)]
pub struct Startpoint {
    pub offset: u32,
    pub label: String,
}

impl Startpoint {
    pub fn new(offset: u32, label: &str) -> Self {
        Self {
            offset,
            label: label.to_string(),
        }
    }

    /// Serialize to 260 bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = vec![0u8; STARTPOINT_ENTRY_SIZE];
        v[0..4].copy_from_slice(&self.offset.to_le_bytes());
        let label_bytes = self.label.as_bytes();
        let len = label_bytes.len().min(10);
        v[4..4 + len].copy_from_slice(&label_bytes[..len]);
        v
    }
}

/// Collection of startpoint markers
#[derive(Debug)]
pub struct StartpointTable {
    pub startpoints: Vec<Startpoint>,
}

impl StartpointTable {
    pub fn new(startpoints: Vec<Startpoint>) -> Self {
        Self { startpoints }
    }

    /// Serialize table with SYNC header
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"SYNC");
        v.extend_from_slice(&(self.startpoints.len() as u32).to_le_bytes());
        for sp in &self.startpoints {
            v.extend_from_slice(&sp.to_bytes());
        }
        v
    }
}

/// Parsed MP3 file data and metadata
#[derive(Debug)]
pub struct Mp3Info {
    pub data: Vec<u8>,
    pub sample_count: u32,
    pub num_channels: u16,
}

impl Mp3Info {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let data = read_file(path.as_ref())?;
        Self::from_bytes(data)
    }

    pub fn from_bytes(data: Vec<u8>) -> Result<Self, String> {
        // Find first MP3 frame
        let sync_offset = find_mp3_sync(&data, 0)
            .ok_or_else(|| "No MP3 frames found".to_string())?;

        let header = read_be_u32(&data, sync_offset);
        let frame_info = parse_mp3_frame_header(header)
            .ok_or_else(|| "Invalid MP3 frame header".to_string())?;

        let num_channels = frame_info.num_channels();
        let sample_count = count_mp3_samples(&data, sync_offset);

        Ok(Self {
            data,
            sample_count,
            num_channels,
        })
    }
}

/// Read entire file into memory
pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, String> {
    let mut file = fs::File::open(path.as_ref())
        .map_err(|e| format!("Failed to open: {}", e))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|e| format!("Failed to read: {}", e))?;
    Ok(data)
}

/// Write data to file
pub fn write_file<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<(), String> {
    fs::write(path.as_ref(), data)
        .map_err(|e| format!("Failed to write: {}", e))
}

/// Count total samples by walking through all MP3 frames
fn count_mp3_samples(data: &[u8], start_offset: usize) -> u32 {
    let mut sample_count: u32 = 0;
    let mut frame_offset = start_offset;

    while frame_offset + 4 <= data.len() {
        let header = read_be_u32(data, frame_offset);

        let Some(frame_info) = parse_mp3_frame_header(header) else {
            frame_offset += 1;
            continue;
        };

        let frame_size = frame_info.frame_size();
        if frame_size == 0 {
            frame_offset += 1;
            continue;
        }

        sample_count += frame_info.samples_per_frame();
        frame_offset += frame_size as usize;
    }

    sample_count
}

/// Builds the complete FSB4 file in memory
pub struct Fsb4Writer;

impl Fsb4Writer {
    pub fn create(
        mp3: &Mp3Info,
        filename: &str,
        bank_uuid: u128,
        startpoints: Option<StartpointTable>,
    ) -> Vec<u8> {
        let raw_data = &mp3.data;
        let raw_len = raw_data.len() as u32;

        let duration = mp3.sample_count as f64 / SAMPLE_RATE as f64;
        let sample_len = calculate_pcm_sample_len(duration, SAMPLE_RATE, mp3.num_channels);

        let startpoint_table = startpoints.map(|sp| sp.to_bytes());
        let startpoint_size = startpoint_table.as_ref().map(|t| t.len()).unwrap_or(0);

        // Layout: header (48) + entry (80) + startpoints + padding + audio
        let startpoint_end = (FSB4_HEADER_SIZE + FSB4_ENTRY_SIZE + startpoint_size) as u32;
        let data_start = align_up(startpoint_end, MPEG_ALIGNMENT);
        let padding_before_audio = data_start - startpoint_end;

        let entry_len = FSB4_ENTRY_SIZE as u32;
        let dir_len = entry_len + startpoint_size as u32 + padding_before_audio;

        let header = Fsb4Header::new(1, dir_len, raw_len, bank_uuid);
        let entry = DirectoryEntry::new(filename, sample_len, raw_len, mp3.num_channels, dir_len, padding_before_audio);

        let total_size = data_start as usize + raw_data.len();
        let mut output = vec![0u8; total_size];

        output[..FSB4_HEADER_SIZE].copy_from_slice(&header.to_bytes());
        output[FSB4_HEADER_SIZE..FSB4_HEADER_SIZE + FSB4_ENTRY_SIZE]
            .copy_from_slice(&entry.to_bytes());

        if let Some(table) = startpoint_table {
            let sp_start = FSB4_HEADER_SIZE + FSB4_ENTRY_SIZE;
            output[sp_start..sp_start + table.len()].copy_from_slice(&table);
        }

        output[data_start as usize..data_start as usize + raw_data.len()]
            .copy_from_slice(raw_data);

        output
    }
}

/// FSB4 file header (48 bytes)
#[derive(Debug)]
pub struct Fsb4Header {
    magic: [u8; 4],
    num_files: u32,
    dir_len: u32,
    dat_len: u32,
    version: u32,
    flags: u32,
    null_bytes: u64,
    bank_uuid: u128,
}

impl Fsb4Header {
    pub fn new(num_files: u32, dir_len: u32, dat_len: u32, bank_uuid: u128) -> Self {
        Self {
            magic: *FSB4_MAGIC,
            num_files,
            dir_len,
            dat_len,
            version: FSB4_VERSION,
            flags: 0,
            null_bytes: 0,
            bank_uuid,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(FSB4_HEADER_SIZE);
        v.extend_from_slice(&self.magic);
        v.extend_from_slice(&self.num_files.to_le_bytes());
        v.extend_from_slice(&self.dir_len.to_le_bytes());
        v.extend_from_slice(&self.dat_len.to_le_bytes());
        v.extend_from_slice(&self.version.to_le_bytes());
        v.extend_from_slice(&self.flags.to_le_bytes());
        v.extend_from_slice(&self.null_bytes.to_le_bytes());
        v.extend_from_slice(&self.bank_uuid.to_le_bytes());
        v
    }
}

/// Directory entry for audio sample (80 bytes)
#[derive(Debug)]
pub struct DirectoryEntry {
    entry_len: u16,
    filename: [u8; 30],
    sample_len: u32,
    compressed_len: u32,
    loop_start: u32,
    loop_end: u32,
    play_mode: u32,
    sample_rate: u32,
    bank_volume: u16,
    pan: u16,
    playback_priority: u16,
    num_channels: u16,
    min_distance: u32,
    max_distance: u32,
    var_freq: u32,
    var_vol: u16,
    var_pan: u16,
}

impl DirectoryEntry {
    pub fn new(
        filename: &str,
        sample_len: u32,
        compressed_len: u32,
        num_channels: u16,
        dir_len: u32,
        padding_before_audio: u32,
    ) -> Self {
        // entry_len covers everything up to the padding before audio data
        let entry_len = (dir_len - padding_before_audio) as u16;

        Self {
            entry_len,
            filename: filename_to_array(filename),
            sample_len,
            compressed_len,
            loop_start: 0,
            loop_end: sample_len.saturating_sub(1),
            play_mode: PLAY_MODE,
            sample_rate: SAMPLE_RATE,
            bank_volume: BANK_VOLUME,
            pan: PAN,
            playback_priority: PLAYBACK_PRIORITY,
            num_channels,
            min_distance: 0,
            max_distance: 0,
            var_freq: 0,
            var_vol: 0,
            var_pan: 0,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(FSB4_ENTRY_SIZE);
        v.extend_from_slice(&self.entry_len.to_le_bytes());
        v.extend_from_slice(&self.filename);
        v.extend_from_slice(&self.sample_len.to_le_bytes());
        v.extend_from_slice(&self.compressed_len.to_le_bytes());
        v.extend_from_slice(&self.loop_start.to_le_bytes());
        v.extend_from_slice(&self.loop_end.to_le_bytes());
        v.extend_from_slice(&self.play_mode.to_le_bytes());
        v.extend_from_slice(&self.sample_rate.to_le_bytes());
        v.extend_from_slice(&self.bank_volume.to_le_bytes());
        v.extend_from_slice(&self.pan.to_le_bytes());
        v.extend_from_slice(&self.playback_priority.to_le_bytes());
        v.extend_from_slice(&self.num_channels.to_le_bytes());
        v.extend_from_slice(&self.min_distance.to_le_bytes());
        v.extend_from_slice(&self.max_distance.to_le_bytes());
        v.extend_from_slice(&self.var_freq.to_le_bytes());
        v.extend_from_slice(&self.var_vol.to_le_bytes());
        v.extend_from_slice(&self.var_pan.to_le_bytes());
        v
    }
}

/// Converts audio files to MP3 using ffmpeg
pub struct FFmpegConverter;

impl FFmpegConverter {
    pub fn convert_to_mp3(input: &str, output: &str) -> Result<(), String> {
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-hide_banner",
                "-loglevel",
                "error",
                "-i",
                input,
                "-map",
                "0:a:0",
                "-vn",
                "-sn",
                "-dn",
                "-map_metadata",
                "-1",
                "-map_chapters",
                "-1",
                "-id3v2_version",
                "0",
                "-write_xing",
                "0",
                "-codec:a",
                "libmp3lame",
                "-ar",
                "48000",
                "-ac",
                "2",
                "-b:a",
                "160k",
                output,
            ])
            .status()
            .map_err(|e| format!("ffmpeg failed: {}", e))?;

        if !status.success() {
            return Err("ffmpeg conversion failed".to_string());
        }
        Ok(())
    }
}
