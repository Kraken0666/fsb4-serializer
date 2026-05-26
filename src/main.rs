//! FSB4 Serializer - MP3 to MPEG_PADDED FSB4
//!
//! Usage: fsb4-serializer <input> <output.fsb>

mod config;
mod core;
mod helpers;
mod io;
mod utils;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <input> <output.fsb>", args[0]);
        process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    if let Err(e) = core::Pipeline::execute(input_path, output_path) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
