# Vulkan Optimization Guide для Whisper.cpp

## 🎯 Быстрый старт

### 1. Проверить возможности вашей системы

```bash
# Собрать и запустить утилиту проверки системы
./build_and_check_system.sh
```

Эта утилита покажет:
- ✅ Какие GPU доступны
- ✅ Поддерживается ли Vulkan, CUDA, Metal
- ✅ Объем памяти GPU
- ✅ Cooperative Matrix support (для максимальной скорости)
- ✅ Рекомендации по оптимизации

### 2. Собрать с оптимизациями Vulkan

```bash
cmake -B build \
  -DGGML_VULKAN=1 \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_CXX_FLAGS="-O3 -march=native -mtune=native" \
  -DCMAKE_C_FLAGS="-O3 -march=native -mtune=native"

cmake --build build -j$(nproc)
```

### 3. Использовать квантизованные модели

```bash
# Скачать оптимизированную модель (в 3 раза быстрее!)
./models/download-ggml-model.sh large-v3-turbo-q5_0

# Запустить worker
./build/bin/whisper-worker -m models/ggml-large-v3-turbo-q5_0.bin
```

## 🚀 Как проверить что CoopMat используется

### Способ 1: Через whisper-system

```bash
# Показывает ВСЕ возможности системы
./build/bin/whisper-system

# Проверить с загрузкой модели
./build/bin/whisper-system -m models/ggml-base.en.bin
```

Ищите в выводе:
```
Backend #X: Vulkan
  cooperativeMatrix         : true
  integerDotProduct         : true
```

### Способ 2: Через vulkaninfo

```bash
# Проверить поддержку Cooperative Matrix
vulkaninfo | grep -i cooperativeMatrix

# Если видите строки с coordinateMatrix* = VK_TRUE
# значит ваш GPU поддерживает эту фичу!
```

### Способ 3: Через GGML_VK_LOG_LEVEL

```bash
# Включить подробное логирование Vulkan
export GGML_VK_LOG_LEVEL=DEBUG

# Запустить worker
./build/bin/whisper-worker -m models/ggml-base.en.bin

# В логах ищите:
# "ggml_vulkan: Cooperative Matrix Shapes: X"
# "coopmat_support: true"
```

## 📊 Новая утилита: whisper-system

### Что она делает?

- 🔍 Показывает все доступные бэкенды (CPU, Vulkan, CUDA, Metal, etc.)
- 💾 Отображает GPU устройства и их память
- ⚙️ Детализирует возможности каждого бэкенда
- 💡 Дает конкретные рекомендации по оптимизации
- 🧪 Может протестировать backend с реальной моделью

### Использование

```bash
# Просто показать инфо
./build/bin/whisper-system

# С тестом загрузки модели
./build/bin/whisper-system -m models/ggml-base.en.bin

# Помощь
./build/bin/whisper-system --help
```

### Пример вывода

```
╔══════════════════════════════════════════════════════════════╗
║               WHISPER.CPP SYSTEM CAPABILITIES                ║
╚══════════════════════════════════════════════════════════════╝

╔══════════════════════════════════════════════════════════════╗
║ AVAILABLE DEVICES                                            ║
╚══════════════════════════════════════════════════════════════╝

  Vulkan Backend:
  ════════════════════════════════════════════════════════════
    Device #0:
      Name                      : NVIDIA GeForce RTX 3080
      Type                      : GPU (Discrete)
      Total Memory              : 10.00 GB
      Free Memory               : 9.50 GB

╔══════════════════════════════════════════════════════════════╗
║ OPTIMIZATION RECOMMENDATIONS                                 ║
╚══════════════════════════════════════════════════════════════╝

  🎮 Vulkan support detected!
     Rebuild with optimizations:
       cmake -B build -DGGML_VULKAN=1 \
         -DCMAKE_CXX_FLAGS="-O3 -march=native"
```

## 🔧 Переменные окружения для оптимизации

```bash
# Подробное логирование Vulkan (для отладки)
export GGML_VK_LOG_LEVEL=DEBUG

# Отключить валидацию (для production - быстрее)
export VK_INSTANCE_LAYERS=""

# Форсировать MMVQ (для некоторых GPU)
export GGML_VK_FORCE_MMVQ=1

# Отключить MMVQ
export GGML_VK_DISABLE_MMVQ=1

# Отключить оптимизацию графа (для отладки)
export GGML_VK_DISABLE_GRAPH_OPTIMIZE=1

# Использовать host memory (UMA системы)
export GGML_VK_PREFER_HOST_MEMORY=1
```

## 📦 Рекомендации по моделям

| Модель | Размер | Скорость | Качество | Использование |
|--------|--------|----------|----------|---------------|
| `large-v3-turbo-q5_0` | 547 MB | ⚡⚡⚡ | ⭐⭐⭐⭐ | Real-time, лучший выбор |
| `large-v3-q5_0` | 1.1 GB | ⚡⚡ | ⭐⭐⭐⭐⭐ | Максимальное качество |
| `large-v3-turbo` | 1.5 GB | ⚡⚡ | ⭐⭐⭐⭐⭐ | Без квантизации |
| `base.en` | 142 MB | ⚡⚡⚡ | ⭐⭐⭐ | CPU, английский |
| `small.en` | 466 MB | ⚡⚡ | ⭐⭐⭐⭐ | Баланс CPU |

Квантизованные модели (`q5_0`, `q8_0`):
- ✅ В 3-5 раз быстрее
- ✅ Занимают меньше памяти
- ✅ Почти не теряют в качестве
- ✅ Используют Integer Dot Product на GPU

## 🎯 Максимальная производительность

### Для NVIDIA GPU (если доступно)

```bash
cmake -B build -DGGML_CUDA=1 -DCMAKE_CUDA_ARCHITECTURES="86"
cmake --build build -j$(nproc)
```

### Для любого Vulkan GPU

```bash
# Сборка
cmake -B build -DGGML_VULKAN=1 \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_CXX_FLAGS="-O3 -march=native"
cmake --build build -j$(nproc)

# Запуск
export GGML_VK_LOG_LEVEL=INFO  # или DEBUG для отладки
./build/bin/whisper-worker -m models/ggml-large-v3-turbo-q5_0.bin
```

### Для Apple Silicon (Metal)

```bash
cmake -B build -DGGML_METAL=1
cmake --build build -j
```

## 🔬 Troubleshooting

### GPU не используется

1. Проверьте доступность:
   ```bash
   ./build/bin/whisper-system
   ```

2. Проверьте Vulkan драйверы:
   ```bash
   vulkaninfo --summary
   ```

3. Включите отладку:
   ```bash
   export GGML_VK_LOG_LEVEL=DEBUG
   export VK_LOADER_DEBUG=all
   ```

### Медленная работа

1. Используйте квантизованные модели (`q5_0`)
2. Проверьте что GPU действительно используется (см. выше)
3. Убедитесь что собрали с оптимизациями (`-O3 -march=native`)
4. Попробуйте разные переменные окружения (см. выше)

## 📚 Дополнительная информация

- [Основной README](README.md)
- [System Info утилита](examples/system-info/README.md)
- [Worker документация](examples/worker/README.md)

## 🤝 Вклад

Если нашли баг или хотите улучшить производительность:
- https://github.com/ggerganov/whisper.cpp/issues

