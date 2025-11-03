#!/bin/bash
# Скрипт для пересборки и проверки системных возможностей

set -e

echo "🔨 Пересборка whisper.cpp с Vulkan..."
echo ""

# Сборка с Vulkan
cmake -B build \
  -DGGML_VULKAN=1 \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_CXX_FLAGS="-O3 -march=native -mtune=native" \
  -DCMAKE_C_FLAGS="-O3 -march=native -mtune=native"

echo ""
echo "⚙️  Компиляция system-info утилиты..."
cmake --build build -j$(nproc) --target whisper-system-info

echo ""
echo "✅ Сборка завершена!"
echo ""
echo "═══════════════════════════════════════════════════════════"
echo "🔍 Запускаем проверку системы..."
echo "═══════════════════════════════════════════════════════════"
echo ""

# Запускаем утилиту
./build/bin/whisper-system

echo ""
echo "💡 Дополнительные команды:"
echo ""
echo "  # Проверить с конкретной моделью:"
echo "  ./build/bin/whisper-system -m models/ggml-base.en.bin"
echo ""
echo "  # Проверить Vulkan возможности через vulkaninfo:"
echo "  vulkaninfo | grep -i cooperativeMatrix"
echo ""
echo "  # Запустить worker с подробным логированием:"
echo "  export GGML_VK_LOG_LEVEL=DEBUG"
echo "  ./build/bin/whisper-worker"
echo ""

