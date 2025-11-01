#!/bin/bash

# Скрипт запуска сервера стриминга Whisper.cpp
# Использование: ./run-streaming-server.sh [опции]

set -e

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Параметры по умолчанию
MODEL_PATH=""
HOST="127.0.0.1"
PORT="8080"
THREADS=$(nproc)
CONVERT_FLAG=""

# Путь к бинарнику
SERVER_BIN="./build/bin/whisper-server"

print_usage() {
    echo "Использование: $0 [опции]"
    echo ""
    echo "Опции:"
    echo "  -m MODEL    Путь к модели (обязательно)"
    echo "  -h HOST     Хост (по умолчанию: 127.0.0.1)"
    echo "  -p PORT     Порт (по умолчанию: 8080)"
    echo "  -t THREADS  Число потоков (по умолчанию: все доступные)"
    echo "  -c          Включить конвертацию через ffmpeg"
    echo "  --help      Показать эту справку"
    echo ""
    echo "Примеры:"
    echo "  $0 -m models/ggml-base.en.bin"
    echo "  $0 -m models/ggml-large-v3-turbo.bin -p 9090 -c"
    echo ""
}

# Парсинг аргументов
while [[ $# -gt 0 ]]; do
    case $1 in
        -m)
            MODEL_PATH="$2"
            shift 2
            ;;
        -h)
            HOST="$2"
            shift 2
            ;;
        -p)
            PORT="$2"
            shift 2
            ;;
        -t)
            THREADS="$2"
            shift 2
            ;;
        -c)
            CONVERT_FLAG="--convert"
            shift
            ;;
        --help)
            print_usage
            exit 0
            ;;
        *)
            echo -e "${RED}Неизвестная опция: $1${NC}"
            print_usage
            exit 1
            ;;
    esac
done

# Проверка наличия бинарника
if [ ! -f "$SERVER_BIN" ]; then
    echo -e "${RED}Ошибка: Сервер не найден: $SERVER_BIN${NC}"
    echo -e "${YELLOW}Запустите сборку:${NC}"
    echo "  cmake -B build"
    echo "  cmake --build build --target whisper-server"
    exit 1
fi

# Если модель не указана, попробуем найти автоматически
if [ -z "$MODEL_PATH" ]; then
    echo -e "${YELLOW}Модель не указана. Поиск доступных моделей...${NC}"
    
    # Приоритетный список моделей
    MODELS=(
        "models/ggml-large-v3-turbo.bin"
        "models/ggml-large-v3-turbo-q8_0.bin"
        "models/ggml-base.en.bin"
        "models/ggml-base.bin"
        "models/ggml-small.en.bin"
        "models/ggml-small.bin"
        "models/ggml-tiny.en.bin"
        "models/ggml-tiny.bin"
    )
    
    for model in "${MODELS[@]}"; do
        if [ -f "$model" ]; then
            MODEL_PATH="$model"
            echo -e "${GREEN}Найдена модель: $MODEL_PATH${NC}"
            break
        fi
    done
    
    if [ -z "$MODEL_PATH" ]; then
        echo -e "${RED}Ошибка: Модель не найдена!${NC}"
        echo -e "${YELLOW}Скачайте модель:${NC}"
        echo "  bash models/download-ggml-model.sh base.en"
        echo ""
        echo "Или укажите путь к модели:"
        echo "  $0 -m /path/to/model.bin"
        exit 1
    fi
fi

# Проверка существования модели
if [ ! -f "$MODEL_PATH" ]; then
    echo -e "${RED}Ошибка: Файл модели не найден: $MODEL_PATH${NC}"
    exit 1
fi

# Вывод информации о запуске
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Запуск Whisper.cpp Streaming Server${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "Модель:      ${YELLOW}$MODEL_PATH${NC}"
echo -e "Адрес:       ${YELLOW}http://$HOST:$PORT${NC}"
echo -e "Потоки:      ${YELLOW}$THREADS${NC}"
echo -e "FFmpeg:      ${YELLOW}$([ -n "$CONVERT_FLAG" ] && echo "включен" || echo "выключен")${NC}"
echo ""
echo -e "${GREEN}Endpoints:${NC}"
echo -e "  • Стриминг:    ${YELLOW}http://$HOST:$PORT/inference-stream${NC}"
echo -e "  • Обычный:     ${YELLOW}http://$HOST:$PORT/inference${NC}"
echo -e "  • Здоровье:    ${YELLOW}http://$HOST:$PORT/health${NC}"
echo ""
echo -e "${GREEN}Тестирование:${NC}"
echo -e "  ${YELLOW}python3 test_streaming_client.py samples/jfk.wav${NC}"
echo ""
echo -e "${GREEN}Нажмите Ctrl+C для остановки${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Запуск сервера
exec "$SERVER_BIN" \
    -m "$MODEL_PATH" \
    --host "$HOST" \
    --port "$PORT" \
    -t "$THREADS" \
    $CONVERT_FLAG

