# Quick Start: Whisper Worker System

## Overview

This guide will help you get started with the enhanced whisper.cpp worker system that supports distributed transcription across multiple machines.

## Prerequisites

- Built whisper.cpp (see main README.md)
- At least one whisper model downloaded
- For distributed setup: orchestrator application (see WORKER_SYSTEM.md)

## Usage Modes

### Mode 1: Standalone Server (No Changes)

Run whisper-server as usual - everything still works the same way:

```bash
./build/bin/whisper-server -m models/ggml-base.en.bin --port 8080
```

**New feature:** Health check endpoint is now available:
```bash
curl http://localhost:8080/health
```

### Mode 2: Worker with Orchestrator

For distributed transcription across multiple machines:

#### Step 1: Build whisper.cpp

```bash
cmake -B build
cmake --build build --config Release --target whisper-server
```

#### Step 2: Download a model

```bash
bash ./models/download-ggml-model.sh base.en
```

#### Step 3: Configure worker

Create `worker.env`:

```bash
# Copy example configuration
cp scripts/worker.env.example worker.env

# Edit with your settings
nano worker.env
```

Minimal configuration:
```bash
export WORKER_ID="my-gpu-worker"
export WORKER_PORT=8080
export ORCHESTRATOR_URL="http://your-orchestrator:4000"
export SHARED_SECRET="your-secure-secret"
export MODEL="models/ggml-base.en.bin"
```

#### Step 4: Start worker

```bash
source worker.env
./scripts/whisper-worker.sh
```

The worker will:
- ✅ Start whisper-server
- ✅ Wait for it to be ready
- ✅ Register with orchestrator
- ✅ Send heartbeats every 20 seconds
- ✅ Handle graceful shutdown (Ctrl+C)

## Testing Without Orchestrator

You can test the new features without setting up an orchestrator:

### Test 1: Health Endpoint

```bash
# Start server
./build/bin/whisper-server -m models/ggml-base.en.bin &

# Wait a moment for startup
sleep 2

# Check health
curl http://localhost:8080/health | jq

# Expected output:
# {
#   "status": "healthy",
#   "model": "models/ggml-base.en.bin",
#   "uptime_seconds": 5,
#   ...
# }
```

### Test 2: Streaming with job_id

```bash
# Start server
./build/bin/whisper-server -m models/ggml-base.en.bin &

# Send transcription request with job_id
curl -X POST http://localhost:8080/inference-stream \
  -F "file=@samples/jfk.wav" \
  -F "job_id=test-123"

# You'll see SSE events with job_id:
# data: {"type":"start","job_id":"test-123",...}
# data: {"type":"segment","job_id":"test-123","text":"..."}
# data: {"type":"done","job_id":"test-123"}
```

## Docker Deployment

### Build Worker Image

```bash
docker build -f Dockerfile.worker -t whisper-worker .
```

### Run Single Worker

```bash
docker run --gpus all \
  -e WORKER_ID=docker-worker-1 \
  -e ORCHESTRATOR_URL=http://orchestrator:4000 \
  -e SHARED_SECRET=your-secret \
  -e MODEL=models/ggml-base.en.bin \
  -v $(pwd)/models:/whisper.cpp/models:ro \
  -p 8080:8080 \
  whisper-worker
```

### Run Multiple Workers with Docker Compose

```bash
# Set environment variables
export ORCHESTRATOR_URL=http://orchestrator:4000
export SHARED_SECRET=your-secret-key

# Start workers
docker-compose -f docker-compose.worker.yml up -d

# View logs
docker-compose -f docker-compose.worker.yml logs -f

# Stop workers
docker-compose -f docker-compose.worker.yml down
```

## Configuration Reference

### Required Variables (Worker Mode)

| Variable | Description | Example |
|----------|-------------|---------|
| `SHARED_SECRET` | Authentication token | `my-secure-token-123` |
| `ORCHESTRATOR_URL` | Orchestrator API URL | `http://192.168.1.100:4000` |

### Optional Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `WORKER_ID` | `worker-$(hostname)-$$` | Unique identifier |
| `WORKER_PORT` | `8080` | Server port |
| `WORKER_HOST` | Auto-detected | Network address |
| `MODEL` | `models/ggml-base.en.bin` | Model path |
| `WHISPER_BIN` | `./build/bin/whisper-server` | Binary path |
| `HEARTBEAT_INTERVAL` | `20` | Heartbeat frequency (seconds) |

## Common Patterns

### Multiple Workers on Same Machine

```bash
# Worker 1
export WORKER_ID=worker-1
export WORKER_PORT=8081
export MODEL=models/ggml-base.en.bin
./scripts/whisper-worker.sh &

# Worker 2
export WORKER_ID=worker-2
export WORKER_PORT=8082
export MODEL=models/ggml-large-v3-turbo.bin
./scripts/whisper-worker.sh &
```

### GPU-Specific Workers

```bash
# Worker on GPU 0 (small model, fast)
export WORKER_ID=gpu-0-fast
export CUDA_VISIBLE_DEVICES=0
export MODEL=models/ggml-base.en.bin
./scripts/whisper-worker.sh &

# Worker on GPU 1 (large model, accurate)
export WORKER_ID=gpu-1-accurate
export CUDA_VISIBLE_DEVICES=1
export MODEL=models/ggml-large-v3-turbo.bin
./scripts/whisper-worker.sh &
```

### Systemd Service

Create `/etc/systemd/system/whisper-worker.service`:

```ini
[Unit]
Description=Whisper Worker
After=network.target

[Service]
Type=simple
User=whisper
WorkingDirectory=/opt/whisper.cpp
EnvironmentFile=/etc/whisper/worker.env
ExecStart=/opt/whisper.cpp/scripts/whisper-worker.sh
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable whisper-worker
sudo systemctl start whisper-worker
sudo systemctl status whisper-worker
```

## Monitoring

### Check Worker Status

```bash
# Health check
curl http://localhost:8080/health

# Full server info
curl -s http://localhost:8080/health | jq
```

### View Worker Logs

```bash
# If running in terminal
# Logs are printed to stdout/stderr

# If running as systemd service
journalctl -u whisper-worker -f

# If running in Docker
docker logs -f whisper-worker-1
```

## Troubleshooting

### Worker fails to register

```bash
# Check if orchestrator is reachable
curl http://your-orchestrator:4000/health

# Verify SHARED_SECRET matches
echo $SHARED_SECRET

# Check network connectivity
ping your-orchestrator
```

### Server fails to start

```bash
# Check if model exists
ls -lh $MODEL

# Check if port is available
lsof -i :8080

# Try running server directly
./build/bin/whisper-server -m $MODEL --port $WORKER_PORT
```

### Heartbeat failures

```bash
# Check orchestrator logs
# Verify network is stable
# Ensure HEARTBEAT_INTERVAL is reasonable (>= 10s)
```

## Next Steps

1. **For standalone use**: You're done! Use the enhanced server as normal.

2. **For distributed system**: 
   - See `WORKER_SYSTEM.md` for full implementation plan
   - Implement Phoenix orchestrator application
   - Deploy multiple workers
   - Scale horizontally as needed

3. **Monitor and optimize**:
   - Set up Prometheus metrics (orchestrator feature)
   - Configure load balancing
   - Tune worker count based on load

## Resources

- **Full Documentation**: `WORKER_README.md`
- **Architecture Plan**: `WORKER_SYSTEM.md`
- **Main README**: `README.md`
- **Examples**: `samples/` directory

## Support

For issues and questions:
- whisper.cpp: https://github.com/ggerganov/whisper.cpp
- Worker system: See WORKER_SYSTEM.md for architecture details

## License

MIT License (same as whisper.cpp)

