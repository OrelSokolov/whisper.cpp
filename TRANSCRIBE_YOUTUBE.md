# YouTube Transcription Pipeline

Автоматический пайплайн для транскрипции YouTube видео с использованием yt-dlp и whisper.cpp worker.

## Установка зависимостей

### 1. Установка yt-dlp (standalone binary)

```bash
sudo wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /usr/local/bin/yt-dlp
sudo chmod a+rx /usr/local/bin/yt-dlp
```

### 2. Установка Ruby и зависимостей для client.rb

```bash
# Установка Ruby
sudo apt install ruby

# Установка библиотеки websocket-client-simple
gem install websocket-client-simple
```

## Использование

### Базовое использование

```bash
./transcribe "https://www.youtube.com/watch?v=VIDEO_ID"
```

### Примеры

```bash
# Транскрипция видео с YouTube
./transcribe "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# Транскрипция короткого видео
./transcribe "https://youtu.be/SHORT_ID"

# Транскрипция с временной меткой
./transcribe "https://www.youtube.com/watch?v=VIDEO_ID&t=120"
```

## Как это работает

1. **Скачивание аудио**: yt-dlp извлекает аудиодорожку из YouTube видео и конвертирует в MP3
2. **Отправка на сервер**: Аудио файл отправляется на whisper worker сервер (хост: `micro`)
3. **Транскрипция**: Whisper обрабатывает аудио и возвращает текст
4. **Вывод**: Текст выводится в консоль без таймстампов (опция `--no-timestamps`)
5. **Очистка**: Временный MP3 файл автоматически удаляется

## Архитектура

```
┌──────────────┐      ┌─────────────┐      ┌──────────────────┐
│   YouTube    │─────▶│   yt-dlp    │─────▶│   temp.mp3       │
└──────────────┘      └─────────────┘      └──────────────────┘
                                                      │
                                                      ▼
┌──────────────┐      ┌─────────────┐      ┌──────────────────┐
│   stdout     │◀─────│  client.rb  │─────▶│ whisper worker   │
│   (текст)    │      │             │ WS   │   (micro:8765)   │
└──────────────┘      └─────────────┘      └──────────────────┘
```

## Опции client.rb

Скрипт `transcribe` использует следующие опции:

- `--host micro` - подключение к серверу на хосте "micro"
- `--no-timestamps` - вывод только текста без временных меток

### Дополнительные опции client.rb

Вы можете запустить client.rb напрямую с другими опциями:

```bash
# С таймстампами и метаинформацией
ruby examples/worker/client.rb --host localhost audio.mp3

# Без таймстампов
ruby examples/worker/client.rb --host localhost --no-timestamps audio.mp3

# На другом порту
ruby examples/worker/client.rb --host micro --port 9000 --no-timestamps audio.mp3

# Справка
ruby examples/worker/client.rb --help
```

## Требования

- **yt-dlp**: Standalone binary для скачивания видео
- **Ruby**: Интерпретатор Ruby (>= 2.0)
- **websocket-client-simple**: Ruby gem для WebSocket
- **whisper worker**: Запущенный WebSocket сервер на хосте `micro` (порт 8765)

## Устранение неполадок

### yt-dlp не найден

```bash
# Проверьте установку
which yt-dlp
yt-dlp --version

# Переустановите если нужно
sudo wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /usr/local/bin/yt-dlp
sudo chmod a+rx /usr/local/bin/yt-dlp
```

### Не удается подключиться к серверу

```bash
# Проверьте доступность хоста
ping micro

# Проверьте порт
nc -zv micro 8765

# Проверьте запущен ли whisper worker
ssh micro "ps aux | grep whisper"
```

### Ruby gem не установлен

```bash
# Установите websocket-client-simple
gem install websocket-client-simple

# Или через bundler (если есть Gemfile)
cd examples/worker
bundle install
```

## Интеграция в другие pipeline

### Использование в скрипте

```bash
#!/bin/bash
URLS=(
    "https://www.youtube.com/watch?v=VIDEO1"
    "https://www.youtube.com/watch?v=VIDEO2"
    "https://www.youtube.com/watch?v=VIDEO3"
)

for url in "${URLS[@]}"; do
    echo "Обработка: $url"
    ./transcribe "$url" > "output_$(date +%s).txt"
done
```

### Перенаправление вывода

```bash
# Сохранить в файл
./transcribe "URL" > transcript.txt

# Сохранить только текст (без служебных сообщений)
./transcribe "URL" 2>/dev/null > transcript.txt

# Передать в другую программу
./transcribe "URL" | grep -i "ключевое слово"
```

## Производительность

- **Скачивание**: зависит от скорости интернета и размера видео
- **Транскрипция**: зависит от модели whisper и мощности сервера
- **Очистка**: мгновенно

Типичное 10-минутное видео обрабатывается за 1-3 минуты (в зависимости от модели).

## Лицензия

Часть проекта whisper.cpp

