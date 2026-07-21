//! FSB4 Serializer - MP3 to FSB4 converter and extractor

mod config;
mod core;
mod io;

use std::env;
use std::fs;
use std::path::Path;
use std::process;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn usage(program: &str) {
    eprintln!("Usage:");
    eprintln!("  {program} --build [--startpoints N] <input> [...] <output.fsb>   # build");
    eprintln!("  {program} --extract [--out-dir <dir>] <input.fsb> [...]         # extract");
    eprintln!("  {program} --help                                                # show this help");
    eprintln!();
    eprintln!("Build:   Convert any ffmpeg-supported audio to FSB4.");
    eprintln!("         MP3 inputs are interleaved in groups of 3 (6ch tracks).");
    eprintln!("         Other formats are converted via ffmpeg.");
    eprintln!("Extract: Pull MP3 tracks out of FSB4 files.");
    eprintln!("         Defaults to current dir unless --out-dir is given.");
    eprintln!();
    eprintln!("  --startpoints N   Number of startpoints (1 sample: 8, multi: 4)");
    eprintln!("  --out-dir <dir>   Output directory for extract");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let program = env::args().next().unwrap_or_else(|| "fsb4-serializer".into());

    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        usage(&program);
        process::exit(if args.is_empty() { 1 } else { 0 });
    }

    if args[0] == "--version" {
        eprintln!("{program} {VERSION}");
        process::exit(0);
    }

    let mut num_startpoints: usize = 8;
    let mut explicit_startpoints = false;
    let mut mode_build = false;
    let mut mode_extract = false;
    let mut out_dir: Option<String> = None;
    let mut positional: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--startpoints" {
            i += 1;
            if i >= args.len() {
                eprintln!("Error: --startpoints requires a number");
                process::exit(1);
            }
            num_startpoints = args[i].parse().unwrap_or_else(|_| {
                eprintln!("Error: --startpoints must be a number, got: {}", args[i]);
                process::exit(1);
            });
            explicit_startpoints = true;
        } else if args[i] == "--out-dir" {
            i += 1;
            if i >= args.len() {
                eprintln!("Error: --out-dir requires a path");
                process::exit(1);
            }
            out_dir = Some(args[i].clone());
        } else if args[i] == "--build" {
            mode_build = true;
        } else if args[i] == "--extract" {
            mode_extract = true;
        } else if args[i].starts_with('-') {
            eprintln!("Error: unknown option: {}", args[i]);
            usage(&program);
            process::exit(1);
        } else {
            positional.push(args[i].clone());
        }
        i += 1;
    }

    if !mode_build && !mode_extract {
        eprintln!("Error: --build or --extract required");
        usage(&program);
        process::exit(1);
    }

    let result = if mode_extract {
        if positional.is_empty() {
            eprintln!("Error: --extract needs at least one .fsb file");
            usage(&program);
            process::exit(1);
        }
        let dir = out_dir.unwrap_or_else(|| ".".into());
        let files: Vec<&str> = positional.iter().map(|s| s.as_str()).collect();
        core::execute_extract(&files, &dir)
    } else {
        if positional.len() < 2 {
            eprintln!("Error: need at least one input and an output path");
            usage(&program);
            process::exit(1);
        }
        let output = positional.last().unwrap();
        if Path::new(output).extension().and_then(|e| e.to_str()) != Some("fsb") {
            eprintln!("Warning: output does not have .fsb extension");
        }
        if let Some(parent) = Path::new(output).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).unwrap_or_else(|e| {
                    eprintln!("Error: cannot create output directory: {e}");
                    process::exit(1);
                });
            }
        }
        let mut inputs: Vec<String> = Vec::new();
        for arg in &positional[..positional.len()-1] {
            if Path::new(arg).is_dir() {
                let mut mp3s: Vec<String> = fs::read_dir(arg)
                    .map_err(|e| format!("read dir {arg}: {e}"))
                    .unwrap_or_else(|e| { eprintln!("{e}"); process::exit(1); })
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        matches!(p.extension().and_then(|e| e.to_str()),
                            Some("mp3" | "wav" | "flac" | "ogg" | "opus" | "aac" | "m4a" | "wma"))
                    })
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                mp3s.sort();
                inputs.extend(mp3s);
            } else {
                inputs.push(arg.to_string());
            }
        }
        if inputs.is_empty() {
            eprintln!("Error: no input files found");
            process::exit(1);
        }
        if !explicit_startpoints {
            let total_tracks = (inputs.len() + 2) / 3;
            num_startpoints = if total_tracks == 1 { 8 } else { 7 };
        }
        let input_refs: Vec<&str> = inputs.iter().map(|s| s.as_str()).collect();
        core::execute(&input_refs, output, num_startpoints)
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
