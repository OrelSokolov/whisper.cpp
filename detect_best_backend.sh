#!/bin/bash
# Script to detect optimal backend BEFORE building whisper.cpp
# Checks system directly through system utilities

set -e

echo "🔍 Detecting optimal backend for your system..."
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

HAS_CUDA=false
HAS_VULKAN=false
HAS_OPENCL=false
HAS_METAL=false
HAS_SYCL=false

# Check CUDA (NVIDIA)
echo -e "${BLUE}━━━ Checking NVIDIA CUDA ━━━${NC}"
if command -v nvidia-smi &> /dev/null; then
    echo -e "${GREEN}✅ nvidia-smi found${NC}"
    nvidia-smi --query-gpu=name,memory.total --format=csv,noheader 2>/dev/null || true
    HAS_CUDA=true
    echo ""
else
    echo -e "${RED}❌ NVIDIA GPU not detected${NC}"
    echo ""
fi

# Check Vulkan
echo -e "${BLUE}━━━ Checking Vulkan ━━━${NC}"
if command -v vulkaninfo &> /dev/null; then
    echo -e "${GREEN}✅ Vulkan installed${NC}"
    
    # Check devices
    GPU_COUNT=$(vulkaninfo --summary 2>/dev/null | grep -c "GPU" || echo "0")
    if [ "$GPU_COUNT" -gt 0 ]; then
        echo "Found GPUs: $GPU_COUNT"
        vulkaninfo --summary 2>/dev/null | grep -A 2 "GPU" || true
        HAS_VULKAN=true
        
        # Check Cooperative Matrix
        echo ""
        if vulkaninfo 2>/dev/null | grep -i "cooperativeMatrix" | grep -i "true" &> /dev/null; then
            echo -e "${GREEN}✅ Cooperative Matrix supported!${NC}"
        else
            echo -e "${YELLOW}⚠️  Cooperative Matrix not supported${NC}"
        fi
        
        # Check Integer Dot Product
        if vulkaninfo 2>/dev/null | grep -i "integerDotProduct" | grep -i "true" &> /dev/null; then
            echo -e "${GREEN}✅ Integer Dot Product supported!${NC}"
        else
            echo -e "${YELLOW}⚠️  Integer Dot Product not supported${NC}"
        fi
    else
        echo -e "${YELLOW}⚠️  Vulkan installed but no GPUs found${NC}"
    fi
    echo ""
else
    echo -e "${RED}❌ Vulkan not installed${NC}"
    echo "   Install: sudo apt install vulkan-tools libvulkan-dev"
    echo ""
fi

# Check Metal (macOS)
echo -e "${BLUE}━━━ Checking Apple Metal ━━━${NC}"
if [[ "$OSTYPE" == "darwin"* ]]; then
    if system_profiler SPDisplaysDataType 2>/dev/null | grep -q "Metal"; then
        echo -e "${GREEN}✅ Metal supported${NC}"
        HAS_METAL=true
    else
        echo -e "${YELLOW}⚠️  Metal not detected${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  Not macOS - Metal unavailable${NC}"
fi
echo ""

# Check OpenCL
echo -e "${BLUE}━━━ Checking OpenCL ━━━${NC}"
if command -v clinfo &> /dev/null; then
    echo -e "${GREEN}✅ OpenCL installed${NC}"
    OPENCL_DEVICES=$(clinfo 2>/dev/null | grep -c "Device Name" || echo "0")
    if [ "$OPENCL_DEVICES" -gt 0 ]; then
        echo "Found OpenCL devices: $OPENCL_DEVICES"
        clinfo 2>/dev/null | grep "Device Name" | head -5 || true
        HAS_OPENCL=true
    fi
else
    echo -e "${YELLOW}⚠️  clinfo not found${NC}"
    echo "   Install: sudo apt install clinfo"
fi
echo ""

# Check Intel SYCL
echo -e "${BLUE}━━━ Checking Intel SYCL ━━━${NC}"
if command -v sycl-ls &> /dev/null; then
    echo -e "${GREEN}✅ Intel SYCL installed${NC}"
    sycl-ls 2>/dev/null || true
    HAS_SYCL=true
else
    echo -e "${YELLOW}⚠️  Intel SYCL not installed${NC}"
fi
echo ""

# Check CPU capabilities
echo -e "${BLUE}━━━ Checking CPU Capabilities ━━━${NC}"
if command -v lscpu &> /dev/null; then
    CPU_MODEL=$(lscpu | grep "Model name" | cut -d: -f2 | xargs)
    echo "CPU: $CPU_MODEL"
    
    CPU_FLAGS=$(lscpu | grep "Flags" | cut -d: -f2)
    
    if echo "$CPU_FLAGS" | grep -q "avx2"; then
        echo -e "${GREEN}✅ AVX2 supported${NC}"
    fi
    
    if echo "$CPU_FLAGS" | grep -q "avx512"; then
        echo -e "${GREEN}✅ AVX512 supported${NC}"
    fi
    
    if echo "$CPU_FLAGS" | grep -q "fma"; then
        echo -e "${GREEN}✅ FMA supported${NC}"
    fi
fi
echo ""

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# BUILD RECOMMENDATIONS
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}🎯 BUILD RECOMMENDATIONS${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

RECOMMENDED_FLAGS=""
PRIORITY=""

if $HAS_CUDA; then
    PRIORITY="CUDA"
    RECOMMENDED_FLAGS="-DGGML_CUDA=1"
    echo -e "${GREEN}🚀 Optimal choice: NVIDIA CUDA${NC}"
    echo ""
    echo "   Build command:"
    echo -e "${BLUE}   cmake -B build -DGGML_CUDA=1 -DCMAKE_BUILD_TYPE=Release${NC}"
    echo -e "${BLUE}   cmake --build build -j\$(nproc)${NC}"
    echo ""
    echo "   Why CUDA?"
    echo "   - Maximum performance on NVIDIA GPUs"
    echo "   - Best optimization for quantized models"
    echo "   - Support for all modern features"
    echo ""
    
elif $HAS_METAL; then
    PRIORITY="Metal"
    RECOMMENDED_FLAGS="-DGGML_METAL=1"
    echo -e "${GREEN}🍎 Optimal choice: Apple Metal${NC}"
    echo ""
    echo "   Build command:"
    echo -e "${BLUE}   cmake -B build -DGGML_METAL=1 -DCMAKE_BUILD_TYPE=Release${NC}"
    echo -e "${BLUE}   cmake --build build -j\$(sysctl -n hw.ncpu)${NC}"
    echo ""
    
elif $HAS_VULKAN; then
    PRIORITY="Vulkan"
    RECOMMENDED_FLAGS="-DGGML_VULKAN=1"
    echo -e "${GREEN}🎮 Optimal choice: Vulkan${NC}"
    echo ""
    echo "   Build command:"
    echo -e "${BLUE}   cmake -B build -DGGML_VULKAN=1 \\${NC}"
    echo -e "${BLUE}     -DCMAKE_BUILD_TYPE=Release \\${NC}"
    echo -e "${BLUE}     -DCMAKE_CXX_FLAGS=\"-O3 -march=native -mtune=native\" \\${NC}"
    echo -e "${BLUE}     -DCMAKE_C_FLAGS=\"-O3 -march=native -mtune=native\"${NC}"
    echo -e "${BLUE}   cmake --build build -j\$(nproc)${NC}"
    echo ""
    echo "   Why Vulkan?"
    echo "   - Cross-platform solution (AMD, NVIDIA, Intel)"
    echo "   - Good performance on all GPUs"
    echo "   - No proprietary drivers required"
    echo ""
    
elif $HAS_OPENCL; then
    PRIORITY="OpenCL"
    RECOMMENDED_FLAGS="-DGGML_OPENCL=1"
    echo -e "${YELLOW}📊 Available: OpenCL${NC}"
    echo ""
    echo "   Build command:"
    echo -e "${BLUE}   cmake -B build -DGGML_OPENCL=1 -DCMAKE_BUILD_TYPE=Release${NC}"
    echo -e "${BLUE}   cmake --build build -j\$(nproc)${NC}"
    echo ""
    
else
    PRIORITY="CPU"
    RECOMMENDED_FLAGS=""
    echo -e "${YELLOW}💻 Using: CPU (with optimizations)${NC}"
    echo ""
    echo "   Build command:"
    echo -e "${BLUE}   cmake -B build \\${NC}"
    echo -e "${BLUE}     -DCMAKE_BUILD_TYPE=Release \\${NC}"
    echo -e "${BLUE}     -DCMAKE_CXX_FLAGS=\"-O3 -march=native -mtune=native\" \\${NC}"
    echo -e "${BLUE}     -DCMAKE_C_FLAGS=\"-O3 -march=native -mtune=native\"${NC}"
    echo -e "${BLUE}   cmake --build build -j\$(nproc)${NC}"
    echo ""
    echo "   For better CPU performance, install:"
    echo "   - OpenBLAS: sudo apt install libopenblas-dev"
    echo "   Then add: -DGGML_BLAS=ON -DGGML_BLAS_VENDOR=OpenBLAS"
    echo ""
fi

# Additional recommendations
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}📦 MODEL RECOMMENDATIONS${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if $HAS_CUDA || $HAS_VULKAN || $HAS_METAL; then
    echo "For GPU, use quantized models:"
    echo "  • large-v3-turbo-q5_0 (547 MB) - best speed/quality balance"
    echo "  • large-v3-q5_0 (1.1 GB) - maximum quality"
    echo ""
    echo "Download:"
    echo "  ./models/download-ggml-model.sh large-v3-turbo-q5_0"
else
    echo "For CPU, use compact models:"
    echo "  • base.en (142 MB) - English, fast"
    echo "  • small.en (466 MB) - English, quality"
    echo "  • small (466 MB) - multilingual"
    echo ""
    echo "Download:"
    echo "  ./models/download-ggml-model.sh base.en"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}🔄 NEXT STEPS${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "1. Copy the build command above"
echo "2. Run the build"
echo "3. Verify the result:"
echo "   ./build/bin/whisper-system"
echo ""
echo "4. Download appropriate model"
echo "5. Run worker or CLI"
echo ""

# Save recommendation to file
cat > /tmp/whisper_build_recommendation.txt << EOF
Detected: $PRIORITY
Flags: $RECOMMENDED_FLAGS
Date: $(date)
EOF

echo "💾 Result saved to: /tmp/whisper_build_recommendation.txt"
echo ""

