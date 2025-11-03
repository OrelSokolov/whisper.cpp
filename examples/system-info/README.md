# whisper-system - System Capabilities Tool

A diagnostic utility for whisper.cpp that displays comprehensive information about your system's hardware capabilities, available compute backends, and provides targeted optimization recommendations.

## Features

- 🖥️ **CPU Information** - Detect CPU features (AVX, AVX2, FMA, etc.)
- 🎮 **GPU Detection** - Identify all available GPU devices with memory information
- 🔧 **Backend Registry** - List all compiled backends (CPU, CUDA, Vulkan, Metal, OpenCL, SYCL, etc.)
- 📊 **Device Capabilities** - Show support for Cooperative Matrix, Integer Dot Product, FP16, BFloat16, etc.
- 💡 **Smart Recommendations** - Get tailored optimization suggestions for your hardware
- 🧪 **Backend Testing** - Optional model loading test to verify backend initialization

## Quick Start

### Build the tool

```bash
# Configure and build (if not already built)
cmake -B build -DGGML_VULKAN=1  # or with other backend flags
cmake --build build -j

# The binary will be at: build/bin/whisper-system
```

### Basic usage

```bash
# Display system capabilities
./build/bin/whisper-system

# Test with a model to verify backend usage
./build/bin/whisper-system -m models/ggml-base.en.bin
```

## Command-Line Options

| Option | Description |
|--------|-------------|
| `-h`, `--help` | Show help message and exit |
| `-m <path>`, `--model <path>` | Test backend with specified model file |

## Output Sections

### 1. System Information

Shows basic hardware info and compiled features:

```
╔══════════════════════════════════════════════════════════════╗
║ SYSTEM INFORMATION                                           ║
╚══════════════════════════════════════════════════════════════╝
  Hardware Concurrency           : 16
  Whisper System Info            : WHISPER : COREML = No | OPENVINO = No | 
                                   CPU : SSE3 = Yes | AVX = Yes | AVX2 = Yes | 
                                   F16C = Yes | FMA = Yes ...
```

### 2. Backend Registry

Lists all available backends and their capabilities:

```
╔══════════════════════════════════════════════════════════════╗
║ BACKEND REGISTRY                                             ║
╚══════════════════════════════════════════════════════════════╝
  Total Backends                 : 3

  Backend #0: Vulkan

  Backend #1: CUDA
    CUDA                         : Yes
    CUBLAS                       : Yes
    FLASH_ATTN                   : Yes

  Backend #2: CPU
    SSE3                         : Yes
    AVX                          : Yes
    AVX2                         : Yes
    F16C                         : Yes
```

### 3. Available Devices

Detailed information about each compute device:

```
╔══════════════════════════════════════════════════════════════╗
║ AVAILABLE DEVICES                                            ║
╚══════════════════════════════════════════════════════════════╝

  Vulkan Backend:
  ════════════════════════════════════════════════════════════
    Device #0:
      Name                      : NVIDIA GeForce RTX 3080
      Description               : NVIDIA GeForce RTX 3080 (driver version 535.161.0)
      Type                      : GPU (Discrete)
      Total Memory              : 10.00 GB
      Free Memory               : 9.50 GB

  CPU Backend:
  ════════════════════════════════════════════════════════════
    Device #0:
      Name                      : CPU
      Description               : AMD Ryzen 9 5950X 16-Core Processor
      Type                      : CPU
      Total Memory              : 64.00 GB
      Free Memory               : 48.50 GB
```

### 4. Optimization Recommendations

Context-aware suggestions based on detected hardware:

```
╔══════════════════════════════════════════════════════════════╗
║ OPTIMIZATION RECOMMENDATIONS                                 ║
╚══════════════════════════════════════════════════════════════╝

  🚀 NVIDIA GPU (CUDA) detected!
     Rebuild with: cmake -B build -DGGML_CUDA=1
     Use quantized models (q5_0, q8_0) for best speed/quality

  🎮 Vulkan support detected!
     Rebuild with optimizations:
       cmake -B build -DGGML_VULKAN=1 \
         -DCMAKE_CXX_FLAGS="-O3 -march=native"
     Check for Cooperative Matrix support:
       vulkaninfo | grep -i cooperativeMatrix

  📦 Model recommendations:
     - For real-time: large-v3-turbo-q5_0 (547 MB, fast)
     - For quality: large-v3-q5_0 (1.1 GB, slower)
     Download: ./models/download-ggml-model.sh <model-name>
```

### 5. Backend Usage Test (with -m flag)

When a model is provided, tests actual backend initialization:

```
╔══════════════════════════════════════════════════════════════╗
║ BACKEND USAGE TEST                                           ║
╚══════════════════════════════════════════════════════════════╝

  Testing with model: models/ggml-base.en.bin

  ✅ Model loaded successfully
  Model type: base.en
  Multilingual: No

  Running test inference...
  ✅ Inference successful

  Check logs above for backend initialization messages
  Look for: 'Vulkan', 'CUDA', 'Metal', etc.
```

## Use Cases

### 1. Pre-Build System Check

Before compiling whisper.cpp, check what hardware acceleration is available:

```bash
./build/bin/whisper-system
# Look for available devices to decide which backends to enable
```

### 2. Post-Build Verification

After building with GPU support, verify it's working:

```bash
./build/bin/whisper-system -m models/ggml-base.en.bin
# Ensure GPU backend is initialized and used
```

### 3. Performance Troubleshooting

Debug why GPU acceleration isn't working as expected:

```bash
# Check if GPU is detected
./build/bin/whisper-system

# Enable debug logging and test with model
export GGML_VK_LOG_LEVEL=DEBUG
./build/bin/whisper-system -m models/ggml-base.en.bin
```

### 4. Hardware Capability Discovery

Find out what optimizations your hardware supports:

```bash
./build/bin/whisper-system | grep -A 10 "BACKEND REGISTRY"
# Look for: Cooperative Matrix, Integer Dot Product, FP16, etc.
```

## Understanding the Output

### Device Types

- **CPU**: Standard CPU computation
- **GPU (Discrete)**: Dedicated GPU with its own VRAM
- **GPU (Integrated)**: Integrated GPU sharing system memory
- **Accelerator**: Specialized accelerators (e.g., BLAS, AMX)

### Key Capabilities

| Capability | Description | Impact |
|------------|-------------|--------|
| **Cooperative Matrix** | Hardware-accelerated matrix operations | 2-3x faster matrix multiplication |
| **Integer Dot Product** | Efficient int8 operations | Essential for quantized models |
| **FP16** | Half-precision floating point | 2x memory bandwidth, faster compute |
| **BFloat16** | Brain floating point (16-bit) | Better accuracy than FP16 for some models |

### Backend Priorities

When multiple backends are available, whisper.cpp typically uses them in this order:
1. CUDA (NVIDIA GPUs)
2. Metal (Apple Silicon)
3. Vulkan (Cross-platform GPU)
4. OpenCL
5. SYCL (Intel)
6. CPU with acceleration (BLAS, AMX)
7. Plain CPU

## Environment Variables

Use these to get more detailed information:

```bash
# Vulkan debug logging
export GGML_VK_LOG_LEVEL=DEBUG

# CUDA debug info
export GGML_CUDA_DEBUG=1

# General backend logging
export GGML_LOG_LEVEL=DEBUG
```

## Integration with Build System

Use whisper-system in your build scripts:

```bash
#!/bin/bash
# Detect best backend automatically
if ./build/bin/whisper-system | grep -q "CUDA"; then
    echo "Building with CUDA support"
    cmake -B build -DGGML_CUDA=1
elif ./build/bin/whisper-system | grep -q "Vulkan"; then
    echo "Building with Vulkan support"
    cmake -B build -DGGML_VULKAN=1
else
    echo "Building with CPU only"
    cmake -B build
fi
```

## Comparing with Other Tools

| Tool | Purpose | When to Use |
|------|---------|-------------|
| `whisper-system` | System capabilities & recommendations | Before/after building, troubleshooting |
| `whisper-bench` | Performance benchmarking | Testing actual inference speed |
| `vulkaninfo` | Vulkan-specific details | Deep Vulkan debugging |
| `nvidia-smi` | NVIDIA GPU monitoring | CUDA-specific monitoring |

## Example Workflows

### Workflow 1: Setting up a new system

```bash
# 1. Check capabilities
./build/bin/whisper-system

# 2. Follow recommendations from output
cmake -B build -DGGML_VULKAN=1 -DCMAKE_CXX_FLAGS="-O3 -march=native"
cmake --build build -j

# 3. Verify it works
./build/bin/whisper-system -m models/ggml-base.en.bin
```

### Workflow 2: Troubleshooting slow inference

```bash
# 1. Check what's being used
export GGML_VK_LOG_LEVEL=DEBUG
./build/bin/whisper-system -m models/ggml-large-v3.bin

# 2. Look for issues in output:
# - Is GPU detected?
# - Is backend initialized?
# - Are there memory issues?

# 3. Try recommended optimizations
# (shown in the recommendations section)
```

### Workflow 3: Choosing the right model

```bash
# 1. Check available GPU memory
./build/bin/whisper-system | grep "Total Memory"

# 2. Match model size to memory:
# - < 4GB: use base.en or small models
# - 4-8GB: use medium or large-v3-turbo-q5_0
# - > 8GB: use large-v3 or large-v3-q5_0

# 3. Download appropriate model
./models/download-ggml-model.sh large-v3-turbo-q5_0
```

## Frequently Asked Questions

**Q: Why does it say "matrix cores: none" on my AMD GPU?**
A: Cooperative Matrix support requires specific hardware features. AMD RDNA2/RDNA3 integrated GPUs typically don't support it. Use quantized models for better performance.

**Q: My GPU is detected but not used during inference. Why?**
A: Run with model testing (`-m` flag) and check logs. Common issues:
- Need to rebuild with backend enabled (`-DGGML_VULKAN=1`)
- Model too large for GPU memory
- Backend initialization failure (check with `GGML_VK_LOG_LEVEL=DEBUG`)

**Q: What does "Yes/No" mean in the output?**
A: These indicate whether specific CPU/GPU features are available:
- `Yes` = Feature is supported and can be used
- `No` = Feature is not available on your hardware

**Q: Should I use CPU or GPU for small models?**
A: Run `whisper-bench` to compare. For small models (tiny, base) on modern CPUs with AVX2, CPU might be faster than integrated GPUs due to overhead.

## Related Documentation

- [../bench/README.md](../bench/README.md) - Benchmarking tool for performance testing
- [../../VULKAN_OPTIMIZATION_GUIDE.md](../../VULKAN_OPTIMIZATION_GUIDE.md) - Vulkan optimization guide
- [../../README.md](../../README.md) - Main whisper.cpp documentation
- [../../README.md#nvidia-gpu-support](../../README.md#nvidia-gpu-support) - NVIDIA GPU setup
- [../../README.md#vulkan-gpu-support](../../README.md#vulkan-gpu-support) - Vulkan setup

## Contributing

Found an issue or want to add support for a new backend? See [../../README.md](../../README.md) for contribution guidelines.

## License

Same as whisper.cpp - MIT License

