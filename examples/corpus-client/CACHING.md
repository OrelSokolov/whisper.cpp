# Transcription Caching - Кэширование транскрипций

## Обзор

Corpus client автоматически кэширует результаты транскрипции, чтобы избежать повторных вызовов Whisper сервера для одинаковых файлов.

## Как работает

### 1. Вычисление MD5 хэша

При первой обработке файла вычисляется MD5 хэш:

```rust
MD5(audio_file) → "a1b2c3d4e5f6..."
```

### 2. Сохранение в кэш

Результаты транскрипции сохраняются в `.cache/corpus-client/`:

```
.cache/corpus-client/
└── a1b2c3d4e5f6...json  ← Timestamps JSON
```

### 3. Проверка при следующем запуске

При повторной обработке того же файла:

```
1. Вычисляется MD5 хэш файла
2. Проверяется наличие кэша
3. Если найден → загружается из кэша
4. Если не найден → транскрибируется и сохраняется
```

## Преимущества

### Скорость

**Первый запуск**:
```bash
./corpus-client --audio-file audio.mp3
# Transcribing: 3-5 минут
# Splitting: 2-3 секунды
# ИТОГО: ~3-5 минут
```

**Второй запуск (с кэшом)**:
```bash
./corpus-client --audio-file audio.mp3
# ✓ Cache hit! Loaded 290 segments from cache
# Splitting: 2-3 секунды
# ИТОГО: ~2-3 секунды ✅

УСКОРЕНИЕ: 60-150x!
```

### Использование

**Идеально для**:
- Экспериментов с параметрами (`--min-duration`, `--max-duration`)
- Тестирования разных форматов (`--format wav/mp3/flac`)
- Повторной генерации датасета
- Исправления ошибок без повторной транскрипции

## Логирование

### Cache Hit (найден в кэше)

```
[INFO] Whisper Corpus Builder Client v0.1.0
[INFO] Piper TTS mode: WAV 22050Hz mono, 0.5-30s segments, vowel hotfix enabled
[INFO] Step 1/2: Transcribing audio...
[INFO] ✓ Cache hit! Loaded 290 segments from cache (hash: a1b2c3d4)
[INFO] Using cached transcription, skipping Whisper server
[INFO] Step 2/2: Splitting audio into segments...
```

### Cache Miss (не найден, транскрибируем)

```
[INFO] Whisper Corpus Builder Client v0.1.0
[INFO] Piper TTS mode: WAV 22050Hz mono, 0.5-30s segments, vowel hotfix enabled
[INFO] Step 1/2: Transcribing audio...
[INFO] Cache miss (hash: a1b2c3d4)
[INFO] Connecting to Whisper server at localhost:8765...
[INFO] ✓ Connected to Whisper server
...
[INFO] ✓ Saved to cache: .cache/corpus-client/a1b2c3d4....json (hash: a1b2c3d4)
```

## Управление кэшом

### Принудительная повторная транскрипция

Используйте флаг `--no-cache`:

```bash
./corpus-client --audio-file audio.mp3 --no-cache
```

Это полезно когда:
- Обновлена модель Whisper
- Изменены параметры транскрипции (язык, etc.)
- Подозреваете ошибку в кэше

### Расположение кэша

По умолчанию: `.cache/corpus-client/` (в текущей директории)

```bash
ls -la .cache/corpus-client/
# a1b2c3d4e5f6....json
# b2c3d4e5f6a7....json
# ...
```

### Очистка кэша

```bash
# Удалить весь кэш
rm -rf .cache/corpus-client/

# Удалить старые записи (> 30 дней)
find .cache/corpus-client/ -name "*.json" -mtime +30 -delete
```

## Структура кэша

### Формат файла

Каждый файл кэша содержит `TimestampsFile` в JSON:

```json
{
  "version": "1.0",
  "source": "whisper.cpp",
  "audio_file": "/path/to/audio.mp3",
  "total_duration": 3600.5,
  "language": "ru",
  "segments": [
    {
      "index": 0,
      "text": "Добро пожаловать.",
      "start": 0.0,
      "end": 2.5,
      "confidence": null
    },
    ...
  ]
}
```

### Именование файлов

Имя файла = MD5 хэш исходного аудио:

```
audio.mp3 (MD5: a1b2c3d4...) → a1b2c3d4....json
```

**Важно**: Разные файлы с одинаковым содержанием имеют один кэш!

## Примеры использования

### Эксперименты с параметрами

```bash
# Первый запуск - транскрибация (5 минут)
./corpus-client --audio-file audio.mp3 \
    --min-duration 1.0 --max-duration 10.0

# Изменили параметры - используется кэш (3 секунды!)
./corpus-client --audio-file audio.mp3 \
    --min-duration 0.5 --max-duration 15.0

# Ещё раз с другими параметрами - снова кэш (3 секунды!)
./corpus-client --audio-file audio.mp3 \
    --min-duration 2.0 --max-duration 8.0 \
    --format flac
```

### Принудительное обновление

```bash
# Обновили модель Whisper на сервере
./corpus-client --audio-file audio.mp3 --no-cache
```

### Batch обработка с кэшом

```bash
# Обработка нескольких файлов
for file in podcasts/*.mp3; do
    ./corpus-client --audio-file "$file" \
        --output-dir "dataset_$(basename "$file" .mp3)"
done

# При повторном запуске все используют кэш!
```

## Статистика кэша

Размер кэша зависит от количества сегментов:

| Сегменты | Размер JSON |
|----------|-------------|
| 100 | ~50 KB |
| 300 | ~150 KB |
| 1000 | ~500 KB |

**Вывод**: Кэш занимает мало места, можно хранить сотни файлов.

## Инвалидация кэша

Кэш автоматически инвалидируется при:
- Изменении содержимого файла (MD5 изменится)
- Ручном удалении `.cache/`

Кэш НЕ инвалидируется при:
- Переименовании файла (MD5 не изменится)
- Перемещении файла (MD5 не изменится)
- Изменении параметров клиента

## Best Practices

### 1. Используйте кэш по умолчанию

Кэш включён автоматически, ничего не нужно настраивать.

### 2. Периодически очищайте старый кэш

```bash
# Раз в месяц
find .cache/corpus-client/ -name "*.json" -mtime +30 -delete
```

### 3. Используйте --no-cache при смене модели

```bash
# Обновили модель на сервере
./corpus-client --audio-file audio.mp3 --no-cache
```

### 4. Делайте backup кэша для больших файлов

```bash
# Если транскрибация занимает часы
tar -czf transcription_cache_backup.tar.gz .cache/
```

## Troubleshooting

### Кэш не работает

Проверьте:
```bash
# Права на запись
ls -la .cache/corpus-client/

# Наличие файлов
ls .cache/corpus-client/*.json

# Лог должен показывать "Cache hit" или "Cache miss"
./corpus-client --audio-file audio.mp3 --verbose
```

### Кэш использует старую версию

```bash
# Принудительно обновить
./corpus-client --audio-file audio.mp3 --no-cache
```

### Много места занимает

```bash
# Посмотреть размер
du -sh .cache/corpus-client/

# Очистить всё
rm -rf .cache/corpus-client/

# Или оставить только свежие
find .cache/corpus-client/ -name "*.json" -mtime +7 -delete
```

## Производительность

### С кэшом

```
Первый запуск:  5 минут (транскрибация)
Второй запуск:  3 секунды (кэш + нарезка)
Третий запуск:  3 секунды (кэш + нарезка)

ЭКОНОМИЯ: 4 минуты 57 секунд на каждый повторный запуск!
```

### Сценарии

**Разработка датасета** (10 итераций с разными параметрами):
- Без кэша: 5 мин × 10 = 50 минут
- С кэшом: 5 мин + 3 сек × 9 = ~5.5 минут
- **Экономия: 45 минут!** 🎉

**Batch обработка** (100 файлов, некоторые повторяются):
- Без кэша: 5 мин × 100 = 500 минут
- С кэшом: зависит от дубликатов
- **Экономия: значительная!**

## См. также

- [cache.rs](src/cache.rs) - Реализация
- [README.md](README.md) - Основная документация
- [PIPER_TTS.md](PIPER_TTS.md) - Piper TTS руководство

