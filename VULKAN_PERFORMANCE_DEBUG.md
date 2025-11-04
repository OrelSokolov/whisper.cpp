# Диагностика производительности Vulkan backend

## Что смотреть в `vulkaninfo` для диагностики проблем производительности

### Запуск vulkaninfo
```bash
vulkaninfo --summary > vulkaninfo.txt
# или для конкретного устройства:
vulkaninfo --device 0 --summary > device0.txt
vulkaninfo --device 1 --summary > device1.txt
```

### Ключевые параметры для проверки

#### 1. **Device Type и UMA**
```
deviceType = DISCRETE_GPU или INTEGRATED_GPU
```
- **DISCRETE_GPU** должен быть быстрее интегрированной графики
- **UMA (Unified Memory Architecture)** = 1 означает интегрированную графику

#### 2. **Driver информация**
```
driverName = "radv" (для AMD с открытыми драйверами)
driverID = MESA_RADV
driverVersion
```
- Проверьте версию драйвера - старые версии могут быть медленными
- RADV должен быть актуальной версии

#### 3. **Compute характеристики (критично для производительности)**

**Max Compute Work Group Invocations:**
```
limits.maxComputeWorkGroupInvocations = обычно 1024 или больше
```

**Max Compute Work Group Size:**
```
limits.maxComputeWorkGroupSize = [X, Y, Z]
```
- Должны быть достаточно большими (например, [1024, 1024, 64])

**Max Compute Shared Memory Size:**
```
limits.maxComputeSharedMemorySize = обычно 32-64 KB
```
- RX 6600 должен иметь 64 KB

#### 4. **Subgroup размеры (AMD)**
```
subgroupSize = 32 или 64
```
- RDNA2 (RX 6600) = 32
- Старые AMD (GCN) = 64

**Subgroup Size Control:**
```
VK_EXT_subgroup_size_control supported = true
minSubgroupSize / maxSubgroupSize
```

#### 5. **AMD Shader Core Properties (если доступно)**
```
VK_AMD_shader_core_properties2:
  activeComputeUnitCount = количество вычислительных единиц
  maxShaderAvailableLocalMemorySize
```

Для RX 6600 должно быть:
- ~28 CUs (Compute Units)
- ~1792 потоковых процессоров

#### 6. **FP16 поддержка (критично!)**
```
VK_KHR_shader_float16_int8:
  shaderFloat16 = true
```
- **Должно быть TRUE** - без FP16 производительность сильно падает

#### 7. **Integer Dot Product (для квантизации)**
```
VK_KHR_shader_integer_dot_product:
  integerDotProduct4x8BitPackedSignedAccelerated = true
```

#### 8. **Cooperative Matrix (матричные ядра)**
```
VK_KHR_cooperative_matrix:
  cooperativeMatrix = true
```
- Может ускорить матричные операции

#### 9. **Buffer Device Address**
```
VK_KHR_buffer_device_address:
  bufferDeviceAddress = true
```
- Используется для оптимизаций

### Типичные проблемы низкой производительности

1. **Используется интегрированная графика вместо дискретной**
   - Проверьте deviceType в логах
   - Используйте `--gpu-device 1` если дискретная карта вторая

2. **Отсутствует FP16 поддержка**
   - shaderFloat16 = false
   - Обновить драйвер или проверить расширения

3. **Маленький размер workgroup**
   - maxComputeWorkGroupInvocations < 256
   - Может ограничивать параллелизм

4. **Проблемы с памятью**
   - UMA = 1 означает разделяемую память с CPU
   - Может быть медленнее

5. **Старый драйвер RADV**
   - Проверьте версию в vulkaninfo
   - Обновите Mesa/Vulkan драйверы

### Проверка в логах whisper-worker-rs

После запуска с улучшенным логированием вы увидите:
```
ggml_vulkan: Device 0: AMD Radeon RX 6600 (driver: radv)
ggml_vulkan:   Type: Discrete GPU | UMA: 0 | FP16: 1 | BF16: 0 | Subgroup size: 32
ggml_vulkan:   Shared memory: 64 KB | Max workgroup invocations: 1024 | Integer dot product: 1 | Matrix cores: none
ggml_vulkan:   Max compute workgroup size: [1024, 1024, 64] | Max push constants: 128 bytes
ggml_vulkan:   Shader cores (CUs): 28 | Subgroup size: 32
```

Если видите низкие значения или Type = Integrated GPU - это проблема!

### Сравнение устройств

Запустите `vulkaninfo` для обоих устройств и сравните:
- `maxComputeWorkGroupInvocations`
- `maxComputeSharedMemorySize`
- `activeComputeUnitCount` (для AMD)
- Поддержку FP16 и расширений

Дискретная карта должна иметь значительно лучшие характеристики!

