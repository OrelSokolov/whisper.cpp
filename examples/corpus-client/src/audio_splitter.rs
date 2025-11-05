use anyhow::{Context, Result};
use log::{info, debug};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use chrono::Utc;
use rayon::prelude::*;

use crate::types::{TimestampsFile, DatasetMetadata, CorpusConfig};

// Список гласных букв для определения необходимости добавления времени
// HOTFIX: добавляем 0.15 секунды к сегментам, заканчивающимся на гласную
const VOWELS_RU: &[char] = &['а', 'е', 'ё', 'и', 'о', 'у', 'ы', 'э', 'ю', 'я',
                              'А', 'Е', 'Ё', 'И', 'О', 'У', 'Ы', 'Э', 'Ю', 'Я'];
const VOWELS_EN: &[char] = &['a', 'e', 'i', 'o', 'u', 'y',
                              'A', 'E', 'I', 'O', 'U', 'Y'];
const VOWEL_HOTFIX_DURATION: f64 = 0.15;  // Дополнительное время для сегментов на гласных (секунды)

/// Проверяет, заканчивается ли текст на гласную букву (до знака препинания)
fn ends_with_vowel(text: &str) -> bool {
    let text = text.trim();
    
    // Убираем знаки препинания с конца
    let text_without_punct = text.trim_end_matches(|c: char| {
        c == '.' || c == '!' || c == '?' || c == ',' || c == ';' || 
        c == ':' || c == '…' || c == '—' || c == '-' || c.is_whitespace()
    });
    
    if text_without_punct.is_empty() {
        return false;
    }
    
    // Получаем последний символ
    if let Some(last_char) = text_without_punct.chars().last() {
        let is_vowel = VOWELS_RU.contains(&last_char) || VOWELS_EN.contains(&last_char);
        if is_vowel {
            debug!("Text '{}' ends with vowel '{}'", text_without_punct, last_char);
        }
        is_vowel
    } else {
        false
    }
}

pub fn get_audio_duration(audio_file: &PathBuf) -> Result<f64> {
    let output = Command::new("ffprobe")
        .arg("-v").arg("error")
        .arg("-show_entries").arg("format=duration")
        .arg("-of").arg("default=noprint_wrappers=1:nokey=1")
        .arg(audio_file)
        .output()
        .context("Failed to run ffprobe. Make sure ffmpeg is installed.")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("ffprobe failed"));
    }

    let duration_str = String::from_utf8_lossy(&output.stdout);
    let duration: f64 = duration_str.trim().parse()
        .context("Failed to parse audio duration")?;

    Ok(duration)
}

pub async fn split_audio(
    audio_file: &PathBuf,
    timestamps: &TimestampsFile,
    config: &CorpusConfig,
) -> Result<()> {
    info!("Creating output directories...");
    let segments_dir = config.output_dir.join("wavs");
    let rejected_dir = config.output_dir.join("rejected");
    
    std::fs::create_dir_all(&segments_dir)
        .context("Failed to create wavs directory")?;
    std::fs::create_dir_all(&rejected_dir)
        .context("Failed to create rejected directory")?;

    // Настройка rayon thread pool
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(12);
    
    info!("Using {} threads for parallel audio splitting", num_threads);
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .context("Failed to build thread pool")?
        .install(|| {
            process_segments_parallel(audio_file, timestamps, config, &segments_dir, &rejected_dir)
        })
}

fn process_segments_parallel(
    audio_file: &PathBuf,
    timestamps: &TimestampsFile,
    config: &CorpusConfig,
    segments_dir: &PathBuf,
    _rejected_dir: &PathBuf,
) -> Result<()> {
    let accepted_count = Arc::new(AtomicUsize::new(0));
    let rejected_count = Arc::new(AtomicUsize::new(0));
    let processed_count = Arc::new(AtomicUsize::new(0));
    let total_segments = timestamps.segments.len();

    info!("Processing {} segments in parallel...", total_segments);

    // Собираем результаты обработки для подсчета статистики
    let results: Vec<_> = timestamps.segments
        .par_iter()
        .enumerate()
        .map(|(idx, segment)| {
            let segment_num = idx + 1;
            
            // HOTFIX: Если сегмент заканчивается на гласную, добавляем время
            let end_time = if ends_with_vowel(&segment.text) {
                let adjusted = segment.end + VOWEL_HOTFIX_DURATION;
                debug!("Segment {}: '{}' ends with vowel, adjusting end time {} -> {} (+{}s)", 
                       segment_num, segment.text.trim(), segment.end, adjusted, VOWEL_HOTFIX_DURATION);
                adjusted
            } else {
                segment.end
            };
            
            let duration = end_time - segment.start;
            
            // Filter by duration
            if duration < config.min_duration || duration > config.max_duration {
                debug!("Segment {} rejected: duration {:.2}s out of range [{:.1}, {:.1}]",
                    segment_num, duration, config.min_duration, config.max_duration);
                rejected_count.fetch_add(1, Ordering::Relaxed);
                return Err(anyhow::anyhow!("Duration out of range"));
            }

            // Filter by text length
            let text = segment.text.trim();
            if text.len() < 5 {
                debug!("Segment {} rejected: text too short ({})", segment_num, text.len());
                rejected_count.fetch_add(1, Ordering::Relaxed);
                return Err(anyhow::anyhow!("Text too short"));
            }

            // Filter music/sound effects markers
            if text.contains("♪") || text.contains("[") || text.contains("]") {
                debug!("Segment {} rejected: contains music/sound markers", segment_num);
                rejected_count.fetch_add(1, Ordering::Relaxed);
                return Err(anyhow::anyhow!("Contains markers"));
            }

            // Extract audio segment using ffmpeg
            let output_filename = format!("{:06}.{}", segment_num, config.format);
            let output_path = segments_dir.join(&output_filename);
            let text_filename = format!("{:06}.txt", segment_num);
            let text_path = segments_dir.join(&text_filename);

            // Build ffmpeg command
            let mut ffmpeg_cmd = Command::new("ffmpeg");
            ffmpeg_cmd
                .arg("-y")  // Overwrite output files
                .arg("-i").arg(audio_file)
                .arg("-ss").arg(format!("{:.3}", segment.start))
                .arg("-to").arg(format!("{:.3}", end_time))
                .arg("-acodec");

            // Set codec based on format
            match config.format.as_str() {
                "mp3" => {
                    ffmpeg_cmd.arg("libmp3lame").arg("-b:a").arg("128k");
                }
                "wav" => {
                    ffmpeg_cmd.arg("pcm_s16le");
                    if let Some(sr) = config.sample_rate {
                        ffmpeg_cmd.arg("-ar").arg(sr.to_string());
                    }
                }
                "flac" => {
                    ffmpeg_cmd.arg("flac");
                }
                _ => {
                    return Err(anyhow::anyhow!("Unsupported format: {}", config.format));
                }
            }

            // Set sample rate if not WAV (already set for WAV above)
            if let Some(sr) = config.sample_rate {
                if config.format != "wav" {
                    ffmpeg_cmd.arg("-ar").arg(sr.to_string());
                }
            }

            // Force mono if requested (for TTS)
            if config.mono {
                ffmpeg_cmd.arg("-ac").arg("1");
            }

            ffmpeg_cmd
                .arg("-loglevel").arg("error")
                .arg(&output_path);

            // Execute ffmpeg
            let output = ffmpeg_cmd.output()
                .context("Failed to execute ffmpeg")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                debug!("Segment {} failed to extract: {}", segment_num, stderr);
                rejected_count.fetch_add(1, Ordering::Relaxed);
                return Err(anyhow::anyhow!("ffmpeg failed"));
            }

            // Write text file
            std::fs::write(&text_path, format!("{}\n", text))
                .context("Failed to write text file")?;

            accepted_count.fetch_add(1, Ordering::Relaxed);
            
            // Progress update every 10 segments
            let processed = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
            if processed % 10 == 0 {
                let accepted = accepted_count.load(Ordering::Relaxed);
                let rejected = rejected_count.load(Ordering::Relaxed);
                info!("Processed {}/{} segments... (accepted: {}, rejected: {})",
                    processed, total_segments, accepted, rejected);
            }

            Ok(duration)
        })
        .collect();

    // Calculate statistics
    let final_accepted = accepted_count.load(Ordering::Relaxed);
    let final_rejected = rejected_count.load(Ordering::Relaxed);
    let mut total_duration = 0.0;
    let mut durations = Vec::new();

    for result in results {
        if let Ok(duration) = result {
            total_duration += duration;
            durations.push(duration);
        }
    }

    info!("Extraction complete!");
    info!("  Accepted: {}", final_accepted);
    info!("  Rejected: {}", final_rejected);
    info!("  Total dataset duration: {:.2}s ({:.2} minutes)",
        total_duration, total_duration / 60.0);

    // Calculate average duration
    let avg_duration = if !durations.is_empty() {
        durations.iter().sum::<f64>() / durations.len() as f64
    } else {
        0.0
    };

    // Create metadata
    let metadata = DatasetMetadata {
        created_at: Utc::now().to_rfc3339(),
        source_audio: audio_file.to_string_lossy().to_string(),
        source_duration: timestamps.total_duration,
        total_segments: timestamps.segments.len(),
        accepted_segments: final_accepted,
        rejected_segments: final_rejected,
        min_duration: config.min_duration,
        max_duration: config.max_duration,
        avg_duration,
        total_dataset_duration: total_duration,
        language: timestamps.language.clone(),
        format: config.format.clone(),
        sample_rate: config.sample_rate,
    };

    // Save metadata.json
    let metadata_path = config.output_dir.join("metadata.json");
    let metadata_json = serde_json::to_string_pretty(&metadata)?;
    std::fs::write(&metadata_path, metadata_json)
        .context("Failed to write metadata.json")?;

    info!("✓ Metadata saved to: {}", metadata_path.display());

    // Generate metadata.csv for Piper TTS
    generate_metadata_csv(&segments_dir, &config.output_dir)?;

    Ok(())
}

/// Generate metadata.csv for Piper TTS
fn generate_metadata_csv(segments_dir: &PathBuf, output_dir: &PathBuf) -> Result<()> {
    use std::fs::File;
    use std::io::Write;
    
    let csv_path = output_dir.join("metadata.csv");
    let mut csv_file = File::create(&csv_path)
        .context("Failed to create metadata.csv")?;
    
    // Get all wav/mp3/flac files and corresponding txt files
    let mut entries = Vec::new();
    
    if let Ok(dir_entries) = std::fs::read_dir(segments_dir) {
        for entry in dir_entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "wav" || ext == "mp3" || ext == "flac" {
                    let txt_path = path.with_extension("txt");
                    if txt_path.exists() {
                        if let Ok(text) = std::fs::read_to_string(&txt_path) {
                            let filename = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string();
                            let text = text.trim().to_string();
                            entries.push((filename, text));
                        }
                    }
                }
            }
        }
    }
    
    // Sort by filename
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    
    let entries_count = entries.len();
    
    // Write CSV
    for (filename, text) in &entries {
        writeln!(csv_file, "{}|{}", filename, text)
            .context("Failed to write to metadata.csv")?;
    }
    
    info!("✓ Piper metadata.csv created with {} entries", entries_count);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ends_with_vowel_russian() {
        // Русские гласные
        assert!(ends_with_vowel("Привет, как дела."));  // а
        assert!(ends_with_vowel("Хорошо."));            // о
        assert!(ends_with_vowel("Сегодня мы."));        // ы
        assert!(ends_with_vowel("Я помню."));           // ю
        
        // Не гласные
        assert!(!ends_with_vowel("Это пример."));       // р - не гласная
        assert!(!ends_with_vowel("Привет мир."));       // р - не гласная
        
        // С восклицательным знаком
        assert!(ends_with_vowel("Привет, дела!"));      // а
        assert!(ends_with_vowel("Хорошо!"));            // о
        
        // С вопросительным знаком
        assert!(ends_with_vowel("Как дела?"));          // а
        
        // С многоточием
        assert!(!ends_with_vowel("Может быть…"));       // ь - не гласная
        
        // С запятой
        assert!(ends_with_vowel("Привет, дела,"));      // а
    }

    #[test]
    fn test_ends_with_vowel_english() {
        // Английские гласные
        assert!(ends_with_vowel("Hello there."));       // e
        assert!(ends_with_vowel("How are you."));       // u
        assert!(ends_with_vowel("I see."));             // e
        assert!(ends_with_vowel("Today."));             // y
        
        // Не гласные
        assert!(!ends_with_vowel("Hello world."));      // d
        assert!(!ends_with_vowel("Good morning."));     // g
    }

    #[test]
    fn test_ends_with_vowel_mixed() {
        // Заглавные буквы
        assert!(ends_with_vowel("ДЕЛА."));              // А
        assert!(ends_with_vowel("YOU."));               // U
        
        // Пустые строки
        assert!(!ends_with_vowel(""));
        assert!(!ends_with_vowel("   "));
        assert!(!ends_with_vowel("."));
        assert!(!ends_with_vowel("..."));
    }

    #[test]
    fn test_ends_with_vowel_punctuation_only() {
        // Только знаки препинания
        assert!(!ends_with_vowel("..."));
        assert!(!ends_with_vowel("!!!"));
        assert!(!ends_with_vowel("???"));
        assert!(!ends_with_vowel("—"));
    }
}

