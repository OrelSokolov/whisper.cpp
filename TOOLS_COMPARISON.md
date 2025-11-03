# Инструменты для проверки системы

## 📊 Сравнение инструментов

| Инструмент | Когда использовать | Что проверяет | Зависит от сборки |
|------------|-------------------|---------------|-------------------|
| `detect_best_backend.sh` | **ДО сборки** | Железо напрямую | ❌ НЕТ |
| `whisper-system` | **ПОСЛЕ сборки** | Скомпилированные бэкенды | ✅ ДА |
| `vulkaninfo` | Отладка Vulkan | Vulkan API детали | ❌ НЕТ |
| `nvidia-smi` | Мониторинг NVIDIA | CUDA GPU статус | ❌ НЕТ |
| `whisper-bench` | Бенчмарки | Реальная скорость | ✅ ДА |

## 🔄 Правильный workflow

### Вариант 1: Первая установка

```bash
# 1. Проверить что доступно на железе
./detect_best_backend.sh

# Вывод покажет рекомендованную команду, например:
#   cmake -B build -DGGML_VULKAN=1 ...

# 2. Собрать с рекомендованными флагами
cmake -B build -DGGML_VULKAN=1 -DCMAKE_BUILD_TYPE=Release
cmake --build build -j$(nproc)

# 3. Проверить что получилось
./build/bin/whisper-system

# 4. (Опционально) Запустить бенчмарк
./build/bin/whisper-bench -m models/ggml-base.en.bin
```

### Вариант 2: Уже собрано, не работает

```bash
# 1. Проверить что скомпилировано
./build/bin/whisper-system

# 2. Проверить что должно быть
./detect_best_backend.sh

# 3. Сравнить - если не совпадает, пересобрать
```

### Вариант 3: Оптимизация производительности

```bash
# 1. Проверить текущую конфигурацию
./build/bin/whisper-system -m models/ggml-base.en.bin

# 2. Запустить бенчмарк
./build/bin/whisper-bench -m models/ggml-base.en.bin

# 3. Проверить возможности железа
./detect_best_backend.sh
vulkaninfo | grep -i cooperative  # для Vulkan

# 4. Пересобрать с оптимизациями если нужно
```

## 🎯 Детали инструментов

### `detect_best_backend.sh`

**Использует системные утилиты:**
- `nvidia-smi` → проверка NVIDIA GPU
- `vulkaninfo` → проверка Vulkan + возможности (CoopMat, IntDot)
- `clinfo` → проверка OpenCL
- `lscpu` → проверка CPU (AVX2, FMA, etc.)
- `system_profiler` → проверка Metal (macOS)

**Выдаёт:**
- ✅ Найденное железо
- ✅ Рекомендованные флаги cmake
- ✅ Подходящие модели
- ✅ Следующие шаги

**Не требует:**
- Предварительной сборки whisper.cpp
- Моделей
- Каких-либо зависимостей кроме системных

### `whisper-system`

**Проверяет скомпилированный бинарник:**
- `ggml_backend_reg_count()` → список бэкендов в сборке
- `ggml_backend_dev_count()` → устройства для каждого бэкенда
- `whisper_print_system_info()` → CPU features
- Опционально: загрузка модели для теста

**Выдаёт:**
- Backend Registry (что скомпилировано)
- Available Devices (что инициализировалось)
- Optimization Recommendations (на основе скомпилированного)

**Требует:**
- Собранный whisper.cpp
- (Опционально) модель для теста с `-m`

## 💡 Часто задаваемые вопросы

**Q: Почему `whisper-system` не показывает мой Vulkan GPU?**

A: Потому что вы собрали **без** `-DGGML_VULKAN=1`. Сначала запустите:
```bash
./detect_best_backend.sh  # узнать что нужно
# пересоберите с нужными флагами
./build/bin/whisper-system  # теперь покажет
```

**Q: `detect_best_backend.sh` показывает CUDA, но я хочу Vulkan**

A: Можете собрать с любым бэкендом:
```bash
# CUDA (быстрее на NVIDIA)
cmake -B build -DGGML_CUDA=1

# Vulkan (кроссплатформенный)
cmake -B build -DGGML_VULKAN=1

# Оба сразу
cmake -B build -DGGML_CUDA=1 -DGGML_VULKAN=1
```

**Q: Можно ли собрать ВСЕ бэкенды сразу?**

A: Да! Но это усложняет отладку:
```bash
cmake -B build \
  -DGGML_CUDA=1 \
  -DGGML_VULKAN=1 \
  -DGGML_OPENCL=1 \
  -DGGML_BLAS=ON
```

whisper.cpp будет выбирать лучший автоматически.

**Q: Какой бэкенд самый быстрый?**

A: Зависит от железа:
1. **NVIDIA GPU**: CUDA > Vulkan > OpenCL
2. **AMD GPU**: Vulkan > OpenCL
3. **Intel GPU**: Vulkan ≈ SYCL > OpenCL
4. **Apple Silicon**: Metal > все остальные
5. **CPU**: BLAS > plain CPU

Используйте `whisper-bench` для точного измерения.

## 🔗 См. также

- [SYSTEM_INFO.md](SYSTEM_INFO.md) - итоговая инструкция
- [VULKAN_OPTIMIZATION_GUIDE.md](VULKAN_OPTIMIZATION_GUIDE.md) - Vulkan гайд
- [examples/system-info/README.md](examples/system-info/README.md) - whisper-system документация

