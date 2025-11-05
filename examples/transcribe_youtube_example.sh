#!/usr/bin/env bash
# Пример использования скрипта transcribe для обработки нескольких YouTube видео

# Массив URL для обработки
URLS=(
    "https://www.youtube.com/watch?v=VIDEO_ID_1"
    "https://www.youtube.com/watch?v=VIDEO_ID_2"
    # Добавьте больше URL по необходимости
)

# Директория для сохранения результатов
OUTPUT_DIR="./transcriptions"
mkdir -p "$OUTPUT_DIR"

# Обработка каждого URL
for i in "${!URLS[@]}"; do
    url="${URLS[$i]}"
    output_file="$OUTPUT_DIR/transcript_$(date +%Y%m%d_%H%M%S)_$((i+1)).txt"
    
    echo "========================================="
    echo "Обработка видео $((i+1)) из ${#URLS[@]}"
    echo "URL: $url"
    echo "Результат будет сохранен в: $output_file"
    echo "========================================="
    
    # Запуск транскрипции (stderr в /dev/null, только транскрипт в файл)
    if ./transcribe "$url" 2>/dev/null > "$output_file"; then
        echo "✓ Успешно транскрибировано: $output_file"
    else
        echo "✗ Ошибка при транскрипции: $url"
    fi
    
    echo ""
    
    # Небольшая пауза между запросами (опционально)
    sleep 2
done

echo "Все видео обработаны. Результаты в директории: $OUTPUT_DIR"

