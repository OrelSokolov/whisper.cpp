# Оптимизация параллелизации Encoder в Vulkan

## Текущая реализация

### Архитектура Encoder
- **Последовательные слои**: Encoder состоит из `n_layer` (обычно 6-32) последовательных transformer слоев
- **Зависимости**: Каждый слой зависит от предыдущего через residual connections
- **Структура слоя**:
  1. Layer Norm
  2. Self-Attention (Q, K, V вычисления → Attention → Projection)
  3. Residual connection
  4. Feed-Forward Network (MLP)
  5. Residual connection

### Текущее выполнение
```cpp
// src/whisper.cpp:2109
for (int il = 0; il < n_layer; ++il) {
    // Каждый слой выполняется последовательно
    // Граф строится полностью, затем выполняется через scheduler
}
```

Граф выполняется через `ggml_backend_sched_graph_compute()`, который:
- Определяет зависимости между операциями
- Выполняет операции в правильном порядке
- Использует один command buffer для всего графа

---

## Возможности для улучшения параллелизации

### 1. ✅ Параллельное вычисление Q, K, V внутри слоя

**Текущая реализация:**
```cpp
// Последовательно:
Qcur = ggml_mul_mat(ctx0, layer.attn_q_w, cur);
Kcur = ggml_mul_mat(ctx0, layer.attn_k_w, cur);
Vcur = ggml_mul_mat(ctx0, layer.attn_v_w, cur);
```

**Оптимизация:**
Q, K, V могут вычисляться параллельно, так как они независимы:
- Все используют один и тот же вход `cur`
- Все используют разные веса, но одинаковую операцию `mul_mat`
- Нет зависимостей между ними

**Реализация:**
```cpp
// В whisper_build_graph_encoder():
// Создать Q, K, V операции как независимые узлы графа
// Scheduler автоматически распараллелит их, если они в разных command buffers

// Можно явно группировать операции:
ggml_tensor * Qcur = ggml_mul_mat(ctx0, layer.attn_q_w, cur);
ggml_tensor * Kcur = ggml_mul_mat(ctx0, layer.attn_k_w, cur);
ggml_tensor * Vcur = ggml_mul_mat(ctx0, layer.attn_v_w, cur);

// Добавить их в граф одновременно
ggml_build_forward_expand(gf, Qcur);
ggml_build_forward_expand(gf, Kcur);
ggml_build_forward_expand(gf, Vcur);
```

**Ожидаемый эффект:** Ускорение ~1.5-2x для attention части каждого слоя

---

### 2. ✅ Использование Async Compute Queues в Vulkan

**Проблема:** Текущая реализация использует один compute queue

**Решение:** Использовать несколько compute queues для параллельного выполнения независимых операций

**Текущая архитектура Vulkan:**
- Один compute queue для всех операций
- Операции выполняются последовательно в command buffer

**Оптимизация:**
```cpp
// В ggml-vulkan.cpp:
// 1. Создать несколько compute queues
// 2. Разделить операции между очередями

// Структура для управления несколькими очередями:
struct vk_multi_queue_context {
    std::vector<vk::Queue> compute_queues;
    std::vector<vk::CommandBuffer> command_buffers;
    std::vector<vk::Fence> fences;
    int current_queue = 0;
};

// Распределение операций:
// Queue 0: Q вычисления
// Queue 1: K вычисления  
// Queue 2: V вычисления
// Затем синхронизация через barriers
```

**Реализация в ggml_backend_vk:**
```cpp
// Модифицировать ggml_vk_graph_compute() для использования нескольких очередей
// Разделить граф на независимые части
// Выполнить каждую часть в отдельной очереди
```

**Ожидаемый эффект:** Ускорение ~2-3x при наличии нескольких compute units в GPU

---

### 3. ✅ Pipeline Barriers и Memory Barriers оптимизация

**Проблема:** Избыточные синхронизации между операциями

**Текущее поведение:**
- Каждая операция ждет завершения предыдущей
- Memory barriers вставляются между всеми операциями

**Оптимизация:**
```cpp
// Использовать более гранулярные barriers:
// - Execution barrier только когда нужно
// - Memory barrier только для реальных зависимостей

// Пример для Q, K, V:
vk::MemoryBarrier barrier;
barrier.srcAccessMask = vk::AccessFlagBits::eShaderWrite;
barrier.dstAccessMask = vk::AccessFlagBits::eShaderRead;

// Использовать memory barrier только между Q/K/V и attention, не между Q, K, V
```

**Реализация:**
```cpp
// В ggml_vk_dispatch_tensor() добавить анализ зависимостей
// Вставлять barriers только когда реально нужно

static void ggml_vk_smart_barrier(vk_context& subctx, 
                                   const ggml_tensor* src, 
                                   const ggml_tensor* dst) {
    // Анализировать зависимости
    // Вставлять минимально необходимые barriers
}
```

**Ожидаемый эффект:** Уменьшение overhead синхронизации на 20-30%

---

### 4. ✅ Batch Processing матричных операций

**Проблема:** Каждая матричная операция отправляется отдельно

**Оптимизация:** Группировать несколько операций в один dispatch

**Текущее:**
```cpp
// Каждая mul_mat отправляется отдельно
Qcur = ggml_mul_mat(ctx0, layer.attn_q_w, cur);
Kcur = ggml_mul_mat(ctx0, layer.attn_k_w, cur);
Vcur = ggml_mul_mat(ctx0, layer.attn_v_w, cur);
```

**Оптимизация:**
```cpp
// Создать batch операцию для Q, K, V одновременно
// Использовать один kernel с несколькими invocation groups

// Новый оператор:
ggml_tensor * QKV = ggml_mul_mat_batch(ctx0, 
    {layer.attn_q_w, layer.attn_k_w, layer.attn_v_w}, 
    cur);
```

**Реализация в Vulkan:**
```cpp
// В ggml_vk_matmul():
// Если несколько операций с одинаковыми размерами и типами,
// группировать их в один dispatch с разными binding slots

void ggml_vk_matmul_batch(vk_context& subctx,
                          const std::vector<ggml_tensor*>& weights,
                          const ggml_tensor* input,
                          std::vector<ggml_tensor*>& outputs) {
    // Один command buffer
    // Несколько binding sets
    // Один dispatch с несколькими группами
}
```

**Ожидаемый эффект:** Ускорение ~1.3-1.5x за счет уменьшения overhead dispatch

---

### 5. ✅ Оптимизация Self-Attention вычислений

**Текущая реализация:**
1. Q = W_q * input
2. K = W_k * input  
3. V = W_v * input
4. KQ = K^T * Q
5. Softmax(KQ)
6. KQV = V * Softmax(KQ)

**Оптимизация:** Использовать Flash Attention или оптимизированный kernel

**Flash Attention уже есть:**
```cpp
if (wctx.params.flash_attn) {
    cur = ggml_flash_attn_ext(ctx0, Q, K, V, nullptr, KQscale, 0.0f, 0.0f);
}
```

**Дополнительная оптимизация:**
- Использовать tile-based computation
- Оптимизировать memory access patterns
- Использовать shared memory эффективнее

**Реализация:**
```cpp
// Модифицировать flash_attn kernel для лучшей параллелизации
// Использовать несколько work groups для больших контекстов
// Оптимизировать для разных размеров (n_ctx, n_head)
```

**Ожидаемый эффект:** Ускорение ~1.5-2x для attention операций

---

### 6. ✅ Pipeline Overlap между слоями

**Проблема:** Слои выполняются строго последовательно

**Ограничение:** Полное распараллеливание слоев невозможно из-за residual connections

**Частичная оптимизация:**
Можно начать вычисления следующего слоя, пока завершаются операции предыдущего:

```cpp
// Pipeline:
// Слой N:   [QKV calc] -> [Attention] -> [MLP] -> [Residual]
// Слой N+1:               [QKV calc] -> [Attention] -> [MLP]

// Когда Attention слоя N завершается, можно начать QKV слоя N+1
// для операций, которые не зависят от residual
```

**Реализация:**
```cpp
// Использовать два command buffers:
// Buffer 0: Слой N (основной)
// Buffer 1: Предварительные вычисления слоя N+1

// После завершения QKV слоя N, начинать нормализацию слоя N+1
// Использовать events для синхронизации
```

**Ожидаемый эффект:** Ускорение ~10-15% за счет overlap

---

### 7. ✅ Оптимизация Memory Access Patterns

**Проблема:** Неоптимальные паттерны доступа к памяти

**Оптимизации:**
- Использовать coalesced memory access
- Оптимизировать layout тензоров для GPU
- Использовать texture memory для весов (если поддерживается)

**Реализация:**
```cpp
// В ggml_vk_buffer_write():
// Оптимизировать порядок записи для coalesced access

// Использовать VK_FORMAT_R16G16B16A16_SFLOAT для весов
// если GPU поддерживает 16-bit операции
```

**Ожидаемый эффект:** Ускорение ~10-20% за счет лучшего использования bandwidth

---

## Приоритетные оптимизации

### Высокий приоритет (быстрая реализация, большой эффект):

1. **Параллельное вычисление Q, K, V** ⭐⭐⭐
   - Относительно просто реализовать
   - Большой эффект (1.5-2x ускорение attention)
   - Минимальные изменения в коде

2. **Оптимизация Pipeline Barriers** ⭐⭐⭐
   - Средняя сложность
   - Хороший эффект (20-30% уменьшение overhead)
   - Требует анализа зависимостей

3. **Batch Processing матричных операций** ⭐⭐
   - Средняя сложность
   - Умеренный эффект (1.3-1.5x)
   - Требует нового API

### Средний приоритет (требует больше работы):

4. **Async Compute Queues** ⭐⭐
   - Высокая сложность
   - Большой эффект (2-3x), но зависит от GPU
   - Требует значительных изменений архитектуры

5. **Оптимизация Self-Attention** ⭐
   - Средняя сложность
   - Умеренный эффект (если flash_attn уже используется)
   - Требует работы с шейдерами

### Низкий приоритет (сложно реализовать, ограниченный эффект):

6. **Pipeline Overlap между слоями** ⭐
   - Высокая сложность
   - Ограниченный эффект (10-15%)
   - Требует сложной синхронизации

7. **Memory Access Patterns** ⭐
   - Средняя сложность
   - Умеренный эффект (10-20%)
   - Требует профилирования

---

## Рекомендуемый план реализации

### Фаза 1: Быстрые улучшения (1-2 недели)
1. Реализовать параллельное вычисление Q, K, V
2. Оптимизировать pipeline barriers
3. Профилирование и измерение эффекта

### Фаза 2: Средние улучшения (2-4 недели)
4. Реализовать batch processing для матричных операций
5. Улучшить flash attention kernel
6. Оптимизировать memory access patterns

### Фаза 3: Долгосрочные улучшения (1-2 месяца)
7. Реализовать async compute queues
8. Реализовать pipeline overlap между слоями
9. Глубокая оптимизация всех компонентов

---

## Пример кода для параллельного Q, K, V

```cpp
// В whisper_build_graph_encoder(), заменить:

// Было:
struct ggml_tensor * Qcur = ggml_mul_mat(ctx0, layer.attn_q_w, cur);
Qcur = ggml_add(ctx0, Qcur, layer.attn_q_b);
struct ggml_tensor * Kcur = ggml_mul_mat(ctx0, layer.attn_k_w, cur);
struct ggml_tensor * Vcur = ggml_mul_mat(ctx0, layer.attn_v_w, cur);
Vcur = ggml_add(ctx0, Vcur, layer.attn_v_b);

// Стало (граф автоматически распараллелит независимые операции):
struct ggml_tensor * Qcur = ggml_mul_mat(ctx0, layer.attn_q_w, cur);
struct ggml_tensor * Kcur = ggml_mul_mat(ctx0, layer.attn_k_w, cur);
struct ggml_tensor * Vcur = ggml_mul_mat(ctx0, layer.attn_v_w, cur);

// Добавить все операции в граф одновременно
ggml_build_forward_expand(gf, Qcur);
ggml_build_forward_expand(gf, Kcur);
ggml_build_forward_expand(gf, Vcur);

// Затем добавить bias операции (они зависят от Q, K, V)
Qcur = ggml_add(ctx0, Qcur, layer.attn_q_b);
Vcur = ggml_add(ctx0, Vcur, layer.attn_v_b);

ggml_build_forward_expand(gf, Qcur);
ggml_build_forward_expand(gf, Vcur);
```

**Примечание:** GGML scheduler уже должен автоматически распараллеливать независимые операции, но можно явно помочь ему, группируя операции.

---

## Метрики для измерения

После реализации оптимизаций, измерять:

1. **Время выполнения encoder** (encode time)
2. **GPU utilization** (должна быть близка к 100%)
3. **Memory bandwidth utilization**
4. **Количество idle compute units**
5. **Время ожидания синхронизации**

Инструменты:
- `vulkaninfo` для информации о GPU
- Vulkan validation layers для анализа
- GPU profiling tools (Radeon GPU Profiler для AMD, Nsight для NVIDIA)

---

## Заключение

Наибольший эффект дадут:
1. **Параллельное вычисление Q, K, V** - просто и эффективно
2. **Оптимизация barriers** - уменьшит overhead
3. **Async compute queues** - максимизирует использование GPU

Рекомендуется начать с оптимизации #1, так как она дает большой эффект при минимальных изменениях кода.

