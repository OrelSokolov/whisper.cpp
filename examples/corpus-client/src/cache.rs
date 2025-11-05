use anyhow::{Context, Result};
use log::info;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;

use crate::types::TimestampsFile;

/// Вычисляет MD5 хэш файла
pub fn calculate_file_hash(file_path: &Path) -> Result<String> {
    let mut file = fs::File::open(file_path)
        .context("Failed to open file for hashing")?;
    
    let mut hasher = md5::Context::new();
    let mut buffer = [0u8; 8192];
    
    loop {
        let n = file.read(&mut buffer)
            .context("Failed to read file for hashing")?;
        if n == 0 {
            break;
        }
        hasher.consume(&buffer[..n]);
    }
    
    let digest = hasher.compute();
    Ok(format!("{:x}", digest))
}

/// Получает путь к директории кэша
fn get_cache_dir() -> Result<PathBuf> {
    // Используем .cache в текущей директории для простоты
    let cache_dir = PathBuf::from(".cache/corpus-client");
    fs::create_dir_all(&cache_dir)
        .context("Failed to create cache directory")?;
    Ok(cache_dir)
}

/// Получает путь к файлу кэша для данного хэша
fn get_cache_file_path(hash: &str) -> Result<PathBuf> {
    let cache_dir = get_cache_dir()?;
    Ok(cache_dir.join(format!("{}.json", hash)))
}

/// Загружает результаты транскрипции из кэша
pub fn load_from_cache(audio_file: &Path) -> Option<TimestampsFile> {
    // Вычисляем хэш файла
    let hash = match calculate_file_hash(audio_file) {
        Ok(h) => h,
        Err(e) => {
            info!("Failed to calculate file hash: {}", e);
            return None;
        }
    };
    
    // Получаем путь к файлу кэша
    let cache_file = match get_cache_file_path(&hash) {
        Ok(p) => p,
        Err(e) => {
            info!("Failed to get cache file path: {}", e);
            return None;
        }
    };
    
    // Проверяем существование файла кэша
    if !cache_file.exists() {
        info!("Cache miss (hash: {})", &hash[..8]);
        return None;
    }
    
    // Читаем и парсим кэш
    match fs::read_to_string(&cache_file) {
        Ok(json) => {
            match serde_json::from_str::<TimestampsFile>(&json) {
                Ok(timestamps) => {
                    info!("✓ Cache hit! Loaded {} segments from cache (hash: {})", 
                          timestamps.segments.len(), &hash[..8]);
                    Some(timestamps)
                }
                Err(e) => {
                    info!("Failed to parse cache file: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            info!("Failed to read cache file: {}", e);
            None
        }
    }
}

/// Сохраняет результаты транскрипции в кэш
pub fn save_to_cache(audio_file: &Path, timestamps: &TimestampsFile) -> Result<()> {
    // Вычисляем хэш файла
    let hash = calculate_file_hash(audio_file)?;
    
    // Получаем путь к файлу кэша
    let cache_file = get_cache_file_path(&hash)?;
    
    // Сериализуем в JSON
    let json = serde_json::to_string_pretty(timestamps)
        .context("Failed to serialize timestamps")?;
    
    // Сохраняем в кэш
    fs::write(&cache_file, json)
        .context("Failed to write cache file")?;
    
    info!("✓ Saved to cache: {} (hash: {})", cache_file.display(), &hash[..8]);
    
    Ok(())
}

/// Очищает весь кэш
pub fn clear_cache() -> Result<()> {
    let cache_dir = get_cache_dir()?;
    
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)
            .context("Failed to remove cache directory")?;
        fs::create_dir_all(&cache_dir)
            .context("Failed to recreate cache directory")?;
        info!("✓ Cache cleared");
    }
    
    Ok(())
}

/// Показывает статистику кэша
pub fn cache_stats() -> Result<()> {
    let cache_dir = get_cache_dir()?;
    
    if !cache_dir.exists() {
        println!("Cache is empty");
        return Ok(());
    }
    
    let entries = fs::read_dir(&cache_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
        .collect::<Vec<_>>();
    
    println!("Cache directory: {}", cache_dir.display());
    println!("Cached transcriptions: {}", entries.len());
    
    let mut total_size = 0u64;
    for entry in &entries {
        if let Ok(metadata) = entry.metadata() {
            total_size += metadata.len();
        }
    }
    
    println!("Total cache size: {:.2} MB", total_size as f64 / 1024.0 / 1024.0);
    
    Ok(())
}

