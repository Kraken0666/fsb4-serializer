//! conversion pipeline

use crate::io::*;
use crate::utils::*;

const NUM_STARTPOINTS: usize = 8;
const STARTPOINT_LABEL: &str = "startpoint";

pub struct Pipeline;

impl Pipeline {
    pub fn execute(input_path: &str, output_path: &str) -> Result<(), String> {
        let temp_mp3 = "temp_converted.mp3";

        // Convert input to raw MP3 frames
        FFmpegConverter::convert_to_mp3(input_path, temp_mp3)?;

        // Parse audio metadata from headers
        eprintln!("Parsing audio file...");
        let mp3 = Mp3Info::from_file(temp_mp3).map_err(|e| {
            let _ = std::fs::remove_file(temp_mp3);
            e
        })?;

        // Generate evenly spaced startpoint markers
        let startpoints = generate_startpoints(mp3.sample_count);

        // Build FSB4 file in memory
        eprintln!("Creating FSB4...");
        let filename = extract_filename(input_path);
        let bank_uuid = 0u128;
        let fsb4_data = Fsb4Writer::create(&mp3, filename, bank_uuid, Some(startpoints));

        // Write to disk
        eprintln!("Writing {}...", output_path);
        write_file(output_path, &fsb4_data).map_err(|e| {
            let _ = std::fs::remove_file(temp_mp3);
            e
        })?;

        let _ = std::fs::remove_file(temp_mp3);

        eprintln!("Done! Output: {} ({} bytes)", output_path, fsb4_data.len());
        Ok(())
    }
}

/// Create 8 startpoint markers evenly spaced through the track.
/// First marker is offset into the song (not at 0), last is at the end.
fn generate_startpoints(total_samples: u32) -> StartpointTable {
    let mut startpoints = Vec::with_capacity(NUM_STARTPOINTS);

    if total_samples == 0 || NUM_STARTPOINTS == 0 {
        return StartpointTable::new(startpoints);
    }

    let segment_size = total_samples / NUM_STARTPOINTS as u32;

    for i in 0..NUM_STARTPOINTS {
        let offset = if i == NUM_STARTPOINTS - 1 {
            total_samples.saturating_sub(1)
        } else {
            segment_size * (i as u32 + 1)
        };
        startpoints.push(Startpoint::new(offset, STARTPOINT_LABEL));
    }

    StartpointTable::new(startpoints)
}
