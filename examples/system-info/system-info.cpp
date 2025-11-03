#include "whisper.h"
#include "ggml.h"
#include "ggml-backend.h"

#include <cstdio>
#include <cstring>
#include <string>
#include <thread>
#include <vector>

// Print a section header
static void print_section(const char * title) {
    fprintf(stdout, "\n");
    fprintf(stdout, "╔══════════════════════════════════════════════════════════════╗\n");
    fprintf(stdout, "║ %-60s ║\n", title);
    fprintf(stdout, "╚══════════════════════════════════════════════════════════════╝\n");
}

// Print a key-value pair
static void print_info(const char * key, const char * value) {
    fprintf(stdout, "  %-30s : %s\n", key, value);
}

static void print_info(const char * key, int value) {
    fprintf(stdout, "  %-30s : %d\n", key, value);
}

static void print_info(const char * key, size_t value) {
    fprintf(stdout, "  %-30s : %zu\n", key, value);
}

static void print_info(const char * key, bool value) {
    fprintf(stdout, "  %-30s : %s\n", key, value ? "✅ YES" : "❌ NO");
}

// Print system information
static void print_system_info() {
    print_section("SYSTEM INFORMATION");
    
    print_info("Hardware Concurrency", (int)std::thread::hardware_concurrency());
    print_info("Whisper System Info", whisper_print_system_info());
}

// Print backend registry information
static void print_backend_registry() {
    print_section("BACKEND REGISTRY");
    
    size_t backend_count = ggml_backend_reg_count();
    print_info("Total Backends", (int)backend_count);
    fprintf(stdout, "\n");
    
    for (size_t i = 0; i < backend_count; i++) {
        auto * reg = ggml_backend_reg_get(i);
        const char * reg_name = ggml_backend_reg_name(reg);
        
        fprintf(stdout, "  Backend #%zu: %s\n", i, reg_name);
        
        // Get backend features
        auto * get_features_fn = (ggml_backend_get_features_t) ggml_backend_reg_get_proc_address(reg, "ggml_backend_get_features");
        if (get_features_fn) {
            ggml_backend_feature * features = get_features_fn(reg);
            for (; features->name; features++) {
                // Convert "0"/"1" to "No"/"Yes"
                std::string value = features->value;
                if (value == "0") {
                    value = "No";
                } else if (value == "1") {
                    value = "Yes";
                }
                fprintf(stdout, "    %-28s : %s\n", features->name, value.c_str());
            }
        }
        fprintf(stdout, "\n");
    }
}

// Print device information
static void print_devices_info() {
    print_section("AVAILABLE DEVICES");
    
    size_t backend_count = ggml_backend_reg_count();
    int total_devices = 0;
    
    for (size_t i = 0; i < backend_count; i++) {
        auto * reg = ggml_backend_reg_get(i);
        size_t dev_count = ggml_backend_reg_dev_count(reg);
        
        if (dev_count == 0) {
            continue;
        }
        
        fprintf(stdout, "\n  %s Backend:\n", ggml_backend_reg_name(reg));
        fprintf(stdout, "  ════════════════════════════════════════════════════════════\n");
        
        for (size_t j = 0; j < dev_count; j++) {
            auto * dev = ggml_backend_reg_dev_get(reg, j);
            const char * dev_name = ggml_backend_dev_name(dev);
            const char * dev_desc = ggml_backend_dev_description(dev);
            enum ggml_backend_dev_type dev_type = ggml_backend_dev_type(dev);
            
            fprintf(stdout, "\n    Device #%zu:\n", j);
            fprintf(stdout, "      Name                      : %s\n", dev_name);
            fprintf(stdout, "      Description               : %s\n", dev_desc);
            
            const char * type_str = "Unknown";
            switch (dev_type) {
                case GGML_BACKEND_DEVICE_TYPE_CPU:      type_str = "CPU"; break;
                case GGML_BACKEND_DEVICE_TYPE_GPU:      type_str = "GPU (Discrete)"; break;
                case GGML_BACKEND_DEVICE_TYPE_IGPU:     type_str = "GPU (Integrated)"; break;
                case GGML_BACKEND_DEVICE_TYPE_ACCEL:    type_str = "Accelerator"; break;
                default:                                 type_str = "Unknown"; break;
            }
            fprintf(stdout, "      Type                      : %s\n", type_str);
            
            // Get memory info
            size_t free_mem = 0, total_mem = 0;
            ggml_backend_dev_memory(dev, &free_mem, &total_mem);
            if (total_mem > 0) {
                fprintf(stdout, "      Total Memory              : %.2f GB\n", total_mem / (1024.0 * 1024.0 * 1024.0));
                fprintf(stdout, "      Free Memory               : %.2f GB\n", free_mem / (1024.0 * 1024.0 * 1024.0));
            }
            
            total_devices++;
        }
    }
    
    fprintf(stdout, "\n");
    print_info("Total Devices Found", total_devices);
}

// Print GPU-specific recommendations
static void print_recommendations() {
    print_section("OPTIMIZATION RECOMMENDATIONS");
    
    bool has_vulkan = false;
    bool has_cuda = false;
    bool has_metal = false;
    bool has_opencl = false;
    bool has_sycl = false;
    
    size_t backend_count = ggml_backend_reg_count();
    for (size_t i = 0; i < backend_count; i++) {
        auto * reg = ggml_backend_reg_get(i);
        const char * name = ggml_backend_reg_name(reg);
        size_t dev_count = ggml_backend_reg_dev_count(reg);
        
        if (dev_count > 0) {
            if (strstr(name, "Vulkan")) has_vulkan = true;
            if (strstr(name, "CUDA")) has_cuda = true;
            if (strstr(name, "Metal")) has_metal = true;
            if (strstr(name, "OpenCL")) has_opencl = true;
            if (strstr(name, "SYCL")) has_sycl = true;
        }
    }
    
    fprintf(stdout, "\n");
    
    if (has_cuda) {
        fprintf(stdout, "  🚀 NVIDIA GPU (CUDA) detected!\n");
        fprintf(stdout, "     Rebuild with: cmake -B build -DGGML_CUDA=1\n");
        fprintf(stdout, "     Use quantized models (q5_0, q8_0) for best speed/quality\n\n");
    }
    
    if (has_vulkan) {
        fprintf(stdout, "  🎮 Vulkan support detected!\n");
        fprintf(stdout, "     Rebuild with optimizations:\n");
        fprintf(stdout, "       cmake -B build -DGGML_VULKAN=1 \\\n");
        fprintf(stdout, "         -DCMAKE_CXX_FLAGS=\"-O3 -march=native\"\n");
        fprintf(stdout, "     Check for Cooperative Matrix support:\n");
        fprintf(stdout, "       vulkaninfo | grep -i cooperativeMatrix\n");
        fprintf(stdout, "     Set environment for detailed logs:\n");
        fprintf(stdout, "       export GGML_VK_LOG_LEVEL=DEBUG\n\n");
    }
    
    if (has_metal) {
        fprintf(stdout, "  🍎 Apple Metal detected!\n");
        fprintf(stdout, "     Rebuild with: cmake -B build -DGGML_METAL=1\n\n");
    }
    
    if (has_sycl) {
        fprintf(stdout, "  💠 Intel SYCL detected!\n");
        fprintf(stdout, "     Rebuild with: cmake -B build -DGGML_SYCL=1\n\n");
    }
    
    if (has_opencl) {
        fprintf(stdout, "  📊 OpenCL support detected!\n");
        fprintf(stdout, "     Rebuild with: cmake -B build -DGGML_OPENCL=1\n\n");
    }
    
    if (!has_vulkan && !has_cuda && !has_metal && !has_opencl && !has_sycl) {
        fprintf(stdout, "  💡 No GPU acceleration detected.\n");
        fprintf(stdout, "     Using CPU backend. Consider:\n");
        fprintf(stdout, "     - Installing Vulkan drivers for cross-platform GPU support\n");
        fprintf(stdout, "     - Using BLAS (OpenBLAS/MKL) for CPU optimization\n");
        fprintf(stdout, "     - Rebuild with: cmake -B build -DGGML_BLAS=ON\n\n");
    }
    
    fprintf(stdout, "  📦 Model recommendations:\n");
    fprintf(stdout, "     - For real-time: large-v3-turbo-q5_0 (547 MB, fast)\n");
    fprintf(stdout, "     - For quality: large-v3-q5_0 (1.1 GB, slower)\n");
    fprintf(stdout, "     - For CPU: base.en or small.en models\n");
    fprintf(stdout, "     Download: ./models/download-ggml-model.sh <model-name>\n");
}

// Test a simple inference to see what backend is actually used
static void test_backend_usage(const char * model_path) {
    print_section("BACKEND USAGE TEST");
    
    fprintf(stdout, "\n");
    fprintf(stdout, "  Testing with model: %s\n\n", model_path);
    
    struct whisper_context_params cparams = whisper_context_default_params();
    cparams.use_gpu = true;
    
    struct whisper_context * ctx = whisper_init_from_file_with_params(model_path, cparams);
    if (ctx == nullptr) {
        fprintf(stdout, "  ⚠️  Could not load model for testing\n");
        fprintf(stdout, "  Specify a valid model path with -m to test backend usage\n");
        return;
    }
    
    fprintf(stdout, "  ✅ Model loaded successfully\n");
    fprintf(stdout, "  Model type: %s\n", whisper_model_type_readable(ctx));
    fprintf(stdout, "  Multilingual: %s\n", whisper_is_multilingual(ctx) ? "Yes" : "No");
    
    // Try encoding with dummy data to trigger backend usage
    const int n_mels = whisper_model_n_mels(ctx);
    std::vector<float> dummy_mel(n_mels * 3000, 0.0f);
    
    fprintf(stdout, "\n  Running test inference...\n");
    if (whisper_set_mel(ctx, dummy_mel.data(), 3000, n_mels) == 0) {
        if (whisper_encode(ctx, 0, 1) == 0) {
            fprintf(stdout, "  ✅ Inference successful\n");
            fprintf(stdout, "\n  Check logs above for backend initialization messages\n");
            fprintf(stdout, "  Look for: 'Vulkan', 'CUDA', 'Metal', etc.\n");
        } else {
            fprintf(stdout, "  ⚠️  Inference failed\n");
        }
    }
    
    whisper_free(ctx);
}

int main(int argc, char ** argv) {
    // Load all backends
    ggml_backend_load_all();
    
    const char * model_path = nullptr;
    bool test_inference = false;
    
    // Parse arguments
    for (int i = 1; i < argc; i++) {
        std::string arg = argv[i];
        if (arg == "-h" || arg == "--help") {
            fprintf(stdout, "Usage: %s [options]\n", argv[0]);
            fprintf(stdout, "\nOptions:\n");
            fprintf(stdout, "  -h, --help              Show this help message\n");
            fprintf(stdout, "  -m, --model <path>      Test backend with specified model\n");
            fprintf(stdout, "\nExamples:\n");
            fprintf(stdout, "  %s\n", argv[0]);
            fprintf(stdout, "  %s -m models/ggml-base.en.bin\n", argv[0]);
            return 0;
        }
        else if (arg == "-m" || arg == "--model") {
            if (i + 1 < argc) {
                model_path = argv[++i];
                test_inference = true;
            }
        }
    }
    
    fprintf(stdout, "\n");
    fprintf(stdout, "╔══════════════════════════════════════════════════════════════╗\n");
    fprintf(stdout, "║               WHISPER.CPP SYSTEM CAPABILITIES                ║\n");
    fprintf(stdout, "╚══════════════════════════════════════════════════════════════╝\n");
    
    print_system_info();
    print_backend_registry();
    print_devices_info();
    print_recommendations();
    
    if (test_inference && model_path) {
        test_backend_usage(model_path);
    }
    
    fprintf(stdout, "\n");
    fprintf(stdout, "═══════════════════════════════════════════════════════════════\n");
    fprintf(stdout, "For more information, visit:\n");
    fprintf(stdout, "  https://github.com/ggerganov/whisper.cpp\n");
    fprintf(stdout, "═══════════════════════════════════════════════════════════════\n");
    fprintf(stdout, "\n");
    
    return 0;
}

