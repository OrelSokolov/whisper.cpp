#include "common.h"
#include "common-whisper.h"

#include "whisper.h"
#include "grammar-parser.h"

#include <cmath>
#include <cstdint>
#include <algorithm>
#include <fstream>
#include <cstdio>
#include <iostream>
#include <iomanip>
#include <string>
#include <thread>
#include <vector>
#include <cstring>
#include <cfloat>
#include <sstream>
#include <atomic>
#include <mutex>

#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <unistd.h>
#include <openssl/sha.h>
#include <openssl/evp.h>
#include <openssl/bio.h>
#include <openssl/buffer.h>
#include <chrono>

// Helper function to get current timestamp string
static std::string get_timestamp() {
    auto now = std::chrono::system_clock::now();
    auto now_time_t = std::chrono::system_clock::to_time_t(now);
    auto now_ms = std::chrono::duration_cast<std::chrono::milliseconds>(now.time_since_epoch()) % 1000;
    
    std::tm tm_buf;
    localtime_r(&now_time_t, &tm_buf);
    
    std::ostringstream oss;
    oss << std::put_time(&tm_buf, "%Y-%m-%d %H:%M:%S")
        << '.' << std::setfill('0') << std::setw(3) << now_ms.count();
    return oss.str();
}

// Log with timestamp
#define LOG_INFO(...) fprintf(stderr, "[%s] ", get_timestamp().c_str()); fprintf(stderr, __VA_ARGS__)

// Helper for network byte order conversion (big-endian to host)
// Используем встроенную функцию из endian.h, если доступна
#ifndef be64toh
static inline uint64_t be64toh_custom(uint64_t value) {
    uint8_t* bytes = (uint8_t*)&value;
    return ((uint64_t)bytes[0] << 56) |
           ((uint64_t)bytes[1] << 48) |
           ((uint64_t)bytes[2] << 40) |
           ((uint64_t)bytes[3] << 32) |
           ((uint64_t)bytes[4] << 24) |
           ((uint64_t)bytes[5] << 16) |
           ((uint64_t)bytes[6] << 8) |
           ((uint64_t)bytes[7]);
}
#define be64toh be64toh_custom
#endif

#define WHISPER_WORKER_PORT 8765
#define WS_MAGIC_STRING "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"

// WebSocket frame opcodes
#define WS_OPCODE_CONT 0x0
#define WS_OPCODE_TEXT 0x1
#define WS_OPCODE_BINARY 0x2
#define WS_OPCODE_CLOSE 0x8
#define WS_OPCODE_PING 0x9
#define WS_OPCODE_PONG 0xA

// Base64 encoding helper
std::string base64_encode(const unsigned char* input, int length) {
    BIO *bio, *b64;
    BUF_MEM *bufferPtr;
    
    b64 = BIO_new(BIO_f_base64());
    bio = BIO_new(BIO_s_mem());
    bio = BIO_push(b64, bio);
    
    BIO_set_flags(bio, BIO_FLAGS_BASE64_NO_NL);
    BIO_write(bio, input, length);
    BIO_flush(bio);
    BIO_get_mem_ptr(bio, &bufferPtr);
    
    std::string encoded_data(bufferPtr->data, bufferPtr->length);
    BIO_free_all(bio);
    
    return encoded_data;
}

// SHA1 hash helper
std::string sha1_hash(const std::string& input) {
    unsigned char hash[SHA_DIGEST_LENGTH];
    SHA1((unsigned char*)input.c_str(), input.length(), hash);
    return std::string((char*)hash, SHA_DIGEST_LENGTH);
}

// Extract WebSocket key from HTTP headers
std::string extract_ws_key(const std::string& request) {
    size_t key_pos = request.find("Sec-WebSocket-Key:");
    if (key_pos == std::string::npos) return "";
    
    size_t key_start = request.find(" ", key_pos) + 1;
    size_t key_end = request.find("\r\n", key_start);
    if (key_end == std::string::npos) return "";
    
    std::string key = request.substr(key_start, key_end - key_start);
    // Remove whitespace
    key.erase(std::remove_if(key.begin(), key.end(), ::isspace), key.end());
    return key;
}

// Create WebSocket accept response
std::string create_ws_accept_response(const std::string& key) {
    std::string accept_key = key + WS_MAGIC_STRING;
    std::string hash = sha1_hash(accept_key);
    std::string encoded = base64_encode((unsigned char*)hash.c_str(), hash.length());
    
    std::string response = 
        "HTTP/1.1 101 Switching Protocols\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        "Sec-WebSocket-Accept: " + encoded + "\r\n"
        "\r\n";
    
    return response;
}

// Read WebSocket frame
bool read_ws_frame(int socket, std::vector<uint8_t>& payload, uint8_t& opcode) {
    uint8_t header[2];
    ssize_t n = recv(socket, header, 2, MSG_WAITALL);
    if (n <= 0 || n != 2) return false;  // Connection closed or error
    
    bool fin = (header[0] & 0x80) != 0;
    opcode = header[0] & 0x0F;
    bool masked = (header[1] & 0x80) != 0;
    uint64_t payload_len = header[1] & 0x7F;
    
    if (payload_len == 126) {
        uint16_t len;
        n = recv(socket, &len, 2, MSG_WAITALL);
        if (n <= 0 || n != 2) return false;
        payload_len = ntohs(len);
    } else if (payload_len == 127) {
        uint64_t len;
        n = recv(socket, &len, 8, MSG_WAITALL);
        if (n <= 0 || n != 8) return false;
        // Convert from network byte order
        len = be64toh(len);
        payload_len = len;
    }
    
    uint8_t mask[4];
    if (masked) {
        n = recv(socket, mask, 4, MSG_WAITALL);
        if (n <= 0 || n != 4) return false;
    }
    
    payload.resize(payload_len);
    if (payload_len > 0) {
        n = recv(socket, payload.data(), payload_len, MSG_WAITALL);
        if (n <= 0 || n != (ssize_t)payload_len) return false;
        
        if (masked) {
            for (size_t i = 0; i < payload.size(); i++) {
                payload[i] ^= mask[i % 4];
            }
        }
    }
    
    return true;
}

// Write WebSocket frame
bool write_ws_frame(int socket, const std::string& data, uint8_t opcode = WS_OPCODE_TEXT) {
    std::vector<uint8_t> frame;
    frame.push_back(0x80 | opcode); // FIN bit set
    
    size_t len = data.length();
    if (len < 126) {
        frame.push_back(len);
    } else if (len < 65536) {
        frame.push_back(126);
        frame.push_back((len >> 8) & 0xFF);
        frame.push_back(len & 0xFF);
    } else {
        frame.push_back(127);
        for (int i = 7; i >= 0; i--) {
            frame.push_back((len >> (i * 8)) & 0xFF);
        }
    }
    
    frame.insert(frame.end(), data.begin(), data.end());
    
    // Use MSG_NOSIGNAL to avoid SIGPIPE when client disconnects
    ssize_t sent = send(socket, frame.data(), frame.size(), MSG_NOSIGNAL);
    if (sent <= 0) {
        // Connection closed or error (EPIPE will not generate SIGPIPE)
        return false;
    }
    return sent == (ssize_t)frame.size();
}

// Command-line parameters (reused from CLI - exact same defaults)
struct whisper_params {
    int32_t n_threads     = std::min(4, (int32_t) std::thread::hardware_concurrency());
    int32_t n_processors  = 1;
    int32_t offset_t_ms   = 0;
    int32_t offset_n      = 0;
    int32_t duration_ms   = 0;
    int32_t progress_step = 5;
    int32_t max_context   = -1;
    int32_t max_len       = 0;
    int32_t best_of       = whisper_full_default_params(WHISPER_SAMPLING_GREEDY).greedy.best_of;
    int32_t beam_size     = whisper_full_default_params(WHISPER_SAMPLING_BEAM_SEARCH).beam_search.beam_size;
    int32_t audio_ctx     = 0;

    float word_thold      =  0.01f;
    float entropy_thold   =  2.40f;
    float logprob_thold   = -1.00f;
    float no_speech_thold =  0.6f;
    float grammar_penalty = 100.0f;
    float temperature     = 0.0f;
    float temperature_inc = 0.2f;

    bool debug_mode      = false;
    bool translate       = false;
    bool detect_language = false;
    bool diarize         = false;
    bool tinydiarize     = false;
    bool split_on_word   = false;
    bool no_fallback     = false;
    bool print_special   = false;
    bool print_colors    = false;
    bool print_confidence= false;
    bool print_progress  = false;
    bool no_timestamps   = false;
    bool log_score       = false;
    bool use_gpu         = true;
    bool flash_attn      = true;
    bool suppress_nst    = false;
    bool verbose         = false;
    bool carry_initial_prompt = false;

    std::string language  = "auto";  // Auto-detect language by default
    std::string prompt;
    std::string model     = "models/ggml-base.bin";  // Multilingual model by default
    std::string grammar;
    std::string grammar_rule;
    std::string suppress_regex;
    std::string openvino_encode_device = "CPU";
    std::string dtw = "";

    grammar_parser::parse_state grammar_parsed;
};

// Global whisper context (loaded once, kept in memory)
struct whisper_context * g_ctx = nullptr;
std::mutex g_ctx_mutex;
whisper_params g_params;

// Estimate diarization speaker (from CLI)
static std::string estimate_diarization_speaker(std::vector<std::vector<float>> pcmf32s, int64_t t0, int64_t t1, bool id_only = false) {
    std::string speaker = "";
    const int64_t n_samples = pcmf32s[0].size();

    const int64_t is0 = timestamp_to_sample(t0, n_samples, WHISPER_SAMPLE_RATE);
    const int64_t is1 = timestamp_to_sample(t1, n_samples, WHISPER_SAMPLE_RATE);

    double energy0 = 0.0f;
    double energy1 = 0.0f;

    for (int64_t j = is0; j < is1; j++) {
        energy0 += fabs(pcmf32s[0][j]);
        energy1 += fabs(pcmf32s[1][j]);
    }

    if (energy0 > 1.1*energy1) {
        speaker = "0";
    } else if (energy1 > 1.1*energy0) {
        speaker = "1";
    } else {
        speaker = "?";
    }

    if (!id_only) {
        speaker.insert(0, "(speaker ");
        speaker.append(")");
    }

    return speaker;
}

// Segment callback that sends results via WebSocket
// Uses same structure as CLI for consistency, with added audio duration for progress
struct ws_user_data {
    int socket;
    const whisper_params* params;
    const std::vector<std::vector<float>>* pcmf32s;
    float audio_duration_s;  // Total audio duration in seconds for progress calculation
    bool is_aborted;  // Set to true if client disconnected
    int64_t t_start_process_us;  // Start time for ETA calculation
};

void whisper_ws_segment_callback(struct whisper_context * ctx, struct whisper_state * /*state*/, int n_new, void * user_data) {
    auto* ws_data = static_cast<ws_user_data*>(user_data);
    const auto & params = *ws_data->params;
    const auto & pcmf32s = *ws_data->pcmf32s;
    const float audio_duration_s = ws_data->audio_duration_s;
    
    const int n_segments = whisper_full_n_segments(ctx);
    
    // print the last n_new segments (same logic as CLI)
    const int s0 = n_segments - n_new;
    
    for (int i = s0; i < n_segments; i++) {
        const char * text = whisper_full_get_segment_text(ctx, i);
        
        // Get timestamps (same as CLI)
        int64_t t0 = 0;
        int64_t t1 = 0;
        if (!params.no_timestamps || params.diarize) {
            t0 = whisper_full_get_segment_t0(ctx, i);
            t1 = whisper_full_get_segment_t1(ctx, i);
        }
        
        std::ostringstream json;
        json << "{\"type\":\"segment\",\"index\":" << i << ",\"text\":\"";
        
        // Escape JSON string
        for (const char* p = text; *p; p++) {
            if (*p == '"' || *p == '\\') {
                json << '\\' << *p;
            } else if (*p == '\n') {
                json << "\\n";
            } else if (*p == '\r') {
                json << "\\r";
            } else if (*p == '\t') {
                json << "\\t";
            } else {
                json << *p;
            }
        }
        json << "\"";
        
        if (!params.no_timestamps) {
            json << ",\"start\":" << (t0 * 0.01) << ",\"end\":" << (t1 * 0.01);
        }
        
        if (params.diarize && pcmf32s.size() == 2) {
            std::string speaker = estimate_diarization_speaker(pcmf32s, t0, t1, true);
            json << ",\"speaker\":\"" << speaker << "\"";
        }
        
        // Calculate and add progress and ETA based on last segment's end time
        if (audio_duration_s > 0 && !params.no_timestamps) {
            const float current_time_s = t1 * 0.01f;
            const float progress = (current_time_s / audio_duration_s) * 100.0f;
            // Clamp progress to 0-100 range
            const float progress_clamped = std::min(100.0f, std::max(0.0f, progress));
            json << ",\"progress\":" << std::fixed << std::setprecision(1) << progress_clamped;
            
            // Calculate ETA (estimated time remaining)
            if (current_time_s > 0.0f && ws_data->t_start_process_us > 0) {
                const int64_t t_now_us = ggml_time_us();
                const float elapsed_s = (t_now_us - ws_data->t_start_process_us) / 1000000.0f;
                const float current_realtime_factor = current_time_s / elapsed_s;
                const float remaining_audio_s = audio_duration_s - current_time_s;
                const float eta_s = remaining_audio_s / current_realtime_factor;
                
                // Only add ETA if it's reasonable (not negative, not too large)
                if (eta_s > 0.0f && eta_s < 3600.0f) {  // Less than 1 hour
                    json << ",\"eta\":" << std::fixed << std::setprecision(1) << eta_s;
                }
            }
        }
        
        json << "}\n";
        
        // Try to send the frame, if it fails - mark as aborted
        if (!write_ws_frame(ws_data->socket, json.str(), WS_OPCODE_TEXT)) {
            LOG_INFO("Failed to send segment to client, marking as aborted\n");
            ws_data->is_aborted = true;
            break;  // Stop processing segments
        }
    }
}

// Process audio data and send results via WebSocket
// NOTE: Caller must hold g_ctx_mutex lock before calling this function
bool process_audio_websocket(int socket, const std::vector<uint8_t>& audio_data) {
    
    if (!g_ctx) {
        write_ws_frame(socket, "{\"type\":\"error\",\"message\":\"Model not loaded\"}\n");
        return false;
    }
    
    if (audio_data.empty()) {
        write_ws_frame(socket, "{\"type\":\"error\",\"message\":\"Empty audio data\"}\n");
        return false;
    }
    
    // Decode audio from memory buffer
    std::vector<float> pcmf32;
    std::vector<std::vector<float>> pcmf32s;
    
    // Create temporary file for audio data
    char tmpfile[] = "/tmp/whisper_worker_XXXXXX";
    int fd = mkstemp(tmpfile);
    if (fd == -1) {
        write_ws_frame(socket, "{\"type\":\"error\",\"message\":\"Failed to create temp file\"}\n");
        return false;
    }
    
    ssize_t written = write(fd, audio_data.data(), audio_data.size());
    if (written != (ssize_t)audio_data.size()) {
        close(fd);
        unlink(tmpfile);
        write_ws_frame(socket, "{\"type\":\"error\",\"message\":\"Failed to write temp file\"}\n");
        return false;
    }
    close(fd);
    
    // Read audio data using common-whisper function
    bool success = read_audio_data(std::string(tmpfile), pcmf32, pcmf32s, g_params.diarize);
    unlink(tmpfile);
    
    if (!success) {
        write_ws_frame(socket, "{\"type\":\"error\",\"message\":\"Failed to decode audio\"}\n");
        return false;
    }
    
    // Calculate and log audio duration
    const float audio_duration_s = pcmf32.size() / (float)WHISPER_SAMPLE_RATE;
    LOG_INFO("Audio decoded successfully: %.2f seconds (%.2f minutes, %zu samples)\n", 
             audio_duration_s, audio_duration_s / 60.0f, pcmf32.size());
    
    // Check language settings
    if (!whisper_is_multilingual(g_ctx)) {
        if (g_params.language != "en" || g_params.translate) {
            g_params.language = "en";
            g_params.translate = false;
        }
    }
    
    if (g_params.detect_language) {
        g_params.language = "auto";
    }
    
    // Setup whisper parameters (exact same as CLI)
    whisper_full_params wparams = whisper_full_default_params(WHISPER_SAMPLING_GREEDY);
    
    const bool use_grammar = (!g_params.grammar_parsed.rules.empty() && !g_params.grammar_rule.empty());
    wparams.strategy = (g_params.beam_size > 1 || use_grammar) ? WHISPER_SAMPLING_BEAM_SEARCH : WHISPER_SAMPLING_GREEDY;
    
    wparams.print_realtime   = false;
    wparams.print_progress   = false;
    wparams.print_timestamps = !g_params.no_timestamps;
    wparams.print_special    = g_params.print_special;
    wparams.translate        = g_params.translate;
    wparams.language         = g_params.language.c_str();
    wparams.detect_language  = g_params.detect_language;
    wparams.n_threads        = g_params.n_threads;
    wparams.n_max_text_ctx   = g_params.max_context >= 0 ? g_params.max_context : wparams.n_max_text_ctx;
    wparams.offset_ms        = g_params.offset_t_ms;
    wparams.duration_ms      = g_params.duration_ms;
    // Use same logic as CLI for token_timestamps and max_len
    wparams.token_timestamps = false; // Not using word-level timestamps
    wparams.thold_pt         = g_params.word_thold;
    // In CLI: wparams.max_len = params.output_wts && params.max_len == 0 ? 60 : params.max_len;
    // For worker, keep max_len as is (0 means no limit, which is correct for full transcription)
    wparams.max_len          = g_params.max_len;
    wparams.split_on_word    = g_params.split_on_word;
    wparams.audio_ctx        = g_params.audio_ctx;
    wparams.debug_mode       = g_params.debug_mode;
    wparams.tdrz_enable      = g_params.tinydiarize;
    wparams.suppress_regex   = g_params.suppress_regex.empty() ? nullptr : g_params.suppress_regex.c_str();
    wparams.initial_prompt       = g_params.prompt.c_str();
    // IMPORTANT: Disable carry_initial_prompt for worker
    // to avoid contamination between different requests (each request should be independent)
    // NOTE: no_context affects segments WITHIN a single transcription, not between requests
    // Setting no_context=true degrades quality (missing punctuation, capitalization)
    wparams.carry_initial_prompt = false;
    wparams.greedy.best_of        = g_params.best_of;
    wparams.beam_search.beam_size = g_params.beam_size;
    wparams.temperature_inc  = g_params.no_fallback ? 0.0f : g_params.temperature_inc;
    wparams.temperature      = g_params.temperature;
    wparams.entropy_thold    = g_params.entropy_thold;
    wparams.logprob_thold    = g_params.logprob_thold;
    wparams.no_speech_thold  = g_params.no_speech_thold;
    wparams.no_timestamps    = g_params.no_timestamps;
    wparams.suppress_nst     = g_params.suppress_nst;
    
    // Capture start time for benchmark and ETA (must be before ws_data initialization)
    const int64_t t_start_process_us = ggml_time_us();
    
    // Setup WebSocket callback (same pattern as CLI) - audio_duration_s already declared above
    ws_user_data ws_data = { socket, &g_params, &pcmf32s, audio_duration_s, false, t_start_process_us };
    
    // Set callback only if not in realtime mode (same as CLI)
    if (!wparams.print_realtime) {
        wparams.new_segment_callback = whisper_ws_segment_callback;
        wparams.new_segment_callback_user_data = &ws_data;
    }
    
    // Set abort callback to stop processing if client disconnects
    wparams.abort_callback = [](void *user_data) {
        auto* ws_data = static_cast<ws_user_data*>(user_data);
        return ws_data->is_aborted;
    };
    wparams.abort_callback_user_data = &ws_data;
    
    // Process audio - check if client is still connected
    if (!write_ws_frame(socket, "{\"type\":\"status\",\"message\":\"Processing audio...\"}\n")) {
        LOG_INFO("Client disconnected before processing started\n");
        return false;
    }
    
    if (whisper_full_parallel(g_ctx, wparams, pcmf32.data(), pcmf32.size(), g_params.n_processors) != 0) {
        if (ws_data.is_aborted) {
            LOG_INFO("Processing aborted due to client disconnect\n");
        } else {
            write_ws_frame(socket, "{\"type\":\"error\",\"message\":\"Failed to process audio\"}\n");
        }
        return false;
    }
    
    // Capture end time for benchmark
    const int64_t t_end_process_us = ggml_time_us();
    
    // Print timing statistics to server console (like CLI)
    LOG_INFO("Processing complete, printing statistics:\n");
    whisper_print_timings(g_ctx);
    
    // Calculate and print realtime factor (same as CLI) - reuse audio_duration_s from above
    const float total_time_s = (t_end_process_us - t_start_process_us) / 1000000.0f;
    const float realtime_factor = audio_duration_s / total_time_s;
    
    LOG_INFO("\n");
    LOG_INFO("benchmark_cli_factor: audio duration  = %7.2f sec\n", audio_duration_s);
    LOG_INFO("benchmark_cli_factor: total time      = %7.2f ms\n", total_time_s * 1000.0f);
    LOG_INFO("benchmark_cli_factor: realtime factor = %7.2fx\n", realtime_factor);
    
    // Send completion message (check if not aborted)
    if (!ws_data.is_aborted) {
        if (!write_ws_frame(socket, "{\"type\":\"complete\"}\n")) {
            LOG_INFO("Failed to send completion message\n");
            return false;
        }
        // Give client time to receive the complete message before closing
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        return true;
    }
    
    return false;
}

// Handle WebSocket connection
void handle_websocket_connection(int client_socket) {
    char buffer[4096];
    ssize_t n = recv(client_socket, buffer, sizeof(buffer) - 1, 0);
    if (n <= 0) {
        LOG_INFO("Failed to receive handshake data\n");
        close(client_socket);
        return;
    }
    
    buffer[n] = '\0';
    std::string request(buffer);
    
    // Check if it's a WebSocket upgrade request
    if (request.find("Upgrade: websocket") == std::string::npos) {
        LOG_INFO("Not a WebSocket upgrade request\n");
        close(client_socket);
        return;
    }
    
    // Extract WebSocket key and send accept response
    std::string ws_key = extract_ws_key(request);
    if (ws_key.empty()) {
        LOG_INFO("Failed to extract WebSocket key\n");
        close(client_socket);
        return;
    }
    
    std::string response = create_ws_accept_response(ws_key);
    ssize_t sent = send(client_socket, response.c_str(), response.length(), MSG_NOSIGNAL);
    if (sent <= 0) {
        LOG_INFO("Failed to send WebSocket handshake response\n");
        close(client_socket);
        return;
    }
    
    // Check if server is busy (try to lock mutex without blocking)
    std::unique_lock<std::mutex> lock(g_ctx_mutex, std::try_to_lock);
    if (!lock.owns_lock()) {
        LOG_INFO("Server is busy processing another request\n");
        write_ws_frame(client_socket, "{\"type\":\"error\",\"message\":\"Server is busy, please try again later\"}\n");
        // Give client time to receive the message before closing
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        close(client_socket);
        return;
    }
    // Mutex is now locked and will be automatically unlocked when lock goes out of scope
    
    // Now handle WebSocket frames
    std::vector<uint8_t> audio_buffer;
    bool audio_complete = false;
    
    LOG_INFO("WebSocket connection established, waiting for data...\n");
    
    while (true) {
        std::vector<uint8_t> payload;
        uint8_t opcode;
        
        if (!read_ws_frame(client_socket, payload, opcode)) {
            LOG_INFO("Connection closed by client or read error\n");
            break;
        }
        
        LOG_INFO("Received frame: opcode=%d, payload_size=%zu\n", opcode, payload.size());
        
        if (opcode == WS_OPCODE_CLOSE) {
            LOG_INFO("Received close frame\n");
            break;
        } else if (opcode == WS_OPCODE_PING) {
            write_ws_frame(client_socket, "", WS_OPCODE_PONG);
        } else if (opcode == WS_OPCODE_BINARY) {
            // Accumulate audio data
            audio_buffer.insert(audio_buffer.end(), payload.begin(), payload.end());
            audio_complete = true;
            LOG_INFO("Received binary data: %zu bytes (total: %zu bytes)\n", payload.size(), audio_buffer.size());
        } else if (opcode == WS_OPCODE_TEXT) {
            // Text message - could be a command or possibly audio data sent as text
            std::string message(payload.begin(), payload.end());
            LOG_INFO("Received text message: '%s' (size: %zu)\n", message.c_str(), payload.size());
            
            // Check if it's a command
            if (message == "process" || message == "ready") {
                if (audio_buffer.empty()) {
                    LOG_INFO("Warning: process command received but audio buffer is empty\n");
                    write_ws_frame(client_socket, "{\"type\":\"error\",\"message\":\"No audio data received\"}\n");
                } else {
                    LOG_INFO("Starting audio processing, buffer size: %zu bytes\n", audio_buffer.size());
                    
                    // Process audio (mutex is already locked)
                    bool success = process_audio_websocket(client_socket, audio_buffer);
                    
                    // Unlock mutex immediately after processing to allow next client
                    lock.unlock();
                    LOG_INFO("Processing complete, mutex unlocked for next client\n");
                    
                    if (!success) {
                        LOG_INFO("Audio processing failed\n");
                    }
                    
                    audio_buffer.clear();
                    audio_complete = false;
                }
            } else if (message == "close") {
                break;
            } else if (payload.size() > 100) {
                // Large text payload might be audio data sent as text (fallback)
                LOG_INFO("Large text payload detected (%zu bytes), treating as audio data\n", payload.size());
                audio_buffer.insert(audio_buffer.end(), payload.begin(), payload.end());
                audio_complete = true;
            }
        } else if (opcode == WS_OPCODE_CONT) {
            // Continuation frame - append to buffer
            audio_buffer.insert(audio_buffer.end(), payload.begin(), payload.end());
            fprintf(stderr, "Received continuation frame: %zu bytes (total: %zu bytes)\n", payload.size(), audio_buffer.size());
        } else {
            fprintf(stderr, "Unknown opcode: %d\n", opcode);
        }
    }
    
    close(client_socket);
}

// Parse command line arguments (simplified version)
bool parse_args(int argc, char ** argv, whisper_params & params) {
    for (int i = 1; i < argc; i++) {
        std::string arg = argv[i];
        
        if (arg == "-h" || arg == "--help") {
            fprintf(stderr, "Usage: %s [options]\n", argv[0]);
            fprintf(stderr, "Options:\n");
            fprintf(stderr, "  -m, --model FILE        Model file (default: %s)\n", params.model.c_str());
            fprintf(stderr, "  -t, --threads N         Number of threads (default: %d)\n", params.n_threads);
            fprintf(stderr, "  -p, --processors N      Number of processors (default: %d)\n", params.n_processors);
            fprintf(stderr, "  -l, --language LANG     Language (default: %s)\n", params.language.c_str());
            fprintf(stderr, "  -tr, --translate        Translate to English\n");
            fprintf(stderr, "  -nt, --no-timestamps    Don't print timestamps\n");
            fprintf(stderr, "  -v, --verbose           Verbose output\n");
            return false;
        } else if ((arg == "-m" || arg == "--model") && i + 1 < argc) {
            params.model = argv[++i];
        } else if ((arg == "-t" || arg == "--threads") && i + 1 < argc) {
            params.n_threads = std::stoi(argv[++i]);
        } else if ((arg == "-p" || arg == "--processors") && i + 1 < argc) {
            params.n_processors = std::stoi(argv[++i]);
        } else if ((arg == "-l" || arg == "--language") && i + 1 < argc) {
            params.language = argv[++i];
        } else if (arg == "-tr" || arg == "--translate") {
            params.translate = true;
        } else if (arg == "-nt" || arg == "--no-timestamps") {
            params.no_timestamps = true;
        } else if (arg == "-v" || arg == "--verbose") {
            params.verbose = true;
        }
    }
    return true;
}

int main(int argc, char ** argv) {
    LOG_INFO("whisper-worker: WebSocket server for audio transcription\n");
    
    // Parse command line arguments
    if (!parse_args(argc, argv, g_params)) {
        return 1;
    }
    
    // Load model into memory
    LOG_INFO("Loading model: %s\n", g_params.model.c_str());
    
    whisper_context_params cparams = whisper_context_default_params();
    cparams.use_gpu = g_params.use_gpu;
    
    g_ctx = whisper_init_from_file_with_params(g_params.model.c_str(), cparams);
    
    if (g_ctx == nullptr) {
        LOG_INFO("error: failed to initialize whisper context\n");
        return 1;
    }
    
    // Initialize OpenVINO encoder if available
    whisper_ctx_init_openvino_encoder(g_ctx, nullptr, g_params.openvino_encode_device.c_str(), nullptr);
    
    LOG_INFO("Model loaded successfully\n");
    
    // Create socket
    int server_socket = socket(AF_INET, SOCK_STREAM, 0);
    if (server_socket < 0) {
        LOG_INFO("error: failed to create socket\n");
        whisper_free(g_ctx);
        return 1;
    }
    
    // Set socket options
    int opt = 1;
    setsockopt(server_socket, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
    
    // Bind socket
    struct sockaddr_in server_addr;
    memset(&server_addr, 0, sizeof(server_addr));
    server_addr.sin_family = AF_INET;
    server_addr.sin_addr.s_addr = INADDR_ANY;
    server_addr.sin_port = htons(WHISPER_WORKER_PORT);
    
    if (bind(server_socket, (struct sockaddr*)&server_addr, sizeof(server_addr)) < 0) {
        LOG_INFO("error: failed to bind socket to port %d\n", WHISPER_WORKER_PORT);
        close(server_socket);
        whisper_free(g_ctx);
        return 1;
    }
    
    // Listen
    if (listen(server_socket, 1) < 0) {
        LOG_INFO("error: failed to listen on socket\n");
        close(server_socket);
        whisper_free(g_ctx);
        return 1;
    }
    
    LOG_INFO("WebSocket server listening on port %d\n", WHISPER_WORKER_PORT);
    LOG_INFO("Ready to accept connections (one at a time)\n");
    
    // Accept connections (one at a time)
    while (true) {
        struct sockaddr_in client_addr;
        socklen_t client_len = sizeof(client_addr);
        
        int client_socket = accept(server_socket, (struct sockaddr*)&client_addr, &client_len);
        if (client_socket < 0) {
            continue;
        }
        
        LOG_INFO("New connection from %s:%d\n", 
                inet_ntoa(client_addr.sin_addr), ntohs(client_addr.sin_port));
        
        // Handle WebSocket connection (blocks until connection closes)
        handle_websocket_connection(client_socket);
        
        LOG_INFO("Connection closed\n");
    }
    
    close(server_socket);
    whisper_free(g_ctx);
    
    return 0;
}

