#!/bin/bash

# Быстрый запуск сервера стриминга с Large Turbo моделью

set -e

MODEL="models/ggml-large-v3-turbo.bin"
HOST="127.0.0.1"
PORT="8080"
THREADS=$(nproc)

echo "🚀 Запуск Whisper Streaming Server (Large Turbo)"
echo "================================================"
echo ""

# Проверка модели
if [ ! -f "$MODEL" ]; then
    echo "❌ Модель не найдена: $MODEL"
    echo ""
    echo "Скачайте модель:"
    echo "  bash models/download-ggml-model.sh large-v3-turbo"
    exit 1
fi

# Проверка сервера
if [ ! -f "./build/bin/whisper-server" ]; then
    echo "❌ Сервер не собран"
    echo ""
    echo "Выполните сборку:"
    echo "  cmake -B build"
    echo "  cmake --build build --target whisper-server"
    exit 1
fi

echo "✅ Модель:  $MODEL"
echo "✅ Адрес:   http://$HOST:$PORT"
echo "✅ Потоки:  $THREADS"
echo ""
echo "📡 Endpoints:"
echo "   • Стриминг: http://$HOST:$PORT/inference-stream"
echo "   • Обычный:  http://$HOST:$PORT/inference"
echo ""
echo "🧪 Тест:"
echo "   python3 test_streaming_client.py samples/jfk.wav"
echo ""
echo "Нажмите Ctrl+C для остановки"
echo "================================================"
echo ""

./build/bin/whisper-server \
    -m "$MODEL" \
    --host "$HOST" \
    --port "$PORT" \
    -t "$THREADS" \
    --language ru
    --convert

