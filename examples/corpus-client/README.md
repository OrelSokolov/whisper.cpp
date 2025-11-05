# Whisper Corpus Client

Rust client for building TTS training corpus from audio files.

## Features

- Download audio from YouTube using yt-dlp
- Transcribe audio via Whisper WebSocket server
- Merge timestamps for complete sentences
- **Parallel audio splitting** (uses all CPU cores)
- Split audio into segments
- Generate dataset with metadata
- **Optimized for Piper TTS** (WAV 22050Hz mono)
- **Vowel hotfix** (adds 0.15s to prevent word cutoff)

## Prerequisites

### System Dependencies

```bash
# Ubuntu/Debian
sudo apt-get install ffmpeg libsoxr-dev

# macOS
brew install ffmpeg libsoxr

# Arch Linux
sudo pacman -S ffmpeg libsoxr
```

### Python (for YouTube downloads)

```bash
pip install yt-dlp
```

## Building

```bash
cargo build --release
```

The release build is optimized for maximum performance with parallel processing enabled.

## Usage

### Quick Start for Piper TTS

**Piper TTS mode is enabled by default!**

```bash
./target/release/corpus-client \
    --audio-file audio.mp3 \
    --output-dir ./piper_dataset
```

Default configuration (optimized for Piper TTS):
- Format: WAV (22050Hz, mono, 16-bit PCM)
- Segment duration: 0.5-30 seconds
- Parallel processing (all CPU cores)
- **Vowel hotfix**: Adds 0.15s to segments ending with vowels
- Generates `metadata.csv` in Piper format

To disable Piper optimizations: `--no-piper`

### Full Pipeline (YouTube → Dataset)

```bash
./target/release/corpus-client \
    --youtube-url "https://www.youtube.com/watch?v=..." \
    --output-dir ./dataset \
    --min-duration 2.0 \
    --max-duration 10.0
```

### Transcribe Existing Audio

```bash
./target/release/corpus-client \
    --audio-file audio.mp3 \
    --output-dir ./dataset
```

### Save Timestamps Only

```bash
./target/release/corpus-client \
    --audio-file audio.mp3 \
    --output-timestamps timestamps.json
```

### Split Audio with Existing Timestamps

```bash
./target/release/corpus-client \
    --split-only \
    --audio-file audio.mp3 \
    --timestamps timestamps.json \
    --output-dir ./dataset
```

## Options

- `--host` - WebSocket server host (default: localhost)
- `--port` - WebSocket server port (default: 8765)
- `--youtube-url` - YouTube URL to download and process
- `--audio-file` - Audio file to process
- `--output-dir` - Output directory for dataset (default: ./dataset)
- `--timestamps` - Timestamps JSON file (for split-only mode)
- `--split-only` - Split only mode
- `--format` - Output audio format: mp3, wav, flac (default: **wav**)
- `--min-duration` - Minimum segment duration in seconds (default: 0.5)
- `--max-duration` - Maximum segment duration in seconds (default: 30.0)
- `--sample-rate` - Sample rate for output audio (default: **22050**)
- `--no-piper` - **Disable Piper TTS optimizations** (mono, vowel hotfix)
- `--no-timestamps` - Text only mode (no timestamps)
- `--output-timestamps` - Save timestamps to file
- `--verbose` - Verbose logging

## Examples

### Create TTS dataset for Russian

```bash
# 1. Start Whisper server with merge-timestamps
cd ../worker-rs
./target/release/whisper-worker-rs \
    --model ../../models/ggml-large-v3.bin \
    --merge-timestamps

# 2. Build corpus from YouTube
cd ../corpus-client
./target/release/corpus-client \
    --youtube-url "https://www.youtube.com/watch?v=..." \
    --output-dir ./ru_dataset \
    --format wav \
    --sample-rate 22050 \
    --min-duration 3.0 \
    --max-duration 12.0
```

### Process local audiobook

```bash
./target/release/corpus-client \
    --audio-file audiobook.m4a \
    --output-dir ./audiobook_dataset \
    --min-duration 2.0 \
    --max-duration 15.0
```

## Output Structure

```
dataset/
├── metadata.json          # Dataset metadata (JSON)
├── metadata.csv           # Piper TTS format (filename|text)
├── wavs/
│   ├── 000001.wav        # Audio segment (22050Hz mono)
│   ├── 000001.txt        # Transcription
│   ├── 000002.wav
│   ├── 000002.txt
│   └── ...
└── rejected/              # Filtered out segments
```

## Filtering

Segments are automatically filtered:

1. **Duration**: Outside [min_duration, max_duration] range
2. **Text length**: Less than 5 characters
3. **Music/SFX**: Contains ♪ or [...] markers
4. **Quality**: Failed audio extraction

## See Also

- [CORPUS.md](../../CORPUS.md) - Full documentation
- [worker-rs](../worker-rs/) - Whisper WebSocket server

