# Corpus Builder - Быстрый старт

## Что это?

Система для автоматического построения датасетов для обучения TTS (Text-to-Speech) моделей из видео YouTube или аудиофайлов.

## Быстрый старт (5 минут)

### 1. Установка зависимостей

```bash
# Ubuntu/Debian
sudo apt-get install ffmpeg yt-dlp

# macOS
brew install ffmpeg yt-dlp

# Или через pip
pip install yt-dlp
```

### 2. Сборка проектов

```bash
# Сборка whisper server
cd examples/worker-rs
cargo build --release

# Сборка corpus client
cd ../corpus-client
cargo build --release
```

### 3. Запуск сервера

```bash
cd examples/worker-rs
./target/release/whisper-worker-rs \
    --model ../../models/ggml-large-v3.bin \
    --merge-timestamps
```

### 4. Создание датасета

```bash
# В другом терминале
cd examples/corpus-client

# Из YouTube видео
./target/release/corpus-client \
    --youtube-url "https://www.youtube.com/watch?v=YOUR_VIDEO_ID" \
    --output-dir ./my_dataset

# Или из локального файла
./target/release/corpus-client \
    --audio-file audio.mp3 \
    --output-dir ./my_dataset
```

### 5. Результат

```
my_dataset/
├── metadata.json          # Информация о датасете
├── segments/
│   ├── 000001.mp3        # Аудио сегмент
│   ├── 000001.txt        # Транскрипция
│   ├── 000002.mp3
│   ├── 000002.txt
│   └── ...
└── rejected/              # Отфильтрованные сегменты
```

## Основные параметры

### Worker Server (--merge-timestamps)

**Ключевая опция** для создания датасета TTS:

```bash
--merge-timestamps    # Объединяет разбитые предложения
```

Без этой опции:
```
"Привет"
", как дела"
"?"
```

С этой опцией:
```
"Привет, как дела?"
```

### Corpus Client

```bash
--min-duration 2.0       # Минимальная длина сегмента (секунды)
--max-duration 10.0      # Максимальная длина сегмента
--format mp3             # Формат: mp3, wav, flac
--sample-rate 22050      # Частота дискретизации (для TTS обычно 22050)
```

## Примеры использования

### Для русского языка (подкаст)

```bash
# Сервер
./whisper-worker-rs \
    --model ../../models/ggml-large-v3.bin \
    --language ru \
    --merge-timestamps

# Клиент
./corpus-client \
    --youtube-url "https://www.youtube.com/watch?v=..." \
    --output-dir ./ru_podcast_dataset \
    --min-duration 3.0 \
    --max-duration 12.0 \
    --format wav \
    --sample-rate 22050
```

### Для английского языка (аудиокнига)

```bash
# Сервер
./whisper-worker-rs \
    --model ../../models/ggml-large-v3-turbo.bin \
    --language en \
    --merge-timestamps

# Клиент
./corpus-client \
    --audio-file audiobook.m4a \
    --output-dir ./en_audiobook_dataset \
    --min-duration 2.0 \
    --max-duration 15.0
```

### Только получить timestamps (без разбивки)

```bash
./corpus-client \
    --audio-file audio.mp3 \
    --output-timestamps timestamps.json
```

### Разбить файл по готовым timestamps

```bash
./corpus-client \
    --split-only \
    --audio-file audio.mp3 \
    --timestamps timestamps.json \
    --output-dir ./dataset
```

## Проверка результата

```bash
# Показать метаданные
cat my_dataset/metadata.json | jq

# Количество сегментов
ls my_dataset/segments/*.mp3 | wc -l

# Прослушать случайный сегмент
mpv my_dataset/segments/000042.mp3
cat my_dataset/segments/000042.txt
```

## Типичные проблемы

### "Connection refused" при запуске клиента

Сервер не запущен. Проверьте:
```bash
netstat -tuln | grep 8765
```

### Плохое качество транскрипции

1. Используйте модель `large-v3` вместо `base`
2. Укажите правильный язык: `--language ru`
3. Проверьте качество исходного аудио

### Слишком много rejected segments

1. Уменьшите `--min-duration`
2. Проверьте качество аудио (музыка, шум)
3. Посмотрите `rejected/` для анализа

## Производительность

На обычном ПК (Intel i7, 16GB RAM):

- **Скачивание**: ~5-10 мин на час видео (зависит от интернета)
- **Транскрибация**: ~3-10 мин на час аудио (зависит от модели и GPU)
- **Разбивка**: **~2-3 сек** на час аудио (нативная обработка!) ✅

**Итого**: ~10-15 минут на 1 час аудио → готовый датасет

## Следующие шаги

1. **Настройка качества**: экспериментируйте с `min-duration` и `max-duration`
2. **Фильтрация**: просмотрите `rejected/` и настройте фильтры
3. **Объединение датасетов**: создайте несколько датасетов и объедините их
4. **Обучение TTS**: используйте готовый датасет для обучения

## Полная документация

- [CORPUS.md](CORPUS.md) - Подробная документация
- [examples/worker-rs/README.md](examples/worker-rs/README.md) - Документация сервера
- [examples/corpus-client/README.md](examples/corpus-client/README.md) - Документация клиента

## Поддержка

- Issues: https://github.com/ggerganov/whisper.cpp/issues
- Discussions: https://github.com/ggerganov/whisper.cpp/discussions

