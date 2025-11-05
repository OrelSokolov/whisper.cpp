use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use log::{info, warn, error};
use std::path::PathBuf;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::types::SegmentMessage;

pub struct WhisperClient {
    host: String,
    port: u16,
}

impl WhisperClient {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
        }
    }

    pub async fn transcribe_file(
        &self,
        audio_file: &PathBuf,
        no_timestamps: bool,
    ) -> Result<Vec<SegmentMessage>> {
        let ws_url = format!("ws://{}:{}", self.host, self.port);
        
        info!("Connecting to {}...", ws_url);
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .context("Failed to connect to WebSocket server")?;

        info!("✓ Connected to Whisper server");

        let (mut write, mut read) = ws_stream.split();

        // Read audio file
        info!("Reading audio file: {}", audio_file.display());
        let audio_data = std::fs::read(audio_file)
            .context("Failed to read audio file")?;
        
        info!("Audio file size: {} bytes", audio_data.len());

        // Send audio data as binary
        info!("Sending audio data...");
        write.send(Message::Binary(audio_data))
            .await
            .context("Failed to send audio data")?;

        // Small delay before sending process command
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Send process command
        write.send(Message::Text("process".to_string()))
            .await
            .context("Failed to send process command")?;

        info!("✓ Audio sent, waiting for transcription...");
        println!("{}", "-".repeat(50));

        let mut segments = Vec::new();
        let mut last_progress = 0.0;

        // Receive messages
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Try to parse as JSON
                    match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(json) => {
                            let msg_type = json["type"].as_str().unwrap_or("");
                            
                            match msg_type {
                                "segment" => {
                                    let segment: SegmentMessage = serde_json::from_value(json)
                                        .context("Failed to parse segment message")?;
                                    
                                    // Display segment
                                    if no_timestamps {
                                        println!("{}", segment.text);
                                    } else {
                                        if let (Some(start), Some(end)) = (segment.start, segment.end) {
                                            let progress_str = if let Some(progress) = segment.progress {
                                                if progress - last_progress >= 1.0 || progress >= 99.0 {
                                                    last_progress = progress;
                                                    format!(" [{:.1}%]", progress)
                                                } else {
                                                    String::new()
                                                }
                                            } else {
                                                String::new()
                                            };
                                            
                                            let eta_str = if let Some(eta) = segment.eta {
                                                format!(" ETA: {}s", eta as i32)
                                            } else {
                                                String::new()
                                            };
                                            
                                            println!("[{:.2}s - {:.2}s]{}{} {}", 
                                                start, end, progress_str, eta_str, segment.text);
                                        } else {
                                            println!("{}", segment.text);
                                        }
                                    }
                                    
                                    segments.push(segment);
                                }
                                "status" => {
                                    let message = json["message"].as_str().unwrap_or("");
                                    info!("Status: {}", message);
                                }
                                "complete" => {
                                    println!("{}", "-".repeat(50));
                                    info!("✓ Transcription complete");
                                    info!("Total segments: {}", segments.len());
                                    break;
                                }
                                "error" => {
                                    let message = json["message"].as_str().unwrap_or("Unknown error");
                                    error!("✗ Error: {}", message);
                                    return Err(anyhow::anyhow!("Server error: {}", message));
                                }
                                _ => {
                                    warn!("Unknown message type: {}", msg_type);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse JSON: {} - {}", e, text);
                        }
                    }
                }
                Ok(Message::Binary(_)) => {
                    // Ignore binary messages
                }
                Ok(Message::Close(_)) => {
                    info!("Connection closed by server");
                    break;
                }
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
                    // Ignore ping/pong
                }
                Ok(Message::Frame(_)) => {
                    // Ignore frames
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        Ok(segments)
    }
}

