#!/bin/bash

echo "=========================================="
echo "GPU0: AMD Radeon Vega 8 Graphics"
echo "=========================================="
vulkaninfo --device 0 2>&1 | grep -i -E "deviceType|deviceName|maxComputeWorkGroup|maxComputeSharedMemory|subgroupSize|shaderFloat16|activeComputeUnitCount|driverName" | head -15

echo ""
echo "=========================================="
echo "GPU1: AMD Radeon RX 6600"
echo "=========================================="
vulkaninfo --device 1 2>&1 | grep -i -E "deviceType|deviceName|maxComputeWorkGroup|maxComputeSharedMemory|subgroupSize|shaderFloat16|activeComputeUnitCount|driverName" | head -15

echo ""
echo "Для полной информации:"
echo "  vulkaninfo --device 0 > gpu0.txt 2>&1"
echo "  vulkaninfo --device 1 > gpu1.txt 2>&1"

