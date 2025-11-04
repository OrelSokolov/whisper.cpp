#!/bin/bash

echo "=========================================="
echo "GPU0: AMD Radeon Vega 8 Graphics (Integrated)"
echo "=========================================="
vulkaninfo --device 0 2>/dev/null | grep -E "(deviceType|deviceName|driverName|maxComputeWorkGroup|maxComputeSharedMemory|subgroupSize|shaderFloat16|VK_AMD_shader_core_properties|activeComputeUnitCount|VK_KHR_shader_float16_int8)" | head -20

echo ""
echo "=========================================="
echo "GPU1: AMD Radeon RX 6600 (Discrete)"
echo "=========================================="
vulkaninfo --device 1 2>/dev/null | grep -E "(deviceType|deviceName|driverName|maxComputeWorkGroup|maxComputeSharedMemory|subgroupSize|shaderFloat16|VK_AMD_shader_core_properties|activeComputeUnitCount|VK_KHR_shader_float16_int8)" | head -20

echo ""
echo "Для полной информации выполните:"
echo "  vulkaninfo --device 0 > gpu0_full.txt"
echo "  vulkaninfo --device 1 > gpu1_full.txt"

