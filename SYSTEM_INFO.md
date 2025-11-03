# ✅ Создана утилита whisper-system для проверки возможностей системы

## 🎯 Что было сделано

### 1. Создана новая утилита `whisper-system`

**Расположение:** `examples/system-info/`

**Возможности:**
- ✅ Показывает все доступные бэкенды (CPU, Vulkan, CUDA, Metal, etc.)
- ✅ Детализирует каждое устройство (GPU/CPU) с памятью
- ✅ Показывает возможности: Cooperative Matrix, Integer Dot Product, FP16, etc.
- ✅ Даёт конкретные рекомендации по оптимизации для вашего железа
- ✅ Может протестировать backend с реальной моделью

### 2. Скрипты для быстрого старта

- `build_and_check_system.sh` - пересобрать и проверить систему
- `VULKAN_OPTIMIZATION_GUIDE.md` - полное руководство по оптимизации

## 🚀 Как использовать

### ⚠️ ВАЖНО: Два инструмента - два этапа!

**1️⃣ ДО сборки - определить оптимальный backend:**
```bash
./detect_best_backend.sh
# Проверяет систему напрямую (nvidia-smi, vulkaninfo, etc.)
# Рекомендует флаги для сборки
```

**2️⃣ ПОСЛЕ сборки - проверить что работает:**
```bash
./build/bin/whisper-system
# Показывает скомпилированные бэкенды и устройства
# Тестирует инициализацию
```

### Правильный workflow:

```bash
# Шаг 1: Узнать что поддерживает железо
./detect_best_backend.sh

# Шаг 2: Собрать с рекомендованными флагами
# (команда будет показана в выводе detect_best_backend.sh)
cmake -B build -DGGML_VULKAN=1 ...
cmake --build build -j$(nproc)

# Шаг 3: Проверить результат
./build/bin/whisper-system
```

### Что показывает для вашей системы

Из вывода видно:
```
✅ AMD Radeon Graphics (RADV REMBRANDT) - Integrated GPU
✅ Vulkan поддерживается
✅ FP16 поддерживается
❌ Cooperative Matrix: нет (matrix cores: none)
❌ Integer Dot Product: нет (int dot: 0)
✅ CPU: AMD Ryzen 7 6800H с AVX2, FMA, F16C
```

### Рекомендации для ВАШЕЙ системы

#### 1. Сборка с Vulkan (уже сделано!)

```bash
cmake -B build -DGGML_VULKAN=1 \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_CXX_FLAGS="-O3 -march=native -mtune=native"

cmake --build build -j$(nproc)
```

#### 2. Лучшие модели для вашего GPU

Поскольку у вас **integrated GPU без Cooperative Matrix**, рекомендую:

```bash
# Самая быстрая модель для integrated GPU
./models/download-ggml-model.sh large-v3-turbo-q5_0

# Запустить worker
./build/bin/whisper-worker -m models/ggml-large-v3-turbo-q5_0.bin
```

**Почему именно эта модель?**
- Квантизация Q5_0 снижает нагрузку на память
- Turbo версия быстрее чем large-v3
- Integrated GPU выиграет от меньшего размера

#### 3. Как проверить что Vulkan работает

```bash
# Способ 1: Через whisper-system с моделью
./build/bin/whisper-system -m models/ggml-base.en.bin

# Способ 2: Через GGML_VK_LOG_LEVEL
export GGML_VK_LOG_LEVEL=DEBUG
./build/bin/whisper-worker -m models/ggml-base.en.bin

# В логах ищите:
# "ggml_vulkan: Found 1 Vulkan devices"
# "AMD Radeon Graphics"
# "using Vulkan backend"
```

## 📊 Как увидеть использование CoopMat (для вашей системы)

Для вашей системы Cooperative Matrix **не поддерживается**, т.к. это AMD integrated GPU.

Но вы можете проверить на других системах:

```bash
# 1. Проверить через whisper-system
./build/bin/whisper-system

# Смотрите строку: "matrix cores: none" или "coopmat"

# 2. Проверить через vulkaninfo
vulkaninfo | grep -i cooperativeMatrix

# Если пусто или false - не поддерживается
# Если есть строки с true - поддерживается

# 3. Проверить в worker логах
export GGML_VK_LOG_LEVEL=DEBUG
./build/bin/whisper-worker -m models/ggml-base.en.bin
# Ищите: "coopmat_support: true/false"
```

## 🎮 Что делать дальше

### Для максимальной производительности на вашей системе:

1. **Используйте квантизованные модели** (`q5_0`, `q8_0`)
2. **Включите Vulkan** (уже включен в сборке)
3. **Попробуйте разные модели:**

```bash
# Для real-time (самое быстрое)
./models/download-ggml-model.sh large-v3-turbo-q5_0

# Для баланса скорость/качество
./models/download-ggml-model.sh base.en

# Для максимального качества (медленнее)
./models/download-ggml-model.sh large-v3-q5_0
```

4. **Настройте переменные окружения:**

```bash
# Для production (быстрее)
export VK_INSTANCE_LAYERS=""

# Для отладки (подробные логи)
export GGML_VK_LOG_LEVEL=DEBUG

# Для UMA систем (как ваша integrated GPU)
export GGML_VK_PREFER_HOST_MEMORY=1
```

## 📁 Структура файлов

```
whisper.cpp/
├── examples/system-info/           # НОВАЯ УТИЛИТА
│   ├── system-info.cpp            # Исходный код
│   ├── CMakeLists.txt             # Конфигурация сборки
│   └── README.md                  # Документация
├── build/bin/
│   └── whisper-system             # СКОМПИЛИРОВАННАЯ УТИЛИТА
├── build_and_check_system.sh      # Скрипт для быстрой проверки
└── VULKAN_OPTIMIZATION_GUIDE.md   # Полное руководство
```

## 🔍 Примеры использования

### 1. Просто показать info

```bash
./build/bin/whisper-system
```

### 2. С тестом модели

```bash
./build/bin/whisper-system -m models/ggml-base.en.bin
```

### 3. Помощь

```bash
./build/bin/whisper-system --help
```

## 💡 Важные заметки

1. **Cooperative Matrix** на вашей системе **НЕ поддерживается**
   - Это нормально для AMD integrated GPU
   - Все равно Vulkan даст ускорение vs CPU
   
2. **Integer Dot Product** тоже **НЕ поддерживается**
   - Но квантизованные модели все равно быстрее
   
3. **FP16 поддерживается!**
   - Это хорошо, используйте fp16 модели

4. **Ваш CPU очень мощный** (Ryzen 7 6800H с AVX2)
   - Для небольших моделей CPU может быть быстрее GPU
   - Попробуйте оба варианта и сравните

## 🎯 Проверка производительности

```bash
# Запустить бенчмарк
./build/bin/whisper-bench -m models/ggml-base.en.bin

# С GPU
./build/bin/whisper-bench -m models/ggml-base.en.bin

# Без GPU (для сравнения)
./build/bin/whisper-bench -m models/ggml-base.en.bin --no-gpu
```

## 📚 Дополнительная информация

- [VULKAN_OPTIMIZATION_GUIDE.md](VULKAN_OPTIMIZATION_GUIDE.md) - полное руководство
- [examples/system-info/README.md](examples/system-info/README.md) - документация утилиты
- [README.md](README.md) - основная документация whisper.cpp

---

**Утилита готова к использованию! 🎉**

