#!/bin/bash
set -e

# === Configuration ===
WORKER_ID="${WORKER_ID:-worker-$(hostname)-$$}"
WORKER_PORT="${WORKER_PORT:-8080}"
WORKER_HOST="${WORKER_HOST:-$(hostname -I | awk '{print $1}')}"
ORCHESTRATOR_URL="${ORCHESTRATOR_URL:-http://localhost:4000}"
SHARED_SECRET="${SHARED_SECRET:?SHARED_SECRET required}"
MODEL="${MODEL:-models/ggml-base.en.bin}"
WHISPER_BIN="${WHISPER_BIN:-./build/bin/whisper-server}"
HEARTBEAT_INTERVAL="${HEARTBEAT_INTERVAL:-20}"

# === Functions ===
cleanup() {
    echo "Shutting down..."
    if [ ! -z "$WHISPER_PID" ]; then
        kill $WHISPER_PID 2>/dev/null || true
        wait $WHISPER_PID 2>/dev/null || true
    fi
    
    # Unregister from orchestrator
    curl -X DELETE "$ORCHESTRATOR_URL/internal/workers/$WORKER_ID" \
        -H "Authorization: Bearer $SHARED_SECRET" \
        -s > /dev/null || true
    
    echo "Cleanup complete"
    exit 0
}

trap cleanup SIGINT SIGTERM

# === Start whisper.cpp server ===
echo "Starting whisper-server..."
$WHISPER_BIN -m "$MODEL" --port "$WORKER_PORT" &
WHISPER_PID=$!

# Wait for server to be ready
echo "Waiting for server to start..."
for i in {1..30}; do
    if curl -s "http://localhost:$WORKER_PORT/health" > /dev/null 2>&1; then
        echo "Server is ready!"
        break
    fi
    sleep 1
done

if ! kill -0 $WHISPER_PID 2>/dev/null; then
    echo "Failed to start whisper-server"
    exit 1
fi

# === Register with orchestrator ===
echo "Registering with orchestrator at $ORCHESTRATOR_URL..."
REGISTER_RESPONSE=$(curl -X POST "$ORCHESTRATOR_URL/internal/workers/register" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $SHARED_SECRET" \
    -d "{
        \"worker_id\": \"$WORKER_ID\",
        \"url\": \"http://$WORKER_HOST:$WORKER_PORT\",
        \"model\": \"$MODEL\"
    }" \
    -s -w "\n%{http_code}")

HTTP_CODE=$(echo "$REGISTER_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" != "200" ] && [ "$HTTP_CODE" != "201" ]; then
    echo "Failed to register with orchestrator (HTTP $HTTP_CODE)"
    cleanup
fi

echo "Successfully registered as $WORKER_ID"

# === Heartbeat loop ===
echo "Starting heartbeat loop (every ${HEARTBEAT_INTERVAL}s)..."
while kill -0 $WHISPER_PID 2>/dev/null; do
    sleep $HEARTBEAT_INTERVAL
    
    curl -X POST "$ORCHESTRATOR_URL/internal/workers/$WORKER_ID/heartbeat" \
        -H "Authorization: Bearer $SHARED_SECRET" \
        -s > /dev/null 2>&1 || {
            echo "Warning: Heartbeat failed"
        }
done

echo "Whisper server stopped unexpectedly"
cleanup

