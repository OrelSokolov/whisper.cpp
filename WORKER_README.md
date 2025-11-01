# Whisper Worker System

## Overview

This implementation adds worker orchestration capabilities to whisper.cpp, enabling horizontal scaling across multiple machines. The system consists of:

1. **Enhanced whisper-server** with `/health` endpoint
2. **Worker wrapper script** for automatic registration and heartbeat
3. **Ready for orchestrator integration** (Phoenix/Elixir backend - separate project)

## What's New in whisper.cpp

### 1. Health Check Endpoint

The whisper-server now includes a `/health` endpoint that provides:

- Server status (healthy/loading)
- Model information
- Uptime
- Capabilities (multilingual, GPU, streaming support, etc.)

**Example:**
```bash
curl http://localhost:8080/health
```

**Response:**
```json
{
  "status": "healthy",
  "model": "models/ggml-large-v3-turbo.bin",
  "model_name": "ggml-large-v3-turbo.bin",
  "uptime_seconds": 3600,
  "version": "1.0.0",
  "capabilities": {
    "multilingual": true,
    "gpu_enabled": true,
    "flash_attn": true,
    "supports_streaming": true,
    "ffmpeg_converter": false
  }
}
```

### 2. Enhanced SSE Streaming with job_id

The `/inference-stream` endpoint now supports `job_id` parameter for tracking:

```bash
curl -X POST http://localhost:8080/inference-stream \
  -F "file=@audio.wav" \
  -F "job_id=unique-job-123"
```

All SSE events will include the `job_id`:
```json
{"type": "segment", "job_id": "unique-job-123", "text": "Hello world", ...}
```

## Worker Wrapper Script

### Purpose

The `whisper-worker.sh` script automatically:
- Starts the whisper-server
- Registers with the orchestrator
- Sends periodic heartbeats
- Handles graceful shutdown

### Configuration

Create a `.env` file or export environment variables:

```bash
# Worker Configuration
export WORKER_ID="gpu-worker-1"
export WORKER_PORT=8081
export WORKER_HOST="192.168.1.100"

# Orchestrator
export ORCHESTRATOR_URL="http://orchestrator.local:4000"
export SHARED_SECRET="your-secure-secret-key"

# Whisper Configuration
export MODEL="models/ggml-large-v3-turbo.bin"
export WHISPER_BIN="./build/bin/whisper-server"
export HEARTBEAT_INTERVAL=20
```

See `scripts/worker.env.example` for a complete example.

### Usage

#### Standalone Mode (without orchestrator)
```bash
# Just run the server normally
./build/bin/whisper-server -m models/ggml-base.en.bin --port 8080
```

#### Worker Mode (with orchestrator)
```bash
# Source your configuration
source worker.env

# Run the worker wrapper
./scripts/whisper-worker.sh
```

The script will:
1. Start whisper-server
2. Wait for it to be ready (checks `/health`)
3. Register with orchestrator
4. Send heartbeats every 20 seconds
5. Gracefully shutdown on SIGINT/SIGTERM

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `WORKER_ID` | `worker-$(hostname)-$$` | Unique worker identifier |
| `WORKER_PORT` | `8080` | Port for whisper-server |
| `WORKER_HOST` | Auto-detected IP | Worker's network address |
| `ORCHESTRATOR_URL` | `http://localhost:4000` | Orchestrator API URL |
| `SHARED_SECRET` | Required | Authentication token |
| `MODEL` | `models/ggml-base.en.bin` | Path to whisper model |
| `WHISPER_BIN` | `./build/bin/whisper-server` | Path to whisper-server binary |
| `HEARTBEAT_INTERVAL` | `20` | Heartbeat frequency (seconds) |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Phoenix Application (Elixir)                │
│                   (Separate Project)                      │
│                                                           │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Public API (Phoenix Router)                       │  │
│  │  POST /api/transcribe        - upload audio        │  │
│  │  GET  /api/transcribe/:id    - get result         │  │
│  │  GET  /api/transcribe/:id/stream - SSE stream     │  │
│  └────────────────────────────────────────────────────┘  │
│                          ↓                                │
│  ┌────────────────────────────────────────────────────┐  │
│  │  WorkerRegistry (GenServer)                        │  │
│  │  - Track workers, health checks, load balancing    │  │
│  └────────────────────────────────────────────────────┘  │
│                          ↓                                │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Internal API (для воркеров)                       │  │
│  │  POST   /internal/workers/register                 │  │
│  │  POST   /internal/workers/:id/heartbeat            │  │
│  │  DELETE /internal/workers/:id                      │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          ↓ HTTP
        ┌─────────────────┼─────────────────┐
        ↓                 ↓                 ↓
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│ Whisper Worker│  │ Whisper Worker│  │ Whisper Worker│
│               │  │               │  │               │
│wrapper.sh     │  │wrapper.sh     │  │wrapper.sh     │
│     ↓         │  │     ↓         │  │     ↓         │
│whisper-server │  │whisper-server │  │whisper-server │
│ (C++)         │  │ (C++)         │  │ (C++)         │
│               │  │               │  │               │
│ GET  /health  │  │ GET  /health  │  │ GET  /health  │
│ POST /inference│ │ POST /inference│ │ POST /inference│
│   -stream     │  │   -stream     │  │   -stream     │
└───────────────┘  └───────────────┘  └───────────────┘
```

## Testing

### Test Health Endpoint
```bash
# Start server
./build/bin/whisper-server -m models/ggml-base.en.bin

# Check health
curl http://localhost:8080/health | jq
```

### Test Streaming with job_id
```bash
curl -X POST http://localhost:8080/inference-stream \
  -F "file=@samples/jfk.wav" \
  -F "job_id=test-job-123"
```

### Test Worker Script (Mock Mode)
To test without an orchestrator, you can modify the script to skip registration:
```bash
# Comment out the registration and heartbeat sections
# Or set up a mock orchestrator endpoint
```

## Next Steps: Building the Orchestrator

The worker system is now ready. To complete the distributed system, you need to:

1. **Create Phoenix application** (see `WORKER_SYSTEM.md` for full plan)
2. **Implement WorkerRegistry** (GenServer for tracking workers)
3. **Implement JobQueue** (in-memory job queue)
4. **Create Internal API** (worker registration endpoints)
5. **Create Public API** (user-facing transcription API)
6. **Implement Job Processor** (dispatch jobs to workers)

Refer to `WORKER_SYSTEM.md` for the complete implementation plan with code examples.

## Benefits

- **Horizontal Scaling**: Add more workers to handle more load
- **High Availability**: If one worker dies, others continue
- **Load Balancing**: Orchestrator distributes work evenly
- **GPU Utilization**: Run workers on multiple GPU machines
- **Graceful Degradation**: System continues with fewer workers
- **Monitoring**: Health checks and metrics via orchestrator

## Deployment Examples

### Single Machine (Development)
```bash
# Terminal 1: Worker
export WORKER_ID=dev-worker-1
export SHARED_SECRET=dev-secret
export MODEL=models/ggml-base.en.bin
./scripts/whisper-worker.sh

# Terminal 2: Orchestrator (to be implemented)
cd whisper_orchestrator
export WORKER_SHARED_SECRET=dev-secret
mix phx.server
```

### Multiple Machines (Production)
```bash
# Machine 1: Orchestrator
docker run -p 4000:4000 \
  -e SECRET_KEY_BASE=xxx \
  -e WORKER_SHARED_SECRET=xxx \
  whisper-orchestrator:latest

# Machine 2-N: Workers
docker run --gpus all \
  -e WORKER_ID=gpu-1 \
  -e ORCHESTRATOR_URL=http://orchestrator:4000 \
  -e SHARED_SECRET=xxx \
  -e MODEL=models/ggml-large-v3-turbo.bin \
  whisper-worker:latest
```

## Compatibility

- ✅ Works with existing whisper-server functionality
- ✅ Backward compatible (no breaking changes)
- ✅ Can run standalone without orchestrator
- ✅ Optional job_id parameter for streaming

## License

Same as whisper.cpp (MIT License)

## Contributing

See main whisper.cpp README for contribution guidelines.

