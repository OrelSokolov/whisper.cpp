use anyhow::{Context, Result};
use std::io::Cursor;
use symphonia::core::audio::{AudioBuffer, SampleBuffer};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use crate::whisper_ffi::{timestamp_to_sample, WHISPER_SAMPLE_RATE};

pub fn decode_audio_data(audio_data: &[u8]) -> Result<Vec<f32>> {
    // Create a media source stream from the audio data
    // We need to clone the data to satisfy 'static lifetime requirement
    let audio_data_owned = audio_data.to_vec();
    let mss = MediaSourceStream::new(Box::new(Cursor::new(audio_data_owned)), Default::default());
    
    // Create a hint to help the format registry guess the format
    let mut hint = Hint::new();
    // Try to detect format from data if possible
    
    // Use the default options for metadata and format
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();
    
    // Probe the media source
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .context("Failed to probe audio format")?;
    
    // Get the format reader
    let mut format = probed.format;
    
    // Find the first audio track and copy its parameters
    let track_params = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow::anyhow!("No audio track found"))?
        .codec_params
        .clone();
    
    // Create a decoder for the track
    let mut decoder = symphonia::default::get_codecs()
        .make(&track_params, &DecoderOptions::default())
        .context("Failed to create decoder")?;
    
    // Resample to WHISPER_SAMPLE_RATE if needed
    let track_sample_rate = track_params.sample_rate.unwrap_or(WHISPER_SAMPLE_RATE as u32);
    let needs_resample = track_sample_rate != WHISPER_SAMPLE_RATE as u32;
    let channels = track_params.channels.map(|c| c.count()).unwrap_or(1);
    
    let mut pcmf32 = Vec::new();
    
    // Decode all packets
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(_) => break, // End of stream or error
        };
        
        // Decode the packet
        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Convert decoded audio to f32 samples
                let spec = *decoded.spec();
                let duration = decoded.capacity() as u64;
                
                // Create a sample buffer to hold the decoded samples
                let mut sample_buffer = SampleBuffer::<f32>::new(duration, spec);
                sample_buffer.copy_interleaved_ref(decoded);
                
                let samples = sample_buffer.samples();
                
                if needs_resample && track_sample_rate != WHISPER_SAMPLE_RATE as u32 {
                    // Simple linear resampling (for better quality, use a proper resampler)
                    let ratio = track_sample_rate as f64 / WHISPER_SAMPLE_RATE as f64;
                    let new_len = (samples.len() as f64 / ratio) as usize;
                    let mut resampled = Vec::with_capacity(new_len);
                    
                    for i in 0..new_len {
                        let src_idx = (i as f64 * ratio) as usize;
                        if src_idx < samples.len() {
                            resampled.push(samples[src_idx]);
                        }
                    }
                    
                    pcmf32.extend_from_slice(&resampled);
                } else {
                    pcmf32.extend_from_slice(samples);
                }
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => {
                // Skip decode errors (may happen with some formats)
                continue;
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Decode error: {}", e));
            }
        }
    }
    
    if pcmf32.is_empty() {
        return Err(anyhow::anyhow!("No audio samples decoded"));
    }
    
    // Convert stereo to mono if needed (take average of channels)
    // Note: symphonia handles interleaved samples, so if stereo, samples are L, R, L, R, ...
    if channels > 1 {
        let mut mono = Vec::with_capacity(pcmf32.len() / channels);
        for chunk in pcmf32.chunks(channels) {
            let sum: f32 = chunk.iter().sum();
            mono.push(sum / channels as f32);
        }
        pcmf32 = mono;
    }
    
    Ok(pcmf32)
}

pub fn calculate_audio_duration(n_samples: usize) -> f32 {
    n_samples as f32 / WHISPER_SAMPLE_RATE as f32
}

pub fn estimate_diarization_speaker(
    pcmf32s: &[Vec<f32>],
    t0: i64,
    t1: i64,
    id_only: bool,
) -> String {
    if pcmf32s.len() < 2 {
        return if id_only { "?".to_string() } else { "(speaker ?)".to_string() };
    }
    
    let n_samples = pcmf32s[0].len();
    let is0 = unsafe { timestamp_to_sample(t0, n_samples, WHISPER_SAMPLE_RATE) };
    let is1 = unsafe { timestamp_to_sample(t1, n_samples, WHISPER_SAMPLE_RATE) };
    
    let mut energy0 = 0.0;
    let mut energy1 = 0.0;
    
    for j in is0..is1.min(n_samples as i64) {
        if j >= 0 && (j as usize) < pcmf32s[0].len() {
            energy0 += pcmf32s[0][j as usize].abs() as f64;
        }
        if j >= 0 && (j as usize) < pcmf32s[1].len() {
            energy1 += pcmf32s[1][j as usize].abs() as f64;
        }
    }
    
    let speaker = if energy0 > 1.1 * energy1 {
        "0"
    } else if energy1 > 1.1 * energy0 {
        "1"
    } else {
        "?"
    };
    
    if id_only {
        speaker.to_string()
    } else {
        format!("(speaker {})", speaker)
    }
}


