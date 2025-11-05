#!/bin/bash
# Build script for corpus-client

set -e

echo "Building corpus-client..."

# Check if ffmpeg is installed
if ! command -v ffmpeg &> /dev/null; then
    echo "Error: ffmpeg is not installed"
    echo "Please install it:"
    echo "  Ubuntu/Debian: sudo apt-get install ffmpeg"
    echo "  macOS: brew install ffmpeg"
    exit 1
fi

# Check if ffprobe is installed
if ! command -v ffprobe &> /dev/null; then
    echo "Error: ffprobe is not installed (should come with ffmpeg)"
    exit 1
fi

# Build
cargo build --release

echo "✓ Build complete!"
echo ""
echo "Binary location: ./target/release/corpus-client"
echo ""
echo "Example usage:"
echo "  ./target/release/corpus-client --audio-file audio.mp3 --output-dir ./dataset"
echo ""
echo "For full pipeline with YouTube:"
echo "  1. Install yt-dlp: pip install yt-dlp"
echo "  2. Start whisper server:"
echo "     cd ../worker-rs && ./target/release/whisper-worker-rs --merge-timestamps"
echo "  3. Run corpus-client:"
echo "     ./target/release/corpus-client --youtube-url 'https://www.youtube.com/...'"

