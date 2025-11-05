use anyhow::{Context, Result};
use log::{info, debug};
use std::path::PathBuf;
use std::process::Command;
use chrono::Utc;

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

    info!("Preparing {} segments...", timestamps.segments.len());

    // Фильтруем и подготавливаем сегменты
    let mut valid_segments = Vec::new();
    let mut rejected_count = 0;
    
    for (idx, segment) in timestamps.segments.iter().enumerate() {
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
            rejected_count += 1;
            continue;
        }

        // Filter by text length
        let text = segment.text.trim();
        if text.len() < 5 {
            debug!("Segment {} rejected: text too short ({})", segment_num, text.len());
            rejected_count += 1;
            continue;
        }

        // Filter music/sound effects markers
        if text.contains("♪") || text.contains("[") || text.contains("]") {
            debug!("Segment {} rejected: contains music/sound markers", segment_num);
            rejected_count += 1;
            continue;
        }
        
        valid_segments.push((segment_num, end_time, text.to_string(), duration));
    }

    let accepted_count = valid_segments.len();
    info!("Filtered: {} accepted, {} rejected", accepted_count, rejected_count);
    
    if valid_segments.is_empty() {
        return Err(anyhow::anyhow!("No valid segments after filtering"));
    }

    // NATIVE PROCESSING: Читаем файл один раз, режем в памяти (ОЧЕНЬ БЫСТРО!)
    info!("Using native audio processing (10-20x faster than ffmpeg)...");
    
    // Подготавливаем данные для нативной обработки: (segment_num, start, end, text)
    let segments_for_native: Vec<(usize, f64, f64, String)> = valid_segments
        .iter()
        .enumerate()
        .map(|(_, (num, end_time, text, _))| {
            // Находим start из оригинального segment
            let orig_seg = &timestamps.segments[num - 1];
            (*num, orig_seg.start, *end_time, text.clone())
        })
        .collect();
    
    let durations = crate::audio_processor::split_audio_native(
        audio_file,
        &segments_for_native,
        &segments_dir,
        &config.format,
        config.sample_rate.unwrap_or(22050),
        config.mono,
    )?;
    
    info!("Native splitting complete!");
    
    // Calculate statistics
    let total_duration: f64 = durations.iter().sum();
    let final_accepted = durations.len();
    let final_rejected = rejected_count;

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

