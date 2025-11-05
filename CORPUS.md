# Whisper.cpp Corpus Builder

Система для автоматического построения корпуса данных для обучения нейросетей TTS (Text-to-Speech) на основе аудиозаписей с YouTube.

## Обзор

Corpus Builder - это набор инструментов для создания высококачественного датасета для обучения TTS моделей. Система автоматически:

1. Скачивает аудио с YouTube
2. Транскрибирует его через Whisper
3. Получает и объединяет временные метки (timestamps)
4. Разбивает исходный аудиофайл на множество мелких сегментов

## Архитектура

```
┌─────────────────┐
│   YouTube URL   │
└────────┬────────┘
         │ yt-dlp
         ▼
┌─────────────────┐
│   audio.mp3     │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────┐
│  Whisper Transcription      │
│  (with --merge-timestamps)  │
└─────────┬───────────────────┘
          │
          ▼
┌─────────────────────────────┐
│  Merged Timestamps JSON     │
│  [{text, start, end}, ...]  │
└─────────┬───────────────────┘
          │
          ▼
┌─────────────────────────────┐
│  Audio Splitter (corpus)    │
└─────────┬───────────────────┘
          │
          ▼
┌─────────────────────────────┐
│  Dataset                    │
│  ├─ segment_0001.mp3        │
│  ├─ segment_0001.txt        │
│  ├─ segment_0002.mp3        │
│  ├─ segment_0002.txt        │
│  └─ ...                     │
└─────────────────────────────┘
```

## Компоненты

### 1. Whisper Worker Server

WebSocket сервер для транскрибации аудио. Запускается командой:

```bash
cd examples/worker-rs
cargo build --release
./target/release/whisper-worker-rs --model ../../models/ggml-base.bin
```

Параметры:
- `--model` - путь к модели Whisper
- `--host` - хост (по умолчанию: 0.0.0.0)
- `--port` - порт (по умолчанию: 8765)
- `--merge-timestamps` - включить режим объединения временных меток

### 2. Corpus Client (Rust)

Клиент для взаимодействия с Whisper сервером и управления пайплайном.

Расположение: `examples/corpus-client/`

Использование:

```bash
cd examples/corpus-client
cargo build --release

# Полный пайплайн
./target/release/corpus-client --youtube-url "https://www.youtube.com/watch?v=..." \
                                --output-dir ./dataset

# Только транскрипция существующего файла
./target/release/corpus-client --audio-file audio.mp3 \
                                --output-dir ./dataset

# Только разбивка по готовому JSON
./target/release/corpus-client --split-only \
                                --audio-file audio.mp3 \
                                --timestamps timestamps.json \
                                --output-dir ./dataset
```

### 3. Timestamp Merging (--merge-timestamps)

Режим объединения временных меток решает проблему фрагментации предложений.

#### Проблема

Whisper часто разбивает одно предложение на несколько сегментов:

```json
[
  {"text": "Привет", "start": 0.0, "end": 0.5},
  {"text": ", как дела", "start": 0.5, "end": 1.2},
  {"text": "?", "start": 1.2, "end": 1.5}
]
```

Для TTS нужно:

```json
[
  {"text": "Привет, как дела?", "start": 0.0, "end": 1.5}
]
```

#### Алгоритм объединения

1. **Обычный стриминг** (без `--merge-timestamps`):
   - Каждый чанк обрабатывается и отдается сразу
   - Быстро, но фрагментировано

2. **Стриминг с `--merge-timestamps`**:
   - Чанки накапливаются в буфере
   - Проверяется завершенность предложения (наличие `.`, `!`, `?` в конце)
   - Если чанк не конечный И предложение незавершенное → ждем следующий чанк
   - Если чанк конечный ИЛИ предложение завершено → объединяем и отдаем

#### Правила объединения внутри чанка

Даже внутри одного завершенного чанка могут быть разрывы:

```
"Привет, " + "мир" → "Привет, мир"  (разрыв на запятой)
```

Сегменты объединяются, если:
- Между ними нет точки/восклицания/вопроса
- Временной промежуток < 1.5 секунд
- Текст не начинается с заглавной буквы (кроме первого)

### 4. Audio Splitter (corpus binary)

Утилита для разбивки аудиофайла на сегменты согласно временным меткам.

```bash
corpus --audio audio.mp3 \
       --timestamps timestamps.json \
       --output-dir ./dataset \
       --format mp3 \
       --min-duration 1.0 \
       --max-duration 15.0
```

Параметры:
- `--audio` - исходный аудиофайл
- `--timestamps` - JSON файл с временными метками
- `--output-dir` - директория для выходных файлов
- `--format` - формат (mp3, wav, flac)
- `--min-duration` - минимальная длительность сегмента (сек)
- `--max-duration` - максимальная длительность сегмента (сек)
- `--sample-rate` - частота дискретизации (опционально)

Выходная структура:

```
dataset/
├── metadata.json          # Сводная информация (JSON)
├── metadata.csv           # Piper TTS формат (filename|text)
├── wavs/
│   ├── 000001.wav        # Аудио сегмент (22050Hz mono)
│   ├── 000001.txt        # Транскрипция
│   ├── 000002.wav
│   ├── 000002.txt
│   └── ...
└── rejected/              # Сегменты, не прошедшие фильтрацию
    └── ...
```

## Полный пример использования

### Шаг 1: Запуск Whisper сервера

```bash
cd examples/worker-rs
cargo build --release
./target/release/whisper-worker-rs \
    --model ../../models/ggml-large-v3.bin \
    --merge-timestamps
```

### Шаг 2: Скачивание и обработка

```bash
# Автоматический пайплайн
cd examples/corpus-client
cargo run --release -- \
    --youtube-url "https://www.youtube.com/watch?v=dQw4w9WgXcQ" \
    --output-dir ./my_dataset \
    --min-duration 2.0 \
    --max-duration 10.0 \
    --format mp3
```

Или вручную:

```bash
# 1. Скачивание
yt-dlp -x --audio-format mp3 -o "audio.%(ext)s" "https://www.youtube.com/watch?v=..."

# 2. Транскрипция
cargo run --release -- \
    --audio-file audio.mp3 \
    --output-timestamps timestamps.json

# 3. Разбивка
cargo run --release -- \
    --split-only \
    --audio-file audio.mp3 \
    --timestamps timestamps.json \
    --output-dir ./dataset
```

### Шаг 3: Проверка результата

```bash
# Просмотр метаданных
cat dataset/metadata.json

# Количество сегментов
ls dataset/segments/*.mp3 | wc -l

# Случайная проверка
mpv dataset/segments/000042.mp3
cat dataset/segments/000042.txt
```

## Формат данных

### Timestamps JSON

```json
{
  "version": "1.0",
  "source": "whisper.cpp",
  "audio_file": "audio.mp3",
  "total_duration": 3600.5,
  "language": "ru",
  "segments": [
    {
      "index": 0,
      "text": "Добро пожаловать в этот подкаст.",
      "start": 0.0,
      "end": 2.5,
      "confidence": 0.95
    },
    {
      "index": 1,
      "text": "Сегодня мы поговорим о машинном обучении.",
      "start": 2.8,
      "end": 5.6,
      "confidence": 0.92
    }
  ]
}
```

### Metadata JSON

```json
{
  "created_at": "2025-11-05T12:34:56Z",
  "source_audio": "audio.mp3",
  "source_duration": 3600.5,
  "total_segments": 1542,
  "accepted_segments": 1489,
  "rejected_segments": 53,
  "min_duration": 2.0,
  "max_duration": 10.0,
  "avg_duration": 4.8,
  "total_dataset_duration": 7147.2,
  "language": "ru",
  "format": "mp3",
  "sample_rate": 22050
}
```

## Фильтрация и качество

Сегменты автоматически фильтруются:

1. **По длительности**: `min_duration <= duration <= max_duration`
2. **По тишине**: отбрасываются сегменты с >30% тишины
3. **По тексту**: 
   - Минимум 5 символов
   - Отсутствие музыкальных нотаций `[♪]`
   - Отсутствие звуковых эффектов `[Sound Effects]`
4. **По качеству аудио**:
   - Проверка на клиппинг
   - Проверка на шум (опционально)

## Интеграция с существующими инструментами

### Использование с Coqui TTS

```bash
# Конвертация в формат LJSpeech
corpus-client --convert-to ljspeech \
              --input-dir dataset \
              --output-dir ljspeech_dataset
```

### Использование с XTTS

```bash
# XTTS требует wav 22050Hz
corpus-client --audio-file audio.mp3 \
              --output-dir xtts_dataset \
              --format wav \
              --sample-rate 22050
```

## Зависимости

### Системные

```bash
# Ubuntu/Debian
sudo apt-get install ffmpeg libsoxr-dev

# macOS
brew install ffmpeg libsoxr

# Arch Linux
sudo pacman -S ffmpeg libsoxr
```

### Python (опционально, для yt-dlp)

```bash
pip install yt-dlp
```

### Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Производительность

На типичном ПК (CPU: Intel i7, RAM: 16GB):

- Скачивание: зависит от интернета (~5-10 минут на час видео)
- Транскрибация: ~0.1-0.3x realtime (10 минут аудио → 30-90 секунд)
- Разбивка: ~50-100x realtime (очень быстро)

**Итого**: 1 час аудио → готовый датасет за ~5-15 минут

## Ограничения и best practices

1. **Качество исходного аудио**: 
   - Предпочтительно: чистая речь, без музыки
   - Избегайте: интервью с перебиванием, сильный фоновый шум

2. **Размер модели Whisper**:
   - `tiny`, `base`: быстро, но низкая точность
   - `small`, `medium`: баланс
   - `large-v3`: лучшая точность, медленнее

3. **Объединение timestamps**:
   - Для TTS: всегда используйте `--merge-timestamps`
   - Для субтитров: можно без объединения

4. **Хранение**:
   - MP3 320kbps: ~1.5-2 MB на минуту
   - WAV 22050Hz: ~2.5 MB на минуту
   - Планируйте ~3-5 GB на час исходного аудио

## Troubleshooting

### "Connection refused" при подключении к серверу

```bash
# Проверьте, запущен ли сервер
netstat -tuln | grep 8765

# Проверьте логи сервера
./whisper-worker-rs --model model.bin --verbose
```

### Плохое качество транскрипции

1. Используйте большую модель (`large-v3`)
2. Проверьте качество исходного аудио
3. Укажите правильный язык: `--language ru`

### Слишком много rejected segments

1. Уменьшите `--min-duration`
2. Проверьте параметры фильтрации
3. Просмотрите `rejected/` директорию для анализа

## Roadmap

- [ ] Поддержка batch обработки (несколько файлов)
- [ ] Автоматическая очистка от музыкальных вставок
- [ ] Детекция и фильтрация по полу/возрасту говорящего
- [ ] Web интерфейс для просмотра и редактирования датасета
- [ ] Поддержка diarization (разделение по спикерам)
- [ ] Экспорт в форматы других TTS фреймворков

## Лицензия

Corpus Builder является частью whisper.cpp и распространяется под той же лицензией MIT.

## Поддержка

Issues: https://github.com/ggerganov/whisper.cpp/issues
Discussions: https://github.com/ggerganov/whisper.cpp/discussions

