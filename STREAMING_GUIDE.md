# Whisper.cpp Streaming API Guide

## 🚀 Быстрый старт

### 1. Запустить сервер

```bash
# С базовой моделью (если есть)
./build/bin/whisper-server -m models/ggml-base.en.bin

# Или с другой моделью
./build/bin/whisper-server -m models/ggml-large-v3-turbo.bin

# С поддержкой ffmpeg для конвертации форматов
./build/bin/whisper-server -m models/ggml-base.en.bin --convert
```

Сервер запустится на `http://127.0.0.1:8080`

### 2. Протестировать стриминг

```bash
# Используя Python клиент
python3 test_streaming_client.py samples/jfk.wav

# Или напрямую через curl
curl -N http://127.0.0.1:8080/inference-stream \
  -F file="@samples/jfk.wav" \
  -F temperature="0.0"
```

## 📡 API Endpoints

### `/inference` (без стриминга)
Классический endpoint - возвращает результат после полной обработки.

```bash
curl http://127.0.0.1:8080/inference \
  -F file="@audio.wav" \
  -F response_format="json"
```

### `/inference-stream` (SSE стриминг) ⭐
Новый endpoint - стримит результаты в реальном времени!

```bash
curl -N http://127.0.0.1:8080/inference-stream \
  -F file="@audio.wav" \
  -F language="ru"
```

## 📥 Формат SSE событий

Server-Sent Events приходят в формате:

```
data: {"type":"start","filename":"audio.wav"}

data: {"type":"segment","index":0,"text":" Привет, мир!","start":0.0,"end":1.5}

data: {"type":"segment","index":1,"text":" Как дела?","start":1.5,"end":3.0}

data: {"type":"done"}
```

### Типы событий:

- **start** - начало обработки
  ```json
  {"type": "start", "filename": "audio.wav"}
  ```

- **segment** - распознанный сегмент (приходит в реальном времени)
  ```json
  {
    "type": "segment",
    "index": 0,
    "text": " Распознанный текст",
    "start": 0.5,
    "end": 2.3
  }
  ```

- **done** - обработка завершена
  ```json
  {"type": "done"}
  ```

- **error** - ошибка
  ```json
  {"type": "error", "message": "описание ошибки"}
  ```

## 🔧 Параметры запроса

Все те же параметры что и у `/inference`:

```bash
curl -N http://127.0.0.1:8080/inference-stream \
  -F file="@audio.wav" \
  -F temperature="0.0" \
  -F temperature_inc="0.2" \
  -F language="ru" \
  -F translate="false"
```

## 💻 Примеры клиентов

### Python (включён: `test_streaming_client.py`)

```python
import requests
import json

response = requests.post(
    "http://127.0.0.1:8080/inference-stream",
    files={'file': open('audio.wav', 'rb')},
    data={'language': 'auto'},
    stream=True
)

for line in response.iter_lines():
    if line and line.startswith(b'data: '):
        event = json.loads(line[6:])
        if event['type'] == 'segment':
            print(f"[{event['start']:.1f}s] {event['text']}")
```

### Elixir (для продакшена)

```elixir
# mix.exs: {:httpoison, "~> 2.0"}

HTTPoison.post(
  "http://127.0.0.1:8080/inference-stream",
  {:multipart, [
    {:file, audio_path},
    {"language", "auto"}
  ]},
  [],
  stream_to: self(),
  async: :once
)

# Обработка SSE в receive блоке
```

### JavaScript/TypeScript

```javascript
const formData = new FormData();
formData.append('file', audioFile);
formData.append('language', 'auto');

const response = await fetch('http://127.0.0.1:8080/inference-stream', {
  method: 'POST',
  body: formData
});

const reader = response.body.getReader();
const decoder = new TextDecoder();

while (true) {
  const {done, value} = await reader.read();
  if (done) break;
  
  const chunk = decoder.decode(value);
  const lines = chunk.split('\n\n');
  
  for (const line of lines) {
    if (line.startsWith('data: ')) {
      const event = JSON.parse(line.slice(6));
      if (event.type === 'segment') {
        console.log(`[${event.start}s] ${event.text}`);
      }
    }
  }
}
```

## 🎯 Отличия от `/inference`

| Параметр | `/inference` | `/inference-stream` |
|----------|--------------|---------------------|
| Формат ответа | JSON/text/srt/vtt | SSE (text/event-stream) |
| Когда приходит | После полной обработки | В реальном времени |
| Использование | Простые случаи | Длинные аудио, UI с прогрессом |
| response_format | Поддерживается | Игнорируется (всегда SSE) |

## 🛠️ Сборка

```bash
cd whisper.cpp
cmake -B build
cmake --build build --target whisper-server
```

Сервер будет в `build/bin/whisper-server`

## 📝 Примечания

- SSE использует односторонний поток: сервер → клиент
- Соединение остаётся открытым до завершения обработки
- При обрыве соединения сервер прекращает обработку
- Поддерживаются все параметры whisper (language, translate, VAD и т.д.)

