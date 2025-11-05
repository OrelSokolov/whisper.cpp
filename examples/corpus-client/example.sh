#!/bin/bash
# Example usage of corpus-client

# Exit on error
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Whisper Corpus Builder - Example Usage${NC}\n"

# Check if server is running
echo -e "${GREEN}Step 1: Checking if Whisper server is running...${NC}"
if ! nc -z localhost 8765 2>/dev/null; then
    echo "Error: Whisper server is not running on port 8765"
    echo "Please start it first:"
    echo "  cd ../worker-rs"
    echo "  ./target/release/whisper-worker-rs --model ../../models/ggml-large-v3.bin --merge-timestamps"
    exit 1
fi
echo "✓ Server is running"

# Example 1: Process existing audio file
if [ -f "sample_audio.mp3" ]; then
    echo -e "\n${GREEN}Step 2: Processing existing audio file...${NC}"
    cargo run --release -- \
        --audio-file sample_audio.mp3 \
        --output-dir ./dataset_example \
        --min-duration 2.0 \
        --max-duration 10.0 \
        --format mp3
    
    echo -e "\n✓ Dataset created in ./dataset_example"
    echo "  Total segments: $(ls ./dataset_example/segments/*.mp3 2>/dev/null | wc -l)"
    
else
    echo -e "\n${BLUE}Note: sample_audio.mp3 not found, skipping audio processing example${NC}"
fi

# Example 2: YouTube download (commented out, uncomment to use)
# echo -e "\n${GREEN}Step 3: Downloading and processing from YouTube...${NC}"
# cargo run --release -- \
#     --youtube-url "https://www.youtube.com/watch?v=dQw4w9WgXcQ" \
#     --output-dir ./dataset_youtube \
#     --min-duration 3.0 \
#     --max-duration 12.0 \
#     --format wav \
#     --sample-rate 22050

echo -e "\n${GREEN}Done!${NC}"
echo "Check the dataset directory for results."

