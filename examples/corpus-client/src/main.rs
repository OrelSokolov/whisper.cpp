use anyhow::{Context, Result};
use clap::Parser;
use log::{info, warn, error};
use std::path::PathBuf;

mod client;
mod types;
mod audio_splitter;
mod audio_processor;
mod downloader;
mod merger;
mod cache;

use client::WhisperClient;
use types::CorpusConfig;

#[derive(Parser, Debug)]
#[command(name = "corpus-client")]
#[command(about = "Whisper Corpus Builder Client", long_about = None)]
struct Args {
    /// WebSocket server host
    #[arg(long, default_value = "localhost")]
    host: String,

    /// WebSocket server port
    #[arg(long, default_value = "8765")]
    port: u16,

    /// YouTube URL to download and process
    #[arg(long)]
    youtube_url: Option<String>,

    /// Audio file to process
    #[arg(long)]
    audio_file: Option<PathBuf>,

    /// Output directory for dataset
    #[arg(long, default_value = "./dataset")]
    output_dir: PathBuf,

    /// Timestamps JSON file (for split-only mode)
    #[arg(long)]
    timestamps: Option<PathBuf>,

    /// Split only mode (requires --audio-file and --timestamps)
    #[arg(long)]
    split_only: bool,

    /// Output audio format (mp3, wav, flac)
    #[arg(long, default_value = "wav")]
    format: String,

    /// Minimum segment duration in seconds
    #[arg(long, default_value = "0.5")]
    min_duration: f64,

    /// Maximum segment duration in seconds
    #[arg(long, default_value = "30.0")]
    max_duration: f64,

    /// Sample rate for output audio (default: 22050 for TTS)
    #[arg(long, default_value = "22050")]
    sample_rate: u32,

    /// Disable Piper TTS optimizations (mono, vowel hotfix)
    #[arg(long)]
    no_piper: bool,

    /// Disable transcription cache (force re-transcription)
    #[arg(long)]
    no_cache: bool,

    /// No timestamps mode (text only)
    #[arg(long)]
    no_timestamps: bool,

    /// Save timestamps to file
    #[arg(long)]
    output_timestamps: Option<PathBuf>,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    if args.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    info!("Whisper Corpus Builder Client v{}", env!("CARGO_PKG_VERSION"));

    // Piper TTS mode is enabled by default (disable with --no-piper)
    let piper_mode = !args.no_piper;
    
    if piper_mode {
        info!("Piper TTS mode: WAV 22050Hz mono, 0.5-30s segments, vowel hotfix enabled");
    } else {
        info!("Standard mode (no Piper optimizations)");
    }

    // Create config
    let config = CorpusConfig {
        host: args.host,
        port: args.port,
        output_dir: args.output_dir.clone(),
        format: args.format,
        min_duration: args.min_duration,
        max_duration: args.max_duration,
        sample_rate: Some(args.sample_rate),
        no_timestamps: args.no_timestamps,
        mono: piper_mode,  // Force mono for Piper
    };

    // Determine workflow
    if args.split_only {
        // Split-only mode
        let audio_file = args.audio_file
            .ok_or_else(|| anyhow::anyhow!("--audio-file required in split-only mode"))?;
        let timestamps_file = args.timestamps
            .ok_or_else(|| anyhow::anyhow!("--timestamps required in split-only mode"))?;

        info!("Running in split-only mode");
        run_split_only(&audio_file, &timestamps_file, &config).await?;

    } else if let Some(youtube_url) = args.youtube_url {
        // Full pipeline: download + transcribe + split
        info!("Running full pipeline from YouTube URL");
        run_full_pipeline_youtube(&youtube_url, &config, args.no_cache).await?;

    } else if let Some(audio_file) = args.audio_file {
        // Transcribe + split existing audio file
        info!("Running pipeline from audio file");
        run_pipeline_from_audio(&audio_file, &config, args.output_timestamps.as_ref(), args.no_cache).await?;

    } else {
        return Err(anyhow::anyhow!(
            "Must provide either --youtube-url, --audio-file, or --split-only mode"
        ));
    }

    info!("✓ Corpus building complete!");
    Ok(())
}

async fn run_split_only(
    audio_file: &PathBuf,
    timestamps_file: &PathBuf,
    config: &CorpusConfig,
) -> Result<()> {
    info!("Loading timestamps from: {}", timestamps_file.display());
    let timestamps_data = std::fs::read_to_string(timestamps_file)
        .context("Failed to read timestamps file")?;
    let timestamps: types::TimestampsFile = serde_json::from_str(&timestamps_data)
        .context("Failed to parse timestamps JSON")?;

    info!("Splitting audio file: {}", audio_file.display());
    audio_splitter::split_audio(audio_file, &timestamps, config).await?;

    Ok(())
}

async fn run_full_pipeline_youtube(
    youtube_url: &str,
    config: &CorpusConfig,
    no_cache: bool,
) -> Result<()> {
    // Step 1: Download from YouTube
    info!("Step 1/3: Downloading from YouTube...");
    let audio_file = downloader::download_from_youtube(youtube_url, &config.output_dir).await?;

    // Step 2: Transcribe
    info!("Step 2/3: Transcribing audio...");
    let timestamps = transcribe_audio(&audio_file, config, no_cache).await?;

    // Step 3: Split audio
    info!("Step 3/3: Splitting audio into segments...");
    audio_splitter::split_audio(&audio_file, &timestamps, config).await?;

    Ok(())
}

async fn run_pipeline_from_audio(
    audio_file: &PathBuf,
    config: &CorpusConfig,
    output_timestamps: Option<&PathBuf>,
    no_cache: bool,
) -> Result<()> {
    // Step 1: Transcribe
    info!("Step 1/2: Transcribing audio...");
    let timestamps = transcribe_audio(audio_file, config, no_cache).await?;

    // Save timestamps if requested
    if let Some(output_path) = output_timestamps {
        info!("Saving timestamps to: {}", output_path.display());
        let json = serde_json::to_string_pretty(&timestamps)?;
        std::fs::write(output_path, json)?;
    }

    // Step 2: Split audio (if output_timestamps is None, we split; otherwise just save timestamps)
    if output_timestamps.is_none() {
        info!("Step 2/2: Splitting audio into segments...");
        audio_splitter::split_audio(audio_file, &timestamps, config).await?;
    }

    Ok(())
}

async fn transcribe_audio(
    audio_file: &PathBuf,
    config: &CorpusConfig,
    no_cache: bool,
) -> Result<types::TimestampsFile> {
    // Проверяем кэш (если не отключен)
    if !no_cache {
        if let Some(cached) = cache::load_from_cache(audio_file) {
            info!("Using cached transcription, skipping Whisper server");
            return Ok(cached);
        }
    }
    
    let client = WhisperClient::new(&config.host, config.port);
    
    info!("Connecting to Whisper server at {}:{}...", config.host, config.port);
    let segments = client.transcribe_file(audio_file, config.no_timestamps).await?;

    // Convert segments to timestamps format
    let audio_duration = audio_splitter::get_audio_duration(audio_file)?;
    
    let timestamps = types::TimestampsFile {
        version: "1.0".to_string(),
        source: "whisper.cpp".to_string(),
        audio_file: audio_file.to_string_lossy().to_string(),
        total_duration: audio_duration,
        language: detect_language(&segments),
        segments: segments.into_iter().map(|s| types::Segment {
            index: s.index as usize,
            text: s.text,
            start: s.start.unwrap_or(0.0),
            end: s.end.unwrap_or(0.0),
            confidence: None,
        }).collect(),
    };

    // Сохраняем в кэш (если не отключен)
    if !no_cache {
        if let Err(e) = cache::save_to_cache(audio_file, &timestamps) {
            warn!("Failed to save to cache: {}", e);
        }
    }

    Ok(timestamps)
}

fn detect_language(segments: &[types::SegmentMessage]) -> String {
    // Simple heuristic: detect Cyrillic characters
    for segment in segments {
        for ch in segment.text.chars() {
            if ('\u{0400}'..='\u{04FF}').contains(&ch) {
                return "ru".to_string();
            }
        }
    }
    "en".to_string()
}

