# Plan реализации Whisper.cpp Worker System

## 🎯 Общая архитектура

```
┌─────────────────────────────────────────────────────────┐
│              Phoenix Application (Elixir)                │
│                                                           │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Public API (Phoenix Router)                       │  │
│  │  POST /api/transcribe        - upload audio        │  │
│  │  GET  /api/transcribe/:id    - get result         │  │
│  │  GET  /api/transcribe/:id/stream - SSE stream     │  │
│  └────────────────────────────────────────────────────┘  │
│                          ↓                                │
│  ┌────────────────────────────────────────────────────┐  │
│  │  JobQueue (GenServer)                              │  │
│  │  - :queue.new() для задач                          │  │
│  │  - In-memory, быстро                               │  │
│  └────────────────────────────────────────────────────┘  │
│                          ↓                                │
│  ┌────────────────────────────────────────────────────┐  │
│  │  WorkerRegistry (GenServer)                        │  │
│  │  - Список воркеров                                 │  │
│  │  - Статус (idle/busy)                              │  │
│  │  - Heartbeat tracking                              │  │
│  │  - Load balancing                                  │  │
│  └────────────────────────────────────────────────────┘  │
│                          ↓                                │
│  ┌────────────────────────────────────────────────────┐  │
│  │  TaskSupervisor                                    │  │
│  │  - HTTP запросы к воркерам                         │  │
│  │  - SSE stream обработка                            │  │
│  └────────────────────────────────────────────────────┘  │
│                          ↓                                │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Phoenix.PubSub                                    │  │
│  │  - Стриминг результатов клиентам                   │  │
│  └────────────────────────────────────────────────────┘  │
│                          ↓                                │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Internal API (для воркеров)                       │  │
│  │  POST /internal/workers/register                   │  │
│  │  POST /internal/workers/:id/heartbeat              │  │
│  │  DELETE /internal/workers/:id                      │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          ↓ HTTP
        ┌─────────────────┼─────────────────┐
        ↓                 ↓                 ↓
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│ Whisper Worker│  │ Whisper Worker│  │ Whisper Worker│
│               │  │               │  │               │
│ wrapper.sh    │  │ wrapper.sh    │  │ wrapper.sh    │
│     ↓         │  │     ↓         │  │     ↓         │
│ whisper-server│  │ whisper-server│  │ whisper-server│
│ (C++)         │  │ (C++)         │  │ (C++)         │
│               │  │               │  │               │
│ GET  /health  │  │ GET  /health  │  │ GET  /health  │
│ POST /inference│ │ POST /inference│ │ POST /inference│
│      -stream  │  │      -stream  │  │      -stream  │
└───────────────┘  └───────────────┘  └───────────────┘
```

---

## 📋 Этапы реализации

### Phase 1: Минимальные изменения в whisper.cpp ⚡

**Цель:** Добавить /health endpoint

**Задачи:**
1. Добавить GET /health endpoint в server.cpp
2. Опционально: добавить поддержку job_id в SSE событиях
3. Пересобрать

**Файлы:**
- `examples/server/server.cpp`

**Время:** 30 минут

---

### Phase 2: Wrapper скрипт для воркера 🔧

**Цель:** Автоматическая регистрация и heartbeat

**Задачи:**
1. Создать whisper-worker.sh скрипт
2. Регистрация при старте
3. Heartbeat loop
4. Graceful shutdown

**Файлы:**
- `scripts/whisper-worker.sh`
- `scripts/worker.env.example`

**Время:** 1 час

---

### Phase 3: Phoenix приложение (базовая структура) 🏗️

**Цель:** Создать Phoenix проект

**Задачи:**
1. mix phx.new whisper_orchestrator --no-ecto (без БД пока)
2. Настроить конфигурацию
3. Базовые роуты

**Команды:**
```bash
mix phx.new whisper_orchestrator --no-ecto
cd whisper_orchestrator
mix deps.get
```

**Время:** 30 минут

---

### Phase 4: WorkerRegistry (GenServer) 🗂️

**Цель:** Управление воркерами

**Компоненты:**
```
lib/whisper_orchestrator/
├── workers/
│   ├── registry.ex          # GenServer для хранения списка
│   ├── worker.ex            # Структура воркера
│   └── health_checker.ex    # Периодическая проверка
```

**Функциональность:**
- Регистрация воркеров
- Heartbeat обработка
- Поиск свободного воркера
- Auto-cleanup мёртвых воркеров

**Время:** 2 часа

---

### Phase 5: JobQueue (GenServer) 📦

**Цель:** Очередь задач в памяти

**Компоненты:**
```
lib/whisper_orchestrator/
├── jobs/
│   ├── queue.ex             # GenServer с :queue
│   ├── job.ex               # Структура задачи
│   └── processor.ex         # Обработка задач
```

**Функциональность:**
- Enqueue/dequeue
- Приоритеты (опционально)
- Таймауты
- Status tracking

**Время:** 2 часа

---

### Phase 6: Internal API (для воркеров) 🔌

**Цель:** Endpoints для регистрации

**Роуты:**
```elixir
scope "/internal", WhisperOrchestratorWeb.Internal do
  pipe_through [:api, :require_worker_secret]
  
  post "/workers/register", WorkerController, :register
  post "/workers/:id/heartbeat", WorkerController, :heartbeat
  delete "/workers/:id", WorkerController, :unregister
end
```

**Middleware:**
- Проверка shared secret
- Rate limiting (опционально)

**Время:** 1.5 часа

---

### Phase 7: Public API (для пользователей) 🌐

**Цель:** Endpoints для загрузки аудио

**Роуты:**
```elixir
scope "/api", WhisperOrchestratorWeb.API do
  pipe_through :api
  
  post "/transcribe", TranscribeController, :create
  get "/transcribe/:id", TranscribeController, :show
  get "/transcribe/:id/stream", TranscribeController, :stream
end
```

**Функциональность:**
- Валидация файлов (формат, размер)
- Временное хранилище
- Создание job
- Возврат job_id

**Время:** 2 часа

---

### Phase 8: Job Processor (Task.Supervisor) ⚙️

**Цель:** Отправка задач на воркеры

**Компоненты:**
```
lib/whisper_orchestrator/
├── jobs/
│   └── worker_client.ex     # HTTP клиент для воркеров
```

**Логика:**
1. Взять задачу из очереди
2. Найти свободный воркер
3. Пометить воркер как busy
4. POST /inference-stream на воркер
5. Обработать SSE stream
6. Пометить воркер как idle
7. Опубликовать результат

**Время:** 3 часа

---

### Phase 9: SSE Streaming (Phoenix.PubSub) 📡

**Цель:** Real-time результаты клиентам

**Компоненты:**
- Phoenix.PubSub для pub/sub
- SSE endpoint для клиентов
- Proxy SSE от воркера к клиенту

**Функциональность:**
- Subscribe на job_id
- Получать сегменты от воркера
- Транслировать клиентам
- Поддержка множественных подписчиков

**Время:** 2 часа

---

### Phase 10: Storage & Cleanup 🗄️

**Цель:** Временные файлы и результаты

**Компоненты:**
```
lib/whisper_orchestrator/
├── storage/
│   ├── temp_storage.ex      # Временные аудио файлы
│   └── result_cache.ex      # Кэш результатов (ETS)
```

**Функциональность:**
- Сохранение загруженных файлов
- TTL для временных файлов (30 минут)
- Cleanup task (раз в час)
- Опционально: S3 для долгосрочного хранения

**Время:** 2 часа

---

### Phase 11: Monitoring & Metrics 📊

**Цель:** Observability

**Компоненты:**
- Telemetry events
- LiveDashboard
- Health endpoint для Phoenix

**Метрики:**
- Количество воркеров (active/busy/idle)
- Размер очереди
- Среднее время обработки
- Error rate
- Latency percentiles

**Время:** 2 часа

---

### Phase 12: Testing & Documentation 🧪

**Цель:** Стабильность и документация

**Задачи:**
1. Unit тесты для GenServers
2. Integration тесты для API
3. Load тесты
4. API документация
5. Deployment guide

**Время:** 4 часа

---

## 🛠️ Детальная реализация компонентов

### 1. Whisper.cpp изменения

**Файл:** `examples/server/server.cpp`

**Добавить:**
```cpp
// После существующих endpoints, перед svr->listen()

// Health check endpoint
svr->Get(sparams.request_path + "/health", [&](const Request &, Response &res){
    json health = {
        {"status", "healthy"},
        {"version", "1.0.0"},
        {"model", params.model},
        {"uptime_seconds", time(nullptr) - server_start_time}
    };
    res.set_content(health.dump(), "application/json");
});

// Добавить в начале main():
time_t server_start_time = time(nullptr);
```

**Опционально в SSE события добавить job_id:**
```cpp
// В callback new_segment_callback:
if (req.has_file("job_id")) {
    std::string job_id = req.get_file_value("job_id").content;
    segment_json["job_id"] = job_id;
}
```

---

### 2. Wrapper скрипт

**Файл:** `scripts/whisper-worker.sh`

```bash
#!/bin/bash
set -e

# === Configuration ===
WORKER_ID="${WORKER_ID:-worker-$(hostname)-$$}"
WORKER_PORT="${WORKER_PORT:-8080}"
WORKER_HOST="${WORKER_HOST:-$(hostname -I | awk '{print $1}')}"
ORCHESTRATOR_URL="${ORCHESTRATOR_URL:-http://localhost:4000}"
SHARED_SECRET="${SHARED_SECRET:?SHARED_SECRET required}"
MODEL="${MODEL:-models/ggml-base.en.bin}"
WHISPER_BIN="${WHISPER_BIN:-./build/bin/whisper-server}"
HEARTBEAT_INTERVAL="${HEARTBEAT_INTERVAL:-20}"

# === Functions ===
cleanup() {
    echo "Shutting down..."
    if [ ! -z "$WHISPER_PID" ]; then
        kill $WHISPER_PID 2>/dev/null || true
        wait $WHISPER_PID 2>/dev/null || true
    fi
    
    # Unregister from orchestrator
    curl -X DELETE "$ORCHESTRATOR_URL/internal/workers/$WORKER_ID" \
        -H "Authorization: Bearer $SHARED_SECRET" \
        -s > /dev/null || true
    
    echo "Cleanup complete"
    exit 0
}

trap cleanup SIGINT SIGTERM

# === Start whisper.cpp server ===
echo "Starting whisper-server..."
$WHISPER_BIN -m "$MODEL" --port "$WORKER_PORT" &
WHISPER_PID=$!

# Wait for server to be ready
echo "Waiting for server to start..."
for i in {1..30}; do
    if curl -s "http://localhost:$WORKER_PORT/health" > /dev/null 2>&1; then
        echo "Server is ready!"
        break
    fi
    sleep 1
done

if ! kill -0 $WHISPER_PID 2>/dev/null; then
    echo "Failed to start whisper-server"
    exit 1
fi

# === Register with orchestrator ===
echo "Registering with orchestrator at $ORCHESTRATOR_URL..."
REGISTER_RESPONSE=$(curl -X POST "$ORCHESTRATOR_URL/internal/workers/register" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $SHARED_SECRET" \
    -d "{
        \"worker_id\": \"$WORKER_ID\",
        \"url\": \"http://$WORKER_HOST:$WORKER_PORT\",
        \"model\": \"$MODEL\"
    }" \
    -s -w "\n%{http_code}")

HTTP_CODE=$(echo "$REGISTER_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" != "200" ] && [ "$HTTP_CODE" != "201" ]; then
    echo "Failed to register with orchestrator (HTTP $HTTP_CODE)"
    cleanup
fi

echo "Successfully registered as $WORKER_ID"

# === Heartbeat loop ===
echo "Starting heartbeat loop (every ${HEARTBEAT_INTERVAL}s)..."
while kill -0 $WHISPER_PID 2>/dev/null; do
    sleep $HEARTBEAT_INTERVAL
    
    curl -X POST "$ORCHESTRATOR_URL/internal/workers/$WORKER_ID/heartbeat" \
        -H "Authorization: Bearer $SHARED_SECRET" \
        -s > /dev/null 2>&1 || {
            echo "Warning: Heartbeat failed"
        }
done

echo "Whisper server stopped unexpectedly"
cleanup
```

**Файл:** `scripts/worker.env.example`

```bash
# Worker Configuration
WORKER_ID=gpu-worker-1
WORKER_PORT=8081
WORKER_HOST=192.168.1.100

# Orchestrator
ORCHESTRATOR_URL=http://orchestrator.local:4000
SHARED_SECRET=your-secure-secret-key-here

# Whisper Configuration
MODEL=models/ggml-large-v3-turbo.bin
WHISPER_BIN=./build/bin/whisper-server
HEARTBEAT_INTERVAL=20
```

---

### 3. Phoenix WorkerRegistry

**Файл:** `lib/whisper_orchestrator/workers/worker.ex`

```elixir
defmodule WhisperOrchestrator.Workers.Worker do
  @moduledoc """
  Структура воркера
  """
  
  defstruct [
    :id,
    :url,
    :model,
    :status,           # :idle | :busy
    :last_heartbeat,
    :registered_at,
    :current_job_id
  ]
  
  @type status :: :idle | :busy
  
  @type t :: %__MODULE__{
    id: String.t(),
    url: String.t(),
    model: String.t(),
    status: status(),
    last_heartbeat: DateTime.t(),
    registered_at: DateTime.t(),
    current_job_id: String.t() | nil
  }
end
```

**Файл:** `lib/whisper_orchestrator/workers/registry.ex`

```elixir
defmodule WhisperOrchestrator.Workers.Registry do
  @moduledoc """
  GenServer для управления воркерами
  """
  use GenServer
  require Logger
  
  alias WhisperOrchestrator.Workers.Worker
  
  @heartbeat_timeout_seconds 90
  
  # Client API
  
  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end
  
  def register_worker(id, url, model) do
    GenServer.call(__MODULE__, {:register, id, url, model})
  end
  
  def heartbeat(worker_id) do
    GenServer.cast(__MODULE__, {:heartbeat, worker_id})
  end
  
  def get_idle_worker(model \\ nil) do
    GenServer.call(__MODULE__, {:get_idle, model})
  end
  
  def mark_busy(worker_id, job_id) do
    GenServer.cast(__MODULE__, {:mark_busy, worker_id, job_id})
  end
  
  def mark_idle(worker_id) do
    GenServer.cast(__MODULE__, {:mark_idle, worker_id})
  end
  
  def list_workers do
    GenServer.call(__MODULE__, :list_workers)
  end
  
  def unregister_worker(worker_id) do
    GenServer.cast(__MODULE__, {:unregister, worker_id})
  end
  
  # Server Callbacks
  
  def init(_opts) do
    schedule_health_check()
    {:ok, %{workers: %{}}}
  end
  
  def handle_call({:register, id, url, model}, _from, state) do
    worker = %Worker{
      id: id,
      url: url,
      model: model,
      status: :idle,
      last_heartbeat: DateTime.utc_now(),
      registered_at: DateTime.utc_now(),
      current_job_id: nil
    }
    
    new_state = put_in(state.workers[id], worker)
    Logger.info("Worker registered: #{id} at #{url} (model: #{model})")
    
    {:reply, {:ok, worker}, new_state}
  end
  
  def handle_call({:get_idle, preferred_model}, _from, state) do
    worker = 
      state.workers
      |> Enum.filter(fn {_id, w} -> w.status == :idle end)
      |> Enum.filter(fn {_id, w} -> 
        is_nil(preferred_model) or w.model =~ preferred_model
      end)
      |> Enum.map(fn {_id, w} -> w end)
      |> Enum.random()
    
    {:reply, worker, state}
  rescue
    _ -> {:reply, nil, state}
  end
  
  def handle_call(:list_workers, _from, state) do
    workers = Map.values(state.workers)
    {:reply, workers, state}
  end
  
  def handle_cast({:heartbeat, worker_id}, state) do
    case Map.get(state.workers, worker_id) do
      nil ->
        {:noreply, state}
      
      worker ->
        updated_worker = %{worker | last_heartbeat: DateTime.utc_now()}
        new_state = put_in(state.workers[worker_id], updated_worker)
        {:noreply, new_state}
    end
  end
  
  def handle_cast({:mark_busy, worker_id, job_id}, state) do
    case Map.get(state.workers, worker_id) do
      nil ->
        {:noreply, state}
      
      worker ->
        updated_worker = %{worker | status: :busy, current_job_id: job_id}
        new_state = put_in(state.workers[worker_id], updated_worker)
        Logger.debug("Worker #{worker_id} marked as busy (job: #{job_id})")
        {:noreply, new_state}
    end
  end
  
  def handle_cast({:mark_idle, worker_id}, state) do
    case Map.get(state.workers, worker_id) do
      nil ->
        {:noreply, state}
      
      worker ->
        updated_worker = %{worker | status: :idle, current_job_id: nil}
        new_state = put_in(state.workers[worker_id], updated_worker)
        Logger.debug("Worker #{worker_id} marked as idle")
        {:noreply, new_state}
    end
  end
  
  def handle_cast({:unregister, worker_id}, state) do
    new_state = %{state | workers: Map.delete(state.workers, worker_id)}
    Logger.info("Worker unregistered: #{worker_id}")
    {:noreply, new_state}
  end
  
  def handle_info(:health_check, state) do
    now = DateTime.utc_now()
    
    {alive, dead} = 
      Enum.split_with(state.workers, fn {_id, worker} ->
        DateTime.diff(now, worker.last_heartbeat, :second) < @heartbeat_timeout_seconds
      end)
    
    if length(dead) > 0 do
      dead_ids = Enum.map(dead, fn {id, _} -> id end)
      Logger.warn("Removing dead workers: #{inspect(dead_ids)}")
    end
    
    new_state = %{state | workers: Map.new(alive)}
    schedule_health_check()
    
    {:noreply, new_state}
  end
  
  defp schedule_health_check do
    Process.send_after(self(), :health_check, 30_000)  # каждые 30 сек
  end
end
```

---

### 4. Phoenix JobQueue

**Файл:** `lib/whisper_orchestrator/jobs/job.ex`

```elixir
defmodule WhisperOrchestrator.Jobs.Job do
  defstruct [
    :id,
    :audio_path,
    :status,       # :queued | :processing | :completed | :failed
    :result,
    :error,
    :created_at,
    :started_at,
    :completed_at,
    :worker_id,
    :options
  ]
  
  @type t :: %__MODULE__{
    id: String.t(),
    audio_path: String.t(),
    status: atom(),
    result: map() | nil,
    error: String.t() | nil,
    created_at: DateTime.t(),
    started_at: DateTime.t() | nil,
    completed_at: DateTime.t() | nil,
    worker_id: String.t() | nil,
    options: map()
  }
end
```

**Файл:** `lib/whisper_orchestrator/jobs/queue.ex`

```elixir
defmodule WhisperOrchestrator.Jobs.Queue do
  @moduledoc """
  In-memory job queue используя Erlang :queue
  """
  use GenServer
  require Logger
  
  alias WhisperOrchestrator.Jobs.Job
  
  # Client API
  
  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end
  
  def enqueue(job) do
    GenServer.call(__MODULE__, {:enqueue, job})
  end
  
  def dequeue do
    GenServer.call(__MODULE__, :dequeue)
  end
  
  def get_job(job_id) do
    GenServer.call(__MODULE__, {:get, job_id})
  end
  
  def update_job(job_id, updates) do
    GenServer.cast(__MODULE__, {:update, job_id, updates})
  end
  
  def queue_size do
    GenServer.call(__MODULE__, :size)
  end
  
  # Server Callbacks
  
  def init(_opts) do
    state = %{
      queue: :queue.new(),
      jobs: %{}  # job_id => Job
    }
    {:ok, state}
  end
  
  def handle_call({:enqueue, job}, _from, state) do
    new_queue = :queue.in(job.id, state.queue)
    new_jobs = Map.put(state.jobs, job.id, job)
    
    new_state = %{state | queue: new_queue, jobs: new_jobs}
    Logger.info("Job enqueued: #{job.id}")
    
    {:reply, {:ok, job}, new_state}
  end
  
  def handle_call(:dequeue, _from, state) do
    case :queue.out(state.queue) do
      {{:value, job_id}, new_queue} ->
        job = Map.get(state.jobs, job_id)
        new_state = %{state | queue: new_queue}
        {:reply, {:ok, job}, new_state}
      
      {:empty, _queue} ->
        {:reply, {:error, :empty}, state}
    end
  end
  
  def handle_call({:get, job_id}, _from, state) do
    job = Map.get(state.jobs, job_id)
    {:reply, job, state}
  end
  
  def handle_call(:size, _from, state) do
    size = :queue.len(state.queue)
    {:reply, size, state}
  end
  
  def handle_cast({:update, job_id, updates}, state) do
    case Map.get(state.jobs, job_id) do
      nil ->
        {:noreply, state}
      
      job ->
        updated_job = struct(job, updates)
        new_jobs = Map.put(state.jobs, job_id, updated_job)
        {:noreply, %{state | jobs: new_jobs}}
    end
  end
end
```

---

## 🚀 Deployment

### Development

```bash
# Terminal 1: Start Phoenix
cd whisper_orchestrator
export WORKER_SHARED_SECRET=dev-secret
mix phx.server

# Terminal 2: Start Worker 1
cd whisper.cpp
export WORKER_ID=dev-worker-1
export WORKER_PORT=8081
export SHARED_SECRET=dev-secret
export MODEL=models/ggml-base.en.bin
./scripts/whisper-worker.sh

# Terminal 3: Test
curl -X POST http://localhost:4000/api/transcribe \
  -F "audio=@samples/jfk.wav"
```

### Production

```bash
# Server 1: Orchestrator
docker run -p 4000:4000 \
  -e SECRET_KEY_BASE=xxx \
  -e WORKER_SHARED_SECRET=xxx \
  whisper-orchestrator:latest

# Server 2-N: Workers
docker run --gpus all \
  -e WORKER_ID=gpu-1 \
  -e ORCHESTRATOR_URL=http://orchestrator:4000 \
  -e SHARED_SECRET=xxx \
  -e MODEL=models/ggml-large-v3-turbo.bin \
  whisper-worker:latest
```

---

## 📊 Timeline

**Total: ~25 часов работы**

| Phase | Время | Приоритет |
|-------|-------|-----------|
| Phase 1: whisper.cpp | 0.5h | P0 |
| Phase 2: Wrapper | 1h | P0 |
| Phase 3: Phoenix setup | 0.5h | P0 |
| Phase 4: WorkerRegistry | 2h | P0 |
| Phase 5: JobQueue | 2h | P0 |
| Phase 6: Internal API | 1.5h | P0 |
| Phase 7: Public API | 2h | P0 |
| Phase 8: Job Processor | 3h | P0 |
| Phase 9: SSE Streaming | 2h | P1 |
| Phase 10: Storage | 2h | P1 |
| Phase 11: Monitoring | 2h | P2 |
| Phase 12: Testing | 4h | P1 |

---

## ✅ Success Criteria

- [ ] Воркер может зарегистрироваться автоматически
- [ ] Heartbeat работает и детектит мёртвые воркеры
- [ ] Задачи обрабатываются на свободных воркерах
- [ ] SSE streaming работает от воркера до клиента
- [ ] Load balancing между воркерами
- [ ] Graceful shutdown воркеров
- [ ] Временные файлы очищаются
- [ ] Monitoring показывает метрики
- [ ] Можно запустить N воркеров без конфигурации

---

## 🔮 Future Improvements

### V2 Features:
- [ ] Персистентная очередь (опционально Redis для HA)
- [ ] Приоритеты задач (VIP пользователи)
- [ ] S3 storage для длительного хранения
- [ ] Webhook callbacks при завершении
- [ ] Multi-region support
- [ ] Auto-scaling воркеров (K8s)
- [ ] Model marketplace (разные модели на разных воркерах)
- [ ] Cost tracking и billing
- [ ] Admin UI (Phoenix LiveView)
- [ ] API rate limiting per user
- [ ] Batch processing mode

### Advanced:
- [ ] Distributed Elixir (multi-node orchestrator)
- [ ] Worker affinity (sticky sessions)
- [ ] Smart routing (длинные файлы на мощные воркеры)
- [ ] Pre/post processing pipelines
- [ ] Audio preprocessing (noise reduction, normalization)
- [ ] Multi-language results (перевод)

---

## 📚 References

- [Phoenix Framework](https://www.phoenixframework.org/)
- [GenServer Guide](https://hexdocs.pm/elixir/GenServer.html)
- [Phoenix PubSub](https://hexdocs.pm/phoenix_pubsub/Phoenix.PubSub.html)
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp)
- [Server-Sent Events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events)

