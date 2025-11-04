use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

#[repr(C)]
pub struct WhisperContext {
    _private: [u8; 0],
}

#[repr(C)]
pub struct WhisperState {
    _private: [u8; 0],
}

// Callback types
pub type WhisperNewSegmentCallback = extern "C" fn(*mut WhisperContext, *mut WhisperState, c_int, *mut c_void);
pub type WhisperProgressCallback = extern "C" fn(*mut WhisperContext, *mut WhisperState, c_int, *mut c_void);
pub type WhisperEncoderBeginCallback = extern "C" fn(*mut WhisperContext, *mut WhisperState, *mut c_void) -> bool;
pub type GgmlAbortCallback = extern "C" fn(*mut c_void) -> bool;
pub type WhisperLogitsFilterCallback = extern "C" fn(*mut WhisperContext, *mut WhisperState, *const c_void, c_int, *mut f32, *mut c_void);

#[repr(C)]
pub struct WhisperFullParams {
    pub strategy: WhisperSamplingStrategy,
    pub n_threads: c_int,
    pub n_max_text_ctx: c_int,
    pub offset_ms: c_int,
    pub duration_ms: c_int,
    pub translate: bool,
    pub no_context: bool,
    pub no_timestamps: bool,
    pub single_segment: bool,
    pub print_special: bool,
    pub print_progress: bool,
    pub print_realtime: bool,
    pub print_timestamps: bool,
    pub token_timestamps: bool,
    pub thold_pt: f32,
    pub thold_ptsum: f32,
    pub max_len: c_int,
    pub split_on_word: bool,
    pub max_tokens: c_int,
    pub debug_mode: bool,
    pub audio_ctx: c_int,
    pub tdrz_enable: bool,
    pub suppress_regex: *const c_char,
    pub initial_prompt: *const c_char,
    pub carry_initial_prompt: bool,
    pub prompt_tokens: *const i32,
    pub prompt_n_tokens: c_int,
    pub language: *const c_char,
    pub detect_language: bool,
    pub suppress_blank: bool,
    pub suppress_nst: bool,
    pub temperature: f32,
    pub max_initial_ts: f32,
    pub length_penalty: f32,
    pub temperature_inc: f32,
    pub entropy_thold: f32,
    pub logprob_thold: f32,
    pub no_speech_thold: f32,
    pub greedy: WhisperGreedyParams,
    pub beam_search: WhisperBeamSearchParams,
    pub new_segment_callback: WhisperNewSegmentCallback,
    pub new_segment_callback_user_data: *mut c_void,
    pub progress_callback: WhisperProgressCallback,
    pub progress_callback_user_data: *mut c_void,
    pub encoder_begin_callback: WhisperEncoderBeginCallback,
    pub encoder_begin_callback_user_data: *mut c_void,
    pub abort_callback: GgmlAbortCallback,
    pub abort_callback_user_data: *mut c_void,
    pub logits_filter_callback: WhisperLogitsFilterCallback,
    pub logits_filter_callback_user_data: *mut c_void,
    pub grammar_rules: *const *const c_void, // Simplified - full type is complex
    pub n_grammar_rules: usize,
    pub i_start_rule: usize,
    pub grammar_penalty: f32,
    pub vad: bool,
    pub vad_model_path: *const c_char,
    // vad_params structure
    pub vad_params_threshold: f32,
    pub vad_params_min_speech_duration_ms: c_int,
    pub vad_params_min_silence_duration_ms: c_int,
    pub vad_params_max_speech_duration_s: f32,
    pub vad_params_speech_pad_ms: c_int,
    pub vad_params_samples_overlap: f32,
}

// Alignment heads preset enum
#[repr(C)]
pub enum WhisperAlignmentHeadsPreset {
    None = 0,
    NTopMost = 1,
}

#[repr(C)]
pub struct WhisperContextParams {
    pub use_gpu: bool,
    pub flash_attn: bool,
    pub gpu_device: c_int,
    pub dtw_token_timestamps: bool,
    pub dtw_aheads_preset: WhisperAlignmentHeadsPreset,
    pub dtw_n_top: c_int,
    // Additional fields may exist, but these are the main ones for GPU
}

#[repr(C)]
pub enum WhisperSamplingStrategy {
    Greedy = 0,
    BeamSearch = 1,
}

#[repr(C)]
pub struct WhisperGreedyParams {
    pub best_of: c_int,
}

#[repr(C)]
pub struct WhisperBeamSearchParams {
    pub beam_size: c_int,
    pub patience: f32,
}

#[link(name = "whisper")]
extern "C" {
    pub fn whisper_context_default_params() -> WhisperContextParams;
    pub fn whisper_init_from_file_with_params(
        path: *const c_char,
        params: WhisperContextParams,
    ) -> *mut WhisperContext;
    pub fn whisper_free(ctx: *mut WhisperContext);
    pub fn whisper_full_default_params(
        strategy: WhisperSamplingStrategy,
    ) -> WhisperFullParams;
    pub fn whisper_full_parallel(
        ctx: *mut WhisperContext,
        params: WhisperFullParams,
        samples: *const f32,
        n_samples: c_int,
        n_processors: c_int,
    ) -> c_int;
    pub fn whisper_full_n_segments(ctx: *mut WhisperContext) -> c_int;
    pub fn whisper_full_get_segment_text(ctx: *mut WhisperContext, i_segment: c_int) -> *const c_char;
    pub fn whisper_full_get_segment_t0(ctx: *mut WhisperContext, i_segment: c_int) -> i64;
    pub fn whisper_full_get_segment_t1(ctx: *mut WhisperContext, i_segment: c_int) -> i64;
    pub fn whisper_is_multilingual(ctx: *mut WhisperContext) -> c_int;
    pub fn whisper_print_timings(ctx: *mut WhisperContext);
    pub fn whisper_reset_timings(ctx: *mut WhisperContext);
    pub fn whisper_ctx_init_openvino_encoder(
        ctx: *mut WhisperContext,
        model_path: *const c_char,
        device: *const c_char,
        cache_dir: *const c_char,
    ) -> c_int;
}

// Note: ggml_time_us is available from ggml library but we use std::time instead
// If needed, uncomment and link:
// #[link(name = "ggml")]
// extern "C" {
//     pub fn ggml_time_us() -> i64;
// }

// Helper functions to set whisper_full_params fields
// These will be implemented using unsafe code to access struct fields
#[allow(unused_variables)]
pub unsafe fn set_whisper_params(
    _wparams: *mut WhisperFullParams,
    _params: &crate::params::WhisperParams,
    _language: &str,
    _translate: bool,
) -> Result<(), String> {
    // Access struct fields using pointer arithmetic or direct memory access
    // This is unsafe and platform-dependent, but necessary for FFI
    // Note: This is a simplified version - full implementation would require
    // proper struct layout knowledge or C wrapper functions
    
    // For now, we'll use a C wrapper function approach
    // Create C wrapper functions in a separate C file
    
    Ok(())
}

#[allow(unused_variables)]
pub unsafe fn set_segment_callback(
    _wparams: *mut WhisperFullParams,
    _data: *mut c_void,
) -> Result<(), String> {
    // Set segment callback
    // This requires proper FFI setup with callback functions
    // For now, we'll handle segments manually after processing
    Ok(())
}

// Note: read_audio_data is a C++ function and cannot be called directly from Rust
// It should be accessed via a C wrapper function (see audio.rs)

#[link(name = "common")]
extern "C" {
    fn timestamp_to_sample_c(t: i64, n_samples: c_int, whisper_sample_rate: c_int) -> i64;
}

pub unsafe fn timestamp_to_sample(t: i64, n_samples: usize, whisper_sample_rate: usize) -> i64 {
    timestamp_to_sample_c(t, n_samples as c_int, whisper_sample_rate as c_int)
}

pub const WHISPER_SAMPLE_RATE: usize = 16000;

pub struct WhisperContextWrapper {
    ctx: *mut WhisperContext,
}

unsafe impl Send for WhisperContextWrapper {}
unsafe impl Sync for WhisperContextWrapper {}

impl WhisperContextWrapper {
    pub fn new(model_path: &str, use_gpu: bool) -> Result<Self, String> {
        unsafe {
            // Get default params - returns struct by value (like C++ version)
            let mut cparams = whisper_context_default_params();
            
            // Set GPU parameters (like C++ version does)
            cparams.use_gpu = use_gpu;
            cparams.gpu_device = 0; // Default GPU device
            cparams.flash_attn = true; // Enable flash attention by default
            
            let path_cstr = CString::new(model_path).map_err(|e| format!("Invalid path: {}", e))?;
            // Pass struct by value (matching C++ signature)
            let ctx = whisper_init_from_file_with_params(path_cstr.as_ptr(), cparams);
            
            if ctx.is_null() {
                return Err("Failed to initialize whisper context".to_string());
            }
            
            Ok(WhisperContextWrapper { ctx })
        }
    }
    
    pub fn init_openvino_encoder(&self, device: &str) -> Result<(), String> {
        unsafe {
            let device_cstr = CString::new(device).map_err(|e| format!("Invalid device: {}", e))?;
            let result = whisper_ctx_init_openvino_encoder(self.ctx, ptr::null(), device_cstr.as_ptr(), ptr::null());
            // Non-fatal: OpenVINO encoder initialization is optional
            if result != 0 {
                return Ok(()); // Continue even if OpenVINO init fails
            }
            Ok(())
        }
    }
    
    pub fn ctx(&self) -> *mut WhisperContext {
        self.ctx
    }
    
    pub fn is_multilingual(&self) -> bool {
        unsafe {
            whisper_is_multilingual(self.ctx) != 0
        }
    }
    
    pub fn n_segments(&self) -> i32 {
        unsafe {
            whisper_full_n_segments(self.ctx)
        }
    }
    
    pub fn get_segment_text(&self, i: i32) -> String {
        unsafe {
            let text_ptr = whisper_full_get_segment_text(self.ctx, i);
            if text_ptr.is_null() {
                return String::new();
            }
            CStr::from_ptr(text_ptr).to_string_lossy().into_owned()
        }
    }
    
    pub fn get_segment_t0(&self, i: i32) -> i64 {
        unsafe {
            whisper_full_get_segment_t0(self.ctx, i)
        }
    }
    
    pub fn get_segment_t1(&self, i: i32) -> i64 {
        unsafe {
            whisper_full_get_segment_t1(self.ctx, i)
        }
    }
    
    pub fn print_timings(&self) {
        unsafe {
            whisper_print_timings(self.ctx);
        }
    }
    
    pub fn reset_timings(&self) {
        unsafe {
            whisper_reset_timings(self.ctx);
        }
    }
}

impl Drop for WhisperContextWrapper {
    fn drop(&mut self) {
        unsafe {
            whisper_free(self.ctx);
        }
    }
}

