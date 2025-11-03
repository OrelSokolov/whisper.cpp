// C wrapper for read_audio_data function
// This allows Rust to call the C++ read_audio_data function

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

// Forward declaration - will be linked with common library
extern bool read_audio_data(const char* fname, float** pcmf32, size_t* pcmf32_size, float*** pcmf32s, size_t* pcmf32s_channels, bool stereo);

// C-compatible wrapper
bool read_audio_data_c(
    const char* fname,
    float** pcmf32,
    size_t* pcmf32_size,
    float*** pcmf32s,
    size_t* pcmf32s_channels,
    bool stereo
) {
    // This is a placeholder - we need to adapt the C++ interface
    // The actual read_audio_data uses std::vector which is C++ specific
    // We'll need to use a different approach or create a proper C wrapper in C++
    return false;
}

