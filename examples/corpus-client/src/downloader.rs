use anyhow::{Context, Result};
use log::info;
use std::path::PathBuf;
use std::process::Command;

pub async fn download_from_youtube(url: &str, output_dir: &PathBuf) -> Result<PathBuf> {
    info!("Downloading from YouTube: {}", url);
    
    // Check if yt-dlp is installed
    let yt_dlp_check = Command::new("yt-dlp")
        .arg("--version")
        .output();
    
    if yt_dlp_check.is_err() {
        return Err(anyhow::anyhow!(
            "yt-dlp is not installed. Please install it: pip install yt-dlp"
        ));
    }

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)
        .context("Failed to create output directory")?;

    let output_template = output_dir.join("source_audio.%(ext)s");
    let output_template_str = output_template.to_string_lossy();

    info!("Running yt-dlp...");
    let output = Command::new("yt-dlp")
        .arg("-x")                          // Extract audio
        .arg("--audio-format").arg("mp3")   // Convert to MP3
        .arg("-o").arg(output_template_str.as_ref())
        .arg(url)
        .output()
        .context("Failed to execute yt-dlp")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("yt-dlp failed: {}", stderr));
    }

    let audio_file = output_dir.join("source_audio.mp3");
    
    if !audio_file.exists() {
        return Err(anyhow::anyhow!("Downloaded file not found: {}", audio_file.display()));
    }

    let file_size = std::fs::metadata(&audio_file)?.len();
    info!("✓ Downloaded: {} ({} bytes)", audio_file.display(), file_size);

    Ok(audio_file)
}

