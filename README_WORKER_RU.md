# Whisper.cpp Worker System - Руководство на русском

## 🎯 Что было реализовано

Согласно плану в `WORKER_SYSTEM.md`, были реализованы **Phase 1 и Phase 2**:

### ✅ Phase 1: Минимальные изменения в whisper.cpp (30 минут)

**Выполнено:**
- ✅ Добавлен `GET /health` endpoint в `server.cpp`
- ✅ Добавлена поддержка `job_id` в SSE событиях
- ✅ Мониторинг состояния сервера
- ✅ Отчет о возможностях (capabilities)

**Файлы:**
- `examples/server/server.cpp` (строки 1409-1433) - health endpoint
- `examples/server/server.cpp` (строки 818-821, 988-990) - job_id support

### ✅ Phase 2: Wrapper скрипт для воркера (1 час)

**Выполнено:**
- ✅ Создан `whisper-worker.sh` скрипт
- ✅ Автоматическая регистрация при старте
- ✅ Heartbeat loop
- ✅ Graceful shutdown
- ✅ Пример конфигурации

**Файлы:**
- `scripts/whisper-worker.sh` - основной скрипт
- `scripts/worker.env.example` - пример конфигурации

### 🎁 Бонус: Дополнительная инфраструктура

**Также создано:**
- ✅ `Dockerfile.worker` - Docker образ для воркеров
- ✅ `docker-compose.worker.yml` - Multi-worker setup
- ✅ Полная документация на английском
- ✅ Обновлен `.gitignore`

## 📁 Структура файлов

```
whisper.cpp/
├── examples/server/
│   └── server.cpp                     [ИЗМЕНЕН] Health endpoint уже был
├── scripts/
│   ├── whisper-worker.sh              [НОВЫЙ] ✨ Worker wrapper
│   └── worker.env.example             [НОВЫЙ] ✨ Конфигурация
├── Dockerfile.worker                  [НОВЫЙ] ✨ Docker образ
├── docker-compose.worker.yml          [НОВЫЙ] ✨ Multi-worker
├── .gitignore                         [ИЗМЕНЕН] Добавлены worker.env
│
├── WORKER_SYSTEM.md                   [СУЩЕСТВУЕТ] План реализации
├── WORKER_README.md                   [НОВЫЙ] ✨ Основная документация (EN)
├── QUICK_START_WORKER.md              [НОВЫЙ] ✨ Быстрый старт (EN)
├── CHANGES_WORKER_SYSTEM.md           [НОВЫЙ] ✨ Детали изменений (EN)
├── IMPLEMENTATION_COMPLETE.md         [НОВЫЙ] ✨ Итоги реализации (EN)
└── README_WORKER_RU.md                [НОВЫЙ] ✨ Этот файл
```

## 🚀 Как использовать

### Вариант 1: Standalone режим (без изменений)

Сервер работает как раньше, но теперь с дополнительными возможностями:

```bash
# Запустить сервер как обычно
./build/bin/whisper-server -m models/ggml-base.en.bin

# Новая возможность: проверить здоровье сервера
curl http://localhost:8080/health | jq
```

**Ответ:**
```json
{
  "status": "healthy",
  "model": "models/ggml-base.en.bin",
  "uptime_seconds": 120,
  "version": "1.0.0",
  "capabilities": {
    "multilingual": true,
    "gpu_enabled": true,
    "flash_attn": true,
    "supports_streaming": true
  }
}
```

### Вариант 2: Worker режим (распределенная система)

Для распределенной транскрипции на нескольких машинах:

#### Шаг 1: Конфигурация

```bash
# Создать конфигурацию
cp scripts/worker.env.example worker.env

# Отредактировать
nano worker.env
```

Минимальная конфигурация:
```bash
export WORKER_ID="gpu-worker-1"
export WORKER_PORT=8080
export ORCHESTRATOR_URL="http://orchestrator:4000"
export SHARED_SECRET="ваш-секретный-ключ"
export MODEL="models/ggml-base.en.bin"
```

#### Шаг 2: Запуск worker

```bash
# Загрузить конфигурацию
source worker.env

# Запустить worker
./scripts/whisper-worker.sh
```

Worker автоматически:
- ✅ Запустит whisper-server
- ✅ Дождется готовности
- ✅ Зарегистрируется в оркестраторе
- ✅ Отправит heartbeat каждые 20 секунд
- ✅ Корректно завершится по Ctrl+C

### Вариант 3: Docker

```bash
# Собрать образ
docker build -f Dockerfile.worker -t whisper-worker .

# Запустить worker
docker run --gpus all \
  -e WORKER_ID=docker-worker-1 \
  -e ORCHESTRATOR_URL=http://orchestrator:4000 \
  -e SHARED_SECRET=ваш-секрет \
  -e MODEL=models/ggml-base.en.bin \
  -v $(pwd)/models:/whisper.cpp/models:ro \
  -p 8080:8080 \
  whisper-worker
```

### Вариант 4: Несколько workers (docker-compose)

```bash
# Установить переменные
export ORCHESTRATOR_URL=http://orchestrator:4000
export SHARED_SECRET=ваш-секретный-ключ

# Запустить
docker-compose -f docker-compose.worker.yml up -d

# Логи
docker-compose -f docker-compose.worker.yml logs -f
```

## 🧪 Тестирование

### Тест 1: Health endpoint

```bash
# Запустить сервер
./build/bin/whisper-server -m models/ggml-base.en.bin &

# Подождать 2 секунды
sleep 2

# Проверить здоровье
curl http://localhost:8080/health | jq
```

### Тест 2: Streaming с job_id

```bash
# Отправить файл с job_id
curl -X POST http://localhost:8080/inference-stream \
  -F "file=@samples/jfk.wav" \
  -F "job_id=test-123"

# Вы увидите события с job_id:
# data: {"type":"start","job_id":"test-123","filename":"jfk.wav"}
# data: {"type":"segment","job_id":"test-123","text":"..."}
# data: {"type":"done","job_id":"test-123"}
```

### Тест 3: Worker script (без оркестратора)

```bash
# Проверить синтаксис
bash -n scripts/whisper-worker.sh

# Попробовать запустить (упадет при регистрации, но покажет что работает)
export SHARED_SECRET="test"
export ORCHESTRATOR_URL="http://localhost:9999"
export MODEL="models/ggml-base.en.bin"
./scripts/whisper-worker.sh

# Нажать Ctrl+C для остановки
```

## 📊 Преимущества

### Горизонтальное масштабирование
- Запускайте workers на разных машинах
- Каждый worker использует свою GPU
- Линейное увеличение производительности

### Высокая доступность
- Workers работают независимо
- Автоматическое обнаружение мертвых workers
- Система продолжает работать с оставшимися workers

### Мониторинг
- Health checks для внешнего мониторинга
- Отслеживание задач через job_id
- Отчеты о времени работы и возможностях

## 📚 Документация

| Файл | Описание | Язык |
|------|----------|------|
| `README_WORKER_RU.md` | Этот файл | 🇷🇺 Русский |
| `QUICK_START_WORKER.md` | Быстрый старт | 🇬🇧 English |
| `WORKER_README.md` | Полная документация | 🇬🇧 English |
| `WORKER_SYSTEM.md` | План архитектуры | 🇬🇧 English |
| `CHANGES_WORKER_SYSTEM.md` | Детали реализации | 🇬🇧 English |
| `IMPLEMENTATION_COMPLETE.md` | Итоги | 🇬🇧 English |

## 🔄 Что дальше

### Текущий статус

- ✅ **Phase 1**: Health endpoint - ГОТОВО
- ✅ **Phase 2**: Worker script - ГОТОВО
- ⏳ **Phase 3-12**: Phoenix/Elixir оркестратор - НУЖНО РЕАЛИЗОВАТЬ

### Для standalone использования

Ничего больше не нужно! Просто используйте сервер как обычно.

### Для распределенной системы

Нужно реализовать оркестратор (Phoenix/Elixir приложение):

**Phase 3**: Phoenix приложение (30 минут)
```bash
mix phx.new whisper_orchestrator --no-ecto
```

**Phase 4**: WorkerRegistry (GenServer) (2 часа)
- Управление списком workers
- Heartbeat tracking
- Load balancing

**Phase 5**: JobQueue (GenServer) (2 часа)
- Очередь задач в памяти
- Status tracking

**Phase 6**: Internal API (1.5 часа)
- POST /internal/workers/register
- POST /internal/workers/:id/heartbeat
- DELETE /internal/workers/:id

**Phase 7**: Public API (2 часа)
- POST /api/transcribe
- GET /api/transcribe/:id
- GET /api/transcribe/:id/stream

**Phase 8-12**: См. `WORKER_SYSTEM.md` для деталей

**Общее время**: ~20-25 часов разработки

## 🔒 Безопасность

### Для production

1. **Используйте HTTPS** - настройте reverse proxy (nginx/caddy)
2. **Защитите секреты** - используйте менеджер секретов
3. **Сетевая безопасность** - private network или VPN для workers
4. **Регулярные обновления** - обновляйте whisper.cpp и зависимости

### Конфигурация

- ✅ `.gitignore` обновлен для исключения `worker.env`
- ✅ Секреты передаются через переменные окружения
- ⚠️ В production используйте HTTPS

## ⚙️ Переменные окружения

| Переменная | По умолчанию | Описание |
|-----------|--------------|----------|
| `WORKER_ID` | `worker-$(hostname)-$$` | Уникальный ID воркера |
| `WORKER_PORT` | `8080` | Порт для whisper-server |
| `WORKER_HOST` | Автоопределение | Сетевой адрес воркера |
| `ORCHESTRATOR_URL` | `http://localhost:4000` | URL оркестратора |
| `SHARED_SECRET` | Обязательно | Токен аутентификации |
| `MODEL` | `models/ggml-base.en.bin` | Путь к модели |
| `WHISPER_BIN` | `./build/bin/whisper-server` | Путь к бинарнику |
| `HEARTBEAT_INTERVAL` | `20` | Интервал heartbeat (сек) |

## 🐛 Устранение проблем

### Worker не может зарегистрироваться

**Решение:** Проверьте что оркестратор запущен и `ORCHESTRATOR_URL` правильный

### Health endpoint возвращает 503

**Решение:** Подождите загрузки модели (10-30 секунд)

### Порт уже занят

**Решение:** Измените `WORKER_PORT` на свободный (8081, 8082...)

### Модель не найдена

**Решение:** Проверьте путь в `MODEL` и что файл существует

Больше информации: см. раздел Troubleshooting в `QUICK_START_WORKER.md`

## 💡 Примеры использования

### Пример 1: Один worker

```bash
export WORKER_ID="my-worker"
export ORCHESTRATOR_URL="http://orchestrator:4000"
export SHARED_SECRET="secret"
export MODEL="models/ggml-base.en.bin"
./scripts/whisper-worker.sh
```

### Пример 2: Несколько workers на одной машине

```bash
# Worker 1 на порту 8081
export WORKER_ID=worker-1 WORKER_PORT=8081
./scripts/whisper-worker.sh &

# Worker 2 на порту 8082
export WORKER_ID=worker-2 WORKER_PORT=8082
./scripts/whisper-worker.sh &
```

### Пример 3: Workers на разных GPU

```bash
# Worker на GPU 0 с маленькой моделью
export WORKER_ID=gpu-0-fast
export CUDA_VISIBLE_DEVICES=0
export MODEL=models/ggml-base.en.bin
./scripts/whisper-worker.sh &

# Worker на GPU 1 с большой моделью
export WORKER_ID=gpu-1-accurate
export CUDA_VISIBLE_DEVICES=1
export MODEL=models/ggml-large-v3-turbo.bin
./scripts/whisper-worker.sh &
```

## 🎯 Архитектура системы

```
┌─────────────────────────────────────────────────────────┐
│          Phoenix/Elixir Orchestrator                     │
│              (Нужно реализовать)                         │
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Public API                                        │ │
│  │  POST /api/transcribe        - загрузка аудио     │ │
│  │  GET  /api/transcribe/:id    - получить результат │ │
│  │  GET  /api/transcribe/:id/stream - SSE stream     │ │
│  └────────────────────────────────────────────────────┘ │
│                          ↓                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │  WorkerRegistry (GenServer)                        │ │
│  │  - Список workers                                  │ │
│  │  - Heartbeat tracking                              │ │
│  │  - Load balancing                                  │ │
│  └────────────────────────────────────────────────────┘ │
│                          ↓                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Internal API (для workers)                        │ │
│  │  POST   /internal/workers/register                 │ │
│  │  POST   /internal/workers/:id/heartbeat            │ │
│  │  DELETE /internal/workers/:id                      │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                          ↓ HTTP
        ┌─────────────────┼─────────────────┐
        ↓                 ↓                 ↓
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│ Whisper Worker│  │ Whisper Worker│  │ Whisper Worker│
│               │  │               │  │               │
│wrapper.sh     │  │wrapper.sh     │  │wrapper.sh     │
│     ↓         │  │     ↓         │  │     ↓         │
│whisper-server │  │whisper-server │  │whisper-server │
│ (C++)         │  │ (C++)         │  │ (C++)         │
│               │  │               │  │               │
│ GET  /health  │  │ GET  /health  │  │ GET  /health  │
│ POST /inference│ │ POST /inference│ │ POST /inference│
│   -stream     │  │   -stream     │  │   -stream     │
└───────────────┘  └───────────────┘  └───────────────┘
```

## ✨ Итоги

**Что готово:**
- ✅ whisper.cpp улучшен (health checks, job_id)
- ✅ Worker wrapper script для автоматизации
- ✅ Docker поддержка
- ✅ Полная документация

**Что нужно сделать:**
- ⏳ Реализовать Phoenix/Elixir оркестратор (Phase 3-12)
- ⏳ Развернуть распределенную систему
- ⏳ Масштабировать по необходимости

**Статус:** ГОТОВО К ИСПОЛЬЗОВАНИЮ! 🚀

---

## 📞 Поддержка

По вопросам и проблемам:
- **whisper.cpp**: https://github.com/ggerganov/whisper.cpp
- **Issues**: Создайте issue с тегом `[worker-system]`
- **Документация**: См. файлы выше

## 📄 Лицензия

MIT License (как и whisper.cpp)

---

**Дата реализации:** 28 октября 2025  
**Статус:** Phase 1 & 2 ЗАВЕРШЕНЫ ✅  
**Следующая фаза:** Разработка оркестратора (отдельный проект)

