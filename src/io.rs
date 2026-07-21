//! I/O, data structures, and serialization

use std::fs;
use std::path::Path;
use std::process::Command;


// --- startpoints ---

const STARTPOINT_ENTRY_SIZE: usize = 260;

pub fn serialize_startpoints(offsets: &[u32]) -> Vec<u8> {
    let mut v = Vec::with_capacity(8 + offsets.len() * STARTPOINT_ENTRY_SIZE);
    v.extend_from_slice(b"SYNC");
    v.extend_from_slice(&(offsets.len() as u32).to_le_bytes());
    for &offset in offsets {
        let mut entry = [0u8; STARTPOINT_ENTRY_SIZE];
        entry[0..4].copy_from_slice(&offset.to_le_bytes());
        entry[4..14].copy_from_slice(b"startpoint");
        v.extend_from_slice(&entry);
    }
    v
}

// --- fsb4 parsing ---

#[allow(dead_code)]
pub struct Fsb4Track {
    pub filename: String,
    pub sample_len: u32,
    pub compressed_len: u32,
    pub data_offset: usize,
    pub play_mode: u32,
    pub sample_rate: u32,
    pub num_channels: u16,
    pub startpoints: Vec<u32>,
}

pub fn parse_fsb4(data: &[u8]) -> Result<Vec<Fsb4Track>, String> {
    if data.len() < 48 {
        return Err("file too small for header".into());
    }
    if &data[0..4] != b"FSB4" {
        return Err("not FSB4".into());
    }

    let num_files = u32::from_le_bytes(data[4..8].try_into().unwrap());
    let dir_len = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    let dat_len = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    let version = u32::from_le_bytes(data[16..20].try_into().unwrap());

    if version != 0x00040000 {
        return Err(format!("unsupported version 0x{:08x}", version));
    }

    let dir_start = 48;
    let data_start = dir_start + dir_len;

    if data_start + dat_len > data.len() {
        return Err(format!("truncated: need {} bytes, have {}", data_start + dat_len, data.len()));
    }

    let mut tracks = Vec::new();
    let mut offset = dir_start;
    let mut data_pos = data_start;

    for i in 0..num_files {
        if offset + 80 > data.len() {
            return Err(format!("truncated at entry {i}"));
        }

        let entry_len = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;

        let filename_raw = &data[offset + 2..offset + 32];
        let filename = filename_raw
            .iter()
            .position(|&b| b == 0)
            .map(|p| String::from_utf8_lossy(&filename_raw[..p]).into_owned())
            .unwrap_or_else(|| String::from_utf8_lossy(filename_raw).into_owned());

        let sample_len = u32::from_le_bytes(data[offset + 32..offset + 36].try_into().unwrap());
        let compressed_len = u32::from_le_bytes(data[offset + 36..offset + 40].try_into().unwrap());
        let play_mode = u32::from_le_bytes(data[offset + 48..offset + 52].try_into().unwrap());
        let sample_rate = u32::from_le_bytes(data[offset + 52..offset + 56].try_into().unwrap());
        let num_channels = u16::from_le_bytes(data[offset + 62..offset + 64].try_into().unwrap());

        let mut startpoints = Vec::new();
        if entry_len > 80 {
            let sp_offset = offset + 80;
            if &data[sp_offset..sp_offset + 4] == b"SYNC" {
                let sp_count = u32::from_le_bytes(data[sp_offset + 4..sp_offset + 8].try_into().unwrap());
                for j in 0..sp_count {
                    let entry_sp = sp_offset + 8 + j as usize * 260;
                    startpoints.push(u32::from_le_bytes(data[entry_sp..entry_sp + 4].try_into().unwrap()));
                }
            }
        }

        tracks.push(Fsb4Track { filename, sample_len, compressed_len, data_offset: data_pos, play_mode, sample_rate, num_channels, startpoints });
        data_pos += compressed_len as usize;
        offset += entry_len;
    }

    Ok(tracks)
}

// --- mp3 extraction ---

pub fn extract_tracks(data: &[u8], tracks: &[Fsb4Track], out_dir: &str) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("Failed to create {out_dir}: {e}"))?;
    let multi = tracks.len() > 1;
    for (i, t) in tracks.iter().enumerate() {
        let mp3_data = &data[t.data_offset..t.data_offset + t.compressed_len as usize];
        let stem = Path::new(&t.filename).file_stem().and_then(|s| s.to_str()).unwrap_or("track");
        let stem = if multi { format!("{i}_{stem}") } else { stem.to_string() };
        extract_track(mp3_data, t.num_channels, &stem, out_dir)?;
    }
    Ok(())
}

pub fn extract_track(mp3_data: &[u8], num_channels: u16, stem: &str, out_dir: &str) -> Result<(), String> {
    let frames = find_mp3_frame_offsets(mp3_data);
    let pairs = (num_channels / 2) as usize;

    if frames.is_empty() {
        return Err("no MP3 frames found".into());
    }

    let mut output = Vec::new();
    for &(off, size) in &frames {
        output.extend_from_slice(&mp3_data[off..off + size]);
    }

    if pairs <= 1 {
        let out_path = Path::new(out_dir).join(format!("{stem}.mp3"));
        fs::write(&out_path, &output).map_err(|e| format!("write failed: {e}"))?;
        return Ok(());
    }

    let mut pair_bufs: Vec<Vec<u8>> = vec![Vec::new(); pairs];
    for (fi, &(off, size)) in frames.iter().enumerate() {
        pair_bufs[fi % pairs].extend_from_slice(&mp3_data[off..off + size]);
    }

    for pi in 0..pairs {
        let out_path = Path::new(out_dir).join(format!("{stem}_{}.mp3", pi + 1));
        fs::write(&out_path, &pair_bufs[pi]).map_err(|e| format!("write failed: {e}"))?;
    }
    Ok(())
}

pub fn find_mp3_frame_offsets(data: &[u8]) -> Vec<(usize, usize)> {
    let mut frames = Vec::new();
    let mut off = 0;
    while off + 4 <= data.len() {
        if data[off] == 0xFF && (data[off + 1] & 0xE0) == 0xE0 {
            let header = u32::from_be_bytes(data[off..off + 4].try_into().unwrap());
            let version = (header >> 19) & 3;
            let layer = (header >> 17) & 3;
            let br_idx = (header >> 12) & 0xF;
            let sr_idx = (header >> 10) & 3;
            let padding = (header >> 9) & 1;

            let sr = [[11025u32, 12000, 8000, 0], [0, 0, 0, 0], [22050, 24000, 16000, 0], [44100, 48000, 32000, 0]][version as usize][sr_idx as usize];
            let br = [0u32, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0][br_idx as usize];

            if sr > 0 && br > 0 && version == 3 && layer == 1 {
                let frame_size = (144 * br * 1000 / sr + padding) as usize;
                if off + frame_size > data.len() { break; }
                frames.push((off, frame_size));
                off += frame_size;
                continue;
            }
        }
        off += 1;
    }
    frames
}

// --- mp3 interleave ---

pub fn pad_mp3_frames(data: &[u8], alignment: usize) -> Result<Vec<u8>, String> {
    let frames = find_mp3_frame_offsets(data);
    if frames.is_empty() {
        return Err("no MP3 frames found".into());
    }
    let mut output = Vec::with_capacity(data.len() + frames.len() * alignment);
    for &(off, size) in &frames {
        output.extend_from_slice(&data[off..off + size]);
        let pad = (alignment - (size % alignment)) % alignment;
        output.resize(output.len() + pad, 0);
    }
    Ok(output)
}

pub fn interleave_pairs(pair_data: &[&[u8]]) -> Result<Vec<u8>, String> {
    if pair_data.is_empty() {
        return Err("no pair data".into());
    }

    let pairs = pair_data.len();
    let mut pair_frames: Vec<Vec<(usize, usize)>> = Vec::new();
    for d in pair_data {
        pair_frames.push(find_mp3_frame_offsets(d));
    }

    let frame_count = pair_frames.iter().map(|f| f.len()).min().ok_or("no frames")?;

    let mut output = Vec::new();
    for fi in 0..frame_count {
        for pi in 0..pairs {
            let (off, size) = pair_frames[pi][fi];
            output.extend_from_slice(&pair_data[pi][off..off + size]);
            let pad = (16 - (size % 16)) % 16;
            output.resize(output.len() + pad, 0);
        }
    }

    Ok(output)
}

// --- ffmpeg ---

fn which_ffmpeg() -> Result<String, String> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();

    let try_names: &[&str] = if cfg!(windows) {
        &["ffmpeg.exe", "ffmpeg"]
    } else {
        &["ffmpeg"]
    };

    // check next to the binary, then bin/ subfolder, then PATH
    for name in try_names {
        let path = exe_dir.join(name);
        if path.exists() {
            return Ok(path.to_string_lossy().into_owned());
        }
        let path = exe_dir.join("bin").join(name);
        if path.exists() {
            return Ok(path.to_string_lossy().into_owned());
        }
    }

    // fall back to PATH
    for name in try_names {
        if Command::new(if cfg!(windows) { "where" } else { "which" })
            .arg(name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Ok(name.to_string());
        }
    }
    Err("ffmpeg not found — place ffmpeg next to the binary or add it to PATH".to_string())
}

pub fn convert_to_mp3(input: &str) -> Result<Vec<u8>, String> {
    let ffmpeg = which_ffmpeg()?;
    let output = Command::new(ffmpeg)
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
            "-codec:a",
            "libmp3lame",
            "-ar",
            "44100",
            "-ac",
            "2",
            "-b:a",
            "160k",
            "-write_xing",
            "0",
            "-f",
            "mp3",
            "pipe:1",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("ffmpeg failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg: {stderr}"));
    }

    let data = output.stdout;
    Ok(data)
}
