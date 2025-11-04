mod whisper_ffi;
mod websocket;
mod websocket_raw;
mod websocket_handshake;
mod audio;
mod params;

use anyhow::{Context, Result};
use clap::Parser;
use log::{info, error};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use whisper_ffi::WhisperContextWrapper;
use params::WhisperParams;

const WHISPER_WORKER_PORT: u16 = 8765;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Model file path
    #[arg(short, long, default_value = "models/ggml-base.bin")]
    model: String,
    
    /// Number of threads
    #[arg(short, long)]
    threads: Option<i32>,
    
    /// Number of processors
    #[arg(short, long, default_value_t = 1)]
    processors: i32,
    
    /// Language (default: auto)
    #[arg(short, long, default_value = "auto")]
    language: String,
    
    /// Translate to English
    #[arg(long)]
    translate: bool,
    
    /// Don't print timestamps
    #[arg(long)]
    no_timestamps: bool,
    
    /// Disable cross-segment context
    #[arg(long)]
    no_context: bool,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
    
    /// Port to listen on
    #[arg(long, default_value_t = WHISPER_WORKER_PORT)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    
    let args = Args::parse();
    
    info!("whisper-worker-rs: WebSocket server for audio transcription");
    
    // Initialize parameters
    let mut params = WhisperParams::default();
    params.model = args.model.clone();
    if let Some(threads) = args.threads {
        params.n_threads = threads;
    }
    params.n_processors = args.processors;
    params.language = args.language.clone();
    params.translate = args.translate;
    params.no_timestamps = args.no_timestamps;
    params.no_context = args.no_context;
    params.verbose = args.verbose;
    
    // Load model
    info!("Loading model: {}", params.model);
    let ctx = WhisperContextWrapper::new(&params.model, params.use_gpu)
        .map_err(|e| anyhow::anyhow!("Failed to initialize whisper context: {}", e))?;
    
    // Initialize OpenVINO encoder if available
    let _ = ctx.init_openvino_encoder(&params.openvino_encode_device);
    
    info!("Model loaded successfully");
    
    // Share context and params across tasks
    let ctx = Arc::new(ctx);
    let params = Arc::new(params);
    
    // Get port from environment variable or command line argument
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(args.port);
    
    // Create TCP listener
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .context("Failed to bind socket")?;
    info!("WebSocket server listening on port {}", port);
    info!("Ready to accept connections (one at a time)");
    
    // Accept connections (one at a time)
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("New connection from {}", addr);
                
                let ctx_clone = Arc::clone(&ctx);
                let params_clone = Arc::clone(&params);
                
                // Use a mutex to ensure only one connection processes at a time
                let connection_mutex = Arc::new(Mutex::new(()));
                let mutex_clone = Arc::clone(&connection_mutex);
                
                tokio::spawn(async move {
                    let _guard = mutex_clone.lock().await;
                    if let Err(e) = websocket::handle_websocket_connection(stream, ctx_clone, params_clone).await {
                        error!("WebSocket connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

