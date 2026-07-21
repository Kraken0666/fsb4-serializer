//! conversion pipeline

use std::fs;
use std::path::Path;

use crate::config::*;
use crate::io::*;

pub fn execute(input_paths: &[&str], output_path: &str, num_startpoints: usize) -> Result<(), String> {
    let mut entries: Vec<(String, Vec<u8>, Vec<u32>, u32, u16)> = Vec::new();

    let mut pair_data: Vec<Vec<u8>> = Vec::new();
    for p in input_paths {
        if Path::new(p).extension().is_some_and(|e| e == "mp3") {
            pair_data.push(fs::read(p).map_err(|e| format!("read {p}: {e}"))?);
        } else {
            eprintln!("Converting {p}...");
            pair_data.push(convert_to_mp3(p)?);
        }
    }

    let input_names: Vec<String> = input_paths.iter().map(|p| {
        Path::new(p).file_stem().and_then(|s| s.to_str()).unwrap_or("track").to_string()
    }).collect();

    for (i, chunk) in pair_data.chunks(3).enumerate() {
        let ch = chunk.len() * 2;
        eprintln!("Track {i}: {ch}ch, {} MP3 pair{}...", chunk.len(), if chunk.len() == 1 { "" } else { "s" });
        let data = if chunk.len() == 1 {
            pad_mp3_frames(&chunk[0], 2)?
        } else {
            let chunk_refs: Vec<&[u8]> = chunk.iter().map(|v| v.as_slice()).collect();
            interleave_pairs(&chunk_refs)?
        };

        let stem = &input_names[i * 3];
        let filename = format!("{stem}.wav");

        let num_channels = (chunk.len() * 2) as u16;

        let mut min_frames = usize::MAX;
        for d in chunk {
            let f = find_mp3_frame_offsets(d);
            min_frames = min_frames.min(f.len());
        }
        let sample_count = (min_frames * 1152) as u32;

        let startpoints = generate_startpoints(sample_count, i, num_startpoints);
        entries.push((filename, data, startpoints, sample_count, num_channels));
    }

    eprintln!("Creating FSB4...");
    let fsb4_data = create_fsb4(&entries);

    eprintln!("Writing {output_path}...");
    fs::write(output_path, &fsb4_data).map_err(|e| format!("Failed to write: {e}"))?;

    eprintln!("Done! {output_path} ({} bytes, {} tracks)", fsb4_data.len(), entries.len());
    Ok(())
}

pub fn execute_extract(input_paths: &[&str], output_dir: &str) -> Result<(), String> {
    for input_path in input_paths {
        eprintln!("=== {input_path} ===");
        let data = fs::read(input_path).map_err(|e| format!("Failed to read {input_path}: {e}"))?;
        let tracks = parse_fsb4(&data)?;
        eprintln!("  {} track(s)", tracks.len());
        for (i, t) in tracks.iter().enumerate() {
            let flags = decode_play_mode(t.play_mode).join("|");
            eprintln!("  track {i}: {} {}ch {}Hz play=0x{:08x} [{}]",
                t.filename, t.num_channels, t.sample_rate, t.play_mode, flags);
            if !t.startpoints.is_empty() {
                let duration = t.sample_len as f64 / t.sample_rate as f64;
                for (j, &sp) in t.startpoints.iter().enumerate() {
                    let secs = sp as f64 / t.sample_rate as f64;
                    eprintln!("    sp[{j}]: {sp} samples ({:.1}s / {:.1}s)", secs, duration);
                }
            }
        }
        extract_tracks(&data, &tracks, output_dir)?;
        eprintln!("  -> {output_dir}");
    }
    Ok(())
}

fn generate_startpoints(total_samples: u32, track_index: usize, num_startpoints: usize) -> Vec<u32> {
    if total_samples == 0 || num_startpoints == 0 || track_index != 0 {
        return Vec::new();
    }
    let segment_size = total_samples / (num_startpoints as u32 + 1);
    (1..=num_startpoints as u32)
        .map(|i| {
            let raw = segment_size * i;
            let snapped = (raw + 576) / 1152 * 1152;
            if snapped >= total_samples { total_samples.saturating_sub(1) } else { snapped }
        })
        .collect()
}

fn create_fsb4(entries: &[(String, Vec<u8>, Vec<u32>, u32, u16)]) -> Vec<u8> {
    let num_files = entries.len() as u32;

    let mut dir_blocks: Vec<Vec<u8>> = Vec::new();
    let mut dat_len: u32 = 0;

    for (filename, data, startpoints, sample_count, num_channels) in entries.iter() {
        let compressed_len = data.len() as u32;
        dat_len += compressed_len;

        let sample_len = *sample_count;

        let has_sp = !startpoints.is_empty();
        let play_mode = if *num_channels > 2 {
            if has_sp { 0x84000200 } else { 0x04000200 }
        } else {
            if has_sp { PLAY_MODE } else { PLAY_MODE & !0x80000000 }
        };

        let sp_table = if !startpoints.is_empty() { Some(serialize_startpoints(startpoints)) } else { None };
        let sp_size = sp_table.as_ref().map(|t| t.len()).unwrap_or(0);
        let entry_len = (FSB4_ENTRY_SIZE + sp_size) as u16;

        let mut block = vec![0u8; FSB4_ENTRY_SIZE + sp_size];
        let mut o = 0;
        block[o..][..2].copy_from_slice(&entry_len.to_le_bytes()); o += 2;
        let fname = filename.as_bytes();
        let fname_len = fname.len().min(30);
        block[o..][..fname_len].copy_from_slice(&fname[..fname_len]); o += 30;
        block[o..][..4].copy_from_slice(&sample_len.to_le_bytes()); o += 4;
        block[o..][..4].copy_from_slice(&compressed_len.to_le_bytes()); o += 4;
        o += 4; // loop_start: 0
        block[o..][..4].copy_from_slice(&sample_len.saturating_sub(1).to_le_bytes()); o += 4;
        block[o..][..4].copy_from_slice(&play_mode.to_le_bytes()); o += 4;
        block[o..][..4].copy_from_slice(&SAMPLE_RATE.to_le_bytes()); o += 4;
        block[o..][..2].copy_from_slice(&BANK_VOLUME.to_le_bytes()); o += 2;
        block[o..][..2].copy_from_slice(&PAN.to_le_bytes()); o += 2;
        block[o..][..2].copy_from_slice(&PLAYBACK_PRIORITY.to_le_bytes()); o += 2;
        block[o..][..2].copy_from_slice(&num_channels.to_le_bytes()); o += 2;
        o += 16;

        if let Some(table) = sp_table {
            block[o..][..table.len()].copy_from_slice(&table);
        }

        dir_blocks.push(block);
    }

    let header_flags = 0x20u32;

    let raw_dir_len: usize = dir_blocks.iter().map(|b| b.len()).sum();
    let total_before_data = FSB4_HEADER_SIZE + raw_dir_len;
    let data_start = (total_before_data + MPEG_ALIGNMENT as usize - 1) & !(MPEG_ALIGNMENT as usize - 1);
    let dir_len = (data_start - FSB4_HEADER_SIZE) as u32;

    let total_size = data_start + dat_len as usize;
    let mut output = vec![0u8; total_size];

    let mut o = 0;
    output[o..][..4].copy_from_slice(FSB4_MAGIC); o += 4;
    output[o..][..4].copy_from_slice(&num_files.to_le_bytes()); o += 4;
    output[o..][..4].copy_from_slice(&dir_len.to_le_bytes()); o += 4;
    output[o..][..4].copy_from_slice(&dat_len.to_le_bytes()); o += 4;
    output[o..][..4].copy_from_slice(&FSB4_VERSION.to_le_bytes()); o += 4;
    output[o..][..4].copy_from_slice(&header_flags.to_le_bytes()); o += 4;
    o += 8;
    output[o..][..16].copy_from_slice(&0u128.to_le_bytes());

    o = FSB4_HEADER_SIZE;
    for block in &dir_blocks {
        output[o..][..block.len()].copy_from_slice(block);
        o += block.len();
    }

    o = data_start;
    for (_, data, _, _, _) in entries {
        output[o..][..data.len()].copy_from_slice(data);
        o += data.len();
    }

    output
}
