# FSB4 Serializer

MP3 to FSB4 converter.

## Build

```bash
cargo build --release
```

Produces a single static binary with zero external dependencies.

## Usage

### Build (audio → FSB4)

```bash
fsb4-serializer --build <input> [...] <output.fsb>
```

- MP3 inputs are interleaved in groups of 3 (6ch tracks).
- Other formats (wav, flac, ogg, opus, aac, m4a, wma) are converted via ffmpeg.
- Directories are expanded to supported audio files, sorted alphabetically.

```bash
# Single stereo track
fsb4-serializer --build song.mp3 output.fsb

# Multiple MP3 pairs → 6ch track
fsb4-serializer --build pair1.mp3 pair2.mp3 pair3.mp3 output.fsb

# Directory of audio files
fsb4-serializer --build ./stems/ output.fsb

# Custom startpoint count (default: 8 for single, 7 for multi)
fsb4-serializer --build --startpoints 12 song.mp3 output.fsb
```

### Extract (FSB4 → MP3)

```bash
fsb4-serializer --extract <input.fsb> [...]
```

- Lossless extraction: raw MP3 frames copied verbatim from FSB4.
- Multichannel tracks are deinterleaved into separate stereo pair files.
- Output defaults to current directory.

```bash
# Extract to current directory
fsb4-serializer --extract music.fsb

# Extract to specific directory
fsb4-serializer --extract --out-dir ./output/ music.fsb
```

### Options

| Flag | Description |
|------|-------------|
| `--build` | Build mode: convert audio to FSB4 |
| `--extract` | Extract mode: pull MP3 tracks from FSB4 |
| `--startpoints N` | Number of startpoints (default: 8 single, 7 multi) |
| `--out-dir <dir>` | Output directory for extract (default: `.`) |
| `--help` | Show help |
| `--version` | Show version |
