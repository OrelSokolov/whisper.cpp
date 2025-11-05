use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CorpusConfig {
    pub host: String,
    pub port: u16,
    pub output_dir: PathBuf,
    pub format: String,
    pub min_duration: f64,
    pub max_duration: f64,
    pub sample_rate: Option<u32>,
    pub no_timestamps: bool,
    pub mono: bool,  // Force mono audio (for TTS)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampsFile {
    pub version: String,
    pub source: String,
    pub audio_file: String,
    pub total_duration: f64,
    pub language: String,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub index: usize,
    pub text: String,
    pub start: f64,
    pub end: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub index: i32,
    pub text: String,
    pub start: Option<f64>,
    pub end: Option<f64>,
    pub progress: Option<f64>,
    pub eta: Option<f64>,
    pub speaker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WsMessage {
    Segment(SegmentMessage),
    Status(StatusMessage),
    Complete(CompleteMessage),
    Error(ErrorMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub created_at: String,
    pub source_audio: String,
    pub source_duration: f64,
    pub total_segments: usize,
    pub accepted_segments: usize,
    pub rejected_segments: usize,
    pub min_duration: f64,
    pub max_duration: f64,
    pub avg_duration: f64,
    pub total_dataset_duration: f64,
    pub language: String,
    pub format: String,
    pub sample_rate: Option<u32>,
}

