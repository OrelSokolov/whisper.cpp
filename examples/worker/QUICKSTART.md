# Быстрый старт - Whisper Worker

## Сборка

Если еще не собрали:

```bash
cd build
cmake ..
make whisper-worker
```

Бинарник будет находиться в `build/bin/whisper-worker`

## Запуск сервера

### Базовый запуск

```bash
./build/bin/whisper-worker -m models/ggml-base.en.bin
```

### С опциями

```bash
# С указанием количества потоков
./build/bin/whisper-worker -m models/ggml-base.en.bin -t 4

# С переводом на английский
./build/bin/whisper-worker -m models/ggml-base.en.bin -tr

# Без временных меток
./build/bin/whisper-worker -m models/ggml-base.en.bin -nt

# С другими опциями
./build/bin/whisper-worker -m models/ggml-base.en.bin -t 4 -p 1 -l en -v
```

### Доступные опции

- `-m, --model FILE` - Путь к файлу модели (обязательно)
- `-t, --threads N` - Количество потоков (по умолчанию: автоматически)
- `-p, --processors N` - Количество процессоров (по умолчанию: 1)
- `-l, --language LANG` - Язык (по умолчанию: `en`)
- `-tr, --translate` - Переводить на английский
- `-nt, --no-timestamps` - Не включать временные метки
- `-v, --verbose` - Подробный вывод
- `-h, --help` - Показать справку

## Проверка работы

После запуска вы увидите:

```
whisper-worker: WebSocket server for audio transcription
Loading model: models/ggml-base.en.bin
Model loaded successfully
WebSocket server listening on port 8765
Ready to accept connections (one at a time)
```

Сервер готов принимать подключения!

## Использование Ruby клиента

В другом терминале:

```bash
cd examples/worker

# Установка зависимостей (если еще не установлены)
bundle install

# Отправка аудио файла
bundle exec ruby client.rb samples/jfk.wav
```

Или напрямую:

```bash
ruby examples/worker/client.rb samples/jfk.wav
```

## Пример полного цикла

Терминал 1 (сервер):
```bash
./build/bin/whisper-worker -m models/ggml-base.en.bin -v
```

Терминал 2 (клиент):
```bash
cd examples/worker
bundle exec ruby client.rb ../samples/jfk.wav
```

## Остановка сервера

Нажмите `Ctrl+C` в терминале, где запущен сервер.

