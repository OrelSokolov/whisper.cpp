use serde::{Deserialize, Serialize};
use std::default::Default;

#[derive(Debug, Clone)]
pub struct WhisperParams {
    pub n_threads: i32,
    pub n_processors: i32,
    pub offset_t_ms: i32,
    pub offset_n: i32,
    pub duration_ms: i32,
    pub progress_step: i32,
    pub max_context: i32,
    pub max_len: i32,
    pub best_of: i32,
    pub beam_size: i32,
    pub audio_ctx: i32,
    
    pub word_thold: f32,
    pub entropy_thold: f32,
    pub logprob_thold: f32,
    pub no_speech_thold: f32,
    pub grammar_penalty: f32,
    pub temperature: f32,
    pub temperature_inc: f32,
    
    pub debug_mode: bool,
    pub translate: bool,
    pub detect_language: bool,
    pub diarize: bool,
    pub tinydiarize: bool,
    pub split_on_word: bool,
    pub no_context: bool,
    pub no_fallback: bool,
    pub print_special: bool,
    pub print_colors: bool,
    pub print_confidence: bool,
    pub print_progress: bool,
    pub no_timestamps: bool,
    pub log_score: bool,
    pub use_gpu: bool, // Default: true (enable GPU by default)
    pub flash_attn: bool,
    pub suppress_nst: bool,
    pub verbose: bool,
    pub carry_initial_prompt: bool,
    pub merge_timestamps: bool,
    
    pub language: String,
    pub prompt: String,
    pub model: String,
    pub grammar: String,
    pub grammar_rule: String,
    pub suppress_regex: String,
    pub openvino_encode_device: String,
    pub dtw: String,
}

impl Default for WhisperParams {
    fn default() -> Self {
        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get() as i32)
            .unwrap_or(4)
            .min(4);
        
        Self {
            n_threads: num_threads,
            n_processors: 1,
            offset_t_ms: 0,
            offset_n: 0,
            duration_ms: 0,
            progress_step: 5,
            max_context: -1,
            max_len: 0,
            best_of: 5,
            beam_size: 5,
            audio_ctx: 0,
            
            word_thold: 0.01,
            entropy_thold: 2.40,
            logprob_thold: -1.00,
            no_speech_thold: 0.6,
            grammar_penalty: 100.0,
            temperature: 0.0,
            temperature_inc: 0.2,
            
            debug_mode: false,
            translate: false,
            detect_language: false,
            diarize: false,
            tinydiarize: false,
            split_on_word: false,
            no_context: false,
            no_fallback: false,
            print_special: false,
            print_colors: false,
            print_confidence: false,
            print_progress: false,
            no_timestamps: false,
            log_score: false,
            use_gpu: true,
            flash_attn: true,
            suppress_nst: false,
            verbose: false,
            carry_initial_prompt: false,
            merge_timestamps: false,
            
            language: "auto".to_string(),
            prompt: String::new(),
            model: "models/ggml-base.bin".to_string(),
            grammar: String::new(),
            grammar_rule: String::new(),
            suppress_regex: String::new(),
            openvino_encode_device: "CPU".to_string(),
            dtw: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub index: i32,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: String,
}

