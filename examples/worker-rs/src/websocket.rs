use anyhow::{Context, Result};
use log::{info, error, warn, debug};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use serde_json;
use crate::whisper_ffi::{WhisperContextWrapper, WhisperSamplingStrategy};
use crate::params::{WhisperParams, SegmentMessage, StatusMessage, CompleteMessage, ErrorMessage};
use crate::audio::{decode_audio_data, calculate_audio_duration};
use crate::websocket_raw::{read_ws_frame, write_ws_frame, WS_OPCODE_TEXT, WS_OPCODE_BINARY, WS_OPCODE_CLOSE, WS_OPCODE_PING, WS_OPCODE_PONG, WS_OPCODE_CONT};
use crate::websocket_handshake::perform_websocket_handshake;
use std::os::raw::c_int;

pub async fn handle_websocket_connection(
    mut stream: TcpStream,
    ctx: Arc<WhisperContextWrapper>,
    params: Arc<WhisperParams>,
) -> Result<()> {
    // Perform WebSocket handshake (like C++ version)
    perform_websocket_handshake(&mut stream).await
        .context("Failed to perform WebSocket handshake")?;
    
    info!("WebSocket connection established, waiting for data...");
    
    let mut audio_buffer = Vec::new();
    
    // Read frames (like C++ version with read_ws_frame)
    loop {
        match read_ws_frame(&mut stream).await {
            Ok(Some(frame)) => {
                info!("Received frame: opcode={}, payload_size={}", frame.opcode, frame.payload.len());
                
                match frame.opcode {
                    WS_OPCODE_CLOSE => {
                        info!("Received close frame");
                        break;
                    }
                    WS_OPCODE_PING => {
                        // Send pong response
                        write_ws_frame(&mut stream, &[], WS_OPCODE_PONG).await?;
                    }
                    WS_OPCODE_BINARY => {
                        // Accumulate audio data
                        audio_buffer.extend_from_slice(&frame.payload);
                        info!("Received binary data: {} bytes (total: {} bytes)", frame.payload.len(), audio_buffer.len());
                    }
                    WS_OPCODE_TEXT => {
                        // Text message - could be a command or possibly audio data sent as text
                        // In C++ version: std::string message(payload.begin(), payload.end());
                        // No UTF-8 validation - just copy bytes
                        let payload = &frame.payload;
                        info!("Received text message: size {} bytes", payload.len());
                        
                        // Try to decode as UTF-8 string for commands
                        match String::from_utf8(payload.clone()) {
                            Ok(text) => {
                                let text_trimmed = text.trim();
                                info!("Received text message: '{}' (size: {} bytes)", text_trimmed, payload.len());
                                
                                // Check if it's a command (like C++ version)
                                if text_trimmed == "process" || text_trimmed == "ready" {
                                    if audio_buffer.is_empty() {
                                        warn!("Warning: process command received but audio buffer is empty");
                                        let error_msg = ErrorMessage {
                                            msg_type: "error".to_string(),
                                            message: "No audio data received".to_string(),
                                        };
                                        let json = serde_json::to_string(&error_msg)?;
                                        write_ws_frame(&mut stream, json.as_bytes(), WS_OPCODE_TEXT).await?;
                                    } else {
                                        info!("Starting audio processing, buffer size: {} bytes", audio_buffer.len());
                                        
                                        // Process audio (takes and returns ownership of stream)
                                        match process_audio_websocket_raw(
                                            stream,
                                            ctx.clone(),
                                            &params,
                                            &audio_buffer,
                                        ).await {
                                            Ok(s) => stream = s,
                                            Err(e) => {
                                                error!("Audio processing failed: {}", e);
                                                break;
                                            }
                                        }
                                        
                                        audio_buffer.clear();
                                    }
                                } else if text_trimmed == "close" {
                                    break;
                                } else if payload.len() > 100 {
                                    // Large text payload might be audio data sent as text (fallback)
                                    // In C++ version: payload.size() > 100 -> treat as audio
                                    warn!("Large text payload detected ({} bytes), treating as audio data", payload.len());
                                    audio_buffer.extend_from_slice(payload);
                                }
                            }
                            Err(_) => {
                                // Invalid UTF-8 - treat as binary audio data (like C++ version)
                                warn!("Received text frame with invalid UTF-8 ({} bytes), treating as binary audio", payload.len());
                                audio_buffer.extend_from_slice(payload);
                            }
                        }
                    }
                    WS_OPCODE_CONT => {
                        // Continuation frame - append to buffer
                        audio_buffer.extend_from_slice(&frame.payload);
                        info!("Received continuation frame: {} bytes (total: {} bytes)", frame.payload.len(), audio_buffer.len());
                    }
                    _ => {
                        warn!("Unknown opcode: {}", frame.opcode);
                    }
                }
            }
            Ok(None) => {
                info!("Connection closed by client or read error");
                break;
            }
            Err(e) => {
                error!("Error reading WebSocket frame: {}", e);
                break;
            }
        }
    }
    
    info!("Connection closed");
    Ok(())
}

async fn process_audio_websocket_raw(
    stream: TcpStream,
    ctx: Arc<WhisperContextWrapper>,
    params: &WhisperParams,
    audio_data: &[u8],
) -> Result<TcpStream> {
    let mut stream = stream; // Make it mutable
    
    if audio_data.is_empty() {
        let error_msg = ErrorMessage {
            msg_type: "error".to_string(),
            message: "Empty audio data".to_string(),
        };
        let json = serde_json::to_string(&error_msg)?;
        write_ws_frame(&mut stream, json.as_bytes(), WS_OPCODE_TEXT).await?;
        return Err(anyhow::anyhow!("Empty audio data"));
    }
    
    // Decode audio from memory buffer
    let pcmf32 = decode_audio_data(audio_data)
        .context("Failed to decode audio")?;
    
    // Calculate and log audio duration
    let audio_duration_s = calculate_audio_duration(pcmf32.len());
    info!("Audio decoded successfully: {:.2} seconds ({:.2} minutes, {} samples)", 
          audio_duration_s, audio_duration_s / 60.0, pcmf32.len());
    
    // Check language settings
    let mut language = params.language.clone();
    let mut translate = params.translate;
    
    if !ctx.is_multilingual() {
        if language != "en" || translate {
            language = "en".to_string();
            translate = false;
        }
    }
    
    if params.detect_language {
        language = "auto".to_string();
    }
    
    // Capture start time for benchmark and ETA
    let t_start_process_us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;
    
    // Process audio using whisper (takes ownership of stream, sends status and segments, returns stream)
    let result = process_audio_with_whisper_raw(
        ctx.clone(),
        params,
        &pcmf32,
        audio_duration_s,
        t_start_process_us,
        stream,
        &language,
        translate,
    ).await;
    
    // Check result
    let _stream_back = match result {
        Ok(s) => s,
        Err(e) => return Err(e),
    };
    
    // Capture end time for benchmark
    let t_end_process_us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;
    
    // Print timing statistics
    info!("Processing complete, printing statistics:");
    ctx.print_timings();
    
    // Calculate and print realtime factor
    let total_time_s = (t_end_process_us - t_start_process_us) as f32 / 1_000_000.0;
    let realtime_factor = audio_duration_s / total_time_s;
    
    info!("");
    info!("benchmark_cli_factor: audio duration  = {:7.2} sec", audio_duration_s);
    info!("benchmark_cli_factor: total time      = {:7.2} ms", total_time_s * 1000.0);
    info!("benchmark_cli_factor: realtime factor = {:7.2}x", realtime_factor);
    
    // Return stream ownership
    Ok(_stream_back)
}

// Data structure for callback
struct CallbackData {
    sender: Arc<StdMutex<Option<mpsc::UnboundedSender<SegmentData>>>>,
    ctx_ptr: *mut crate::whisper_ffi::WhisperContext,
    audio_duration_s: f32,
    t_start_process_us: i64,
    no_timestamps: bool,
}

// Safety: CallbackData is only used within the blocking task and callback
// The pointer is managed carefully and never accessed concurrently
unsafe impl Send for CallbackData {}

// Wrapper for CallbackData pointer to make it Send
struct CallbackDataPtr(*mut CallbackData);
unsafe impl Send for CallbackDataPtr {}

#[derive(Debug, Clone)]
struct SegmentData {
    index: i32,
    text: String,
    t0: i64,
    t1: i64,
}

// Callback function called by whisper for each new segment
extern "C" fn segment_callback(
    ctx: *mut crate::whisper_ffi::WhisperContext,
    _state: *mut crate::whisper_ffi::WhisperState,
    n_new: c_int,
    user_data: *mut std::os::raw::c_void,
) {
    unsafe {
        let callback_data = &*(user_data as *const CallbackData);
        
        let n_segments = crate::whisper_ffi::whisper_full_n_segments(ctx);
        let s0 = n_segments - n_new;
        
        debug!("CALLBACK: n_new={}, total_segments={}, sending segments {} to {}", 
              n_new, n_segments, s0, n_segments - 1);
        
        // Send new segments through channel
        if let Ok(sender_guard) = callback_data.sender.lock() {
            if let Some(sender) = sender_guard.as_ref() {
                for i in s0..n_segments {
                    let text_ptr = crate::whisper_ffi::whisper_full_get_segment_text(ctx, i);
                    let text = if text_ptr.is_null() {
                        String::new()
                    } else {
                        std::ffi::CStr::from_ptr(text_ptr).to_string_lossy().into_owned()
                    };
                    
                    let t0 = crate::whisper_ffi::whisper_full_get_segment_t0(ctx, i);
                    let t1 = crate::whisper_ffi::whisper_full_get_segment_t1(ctx, i);
                    
                    let segment = SegmentData {
                        index: i,
                        text,
                        t0,
                        t1,
                    };
                    
                    debug!("CALLBACK: sending segment {} through channel", i);
                    // Send through channel (ignore error if receiver dropped)
                    let _ = sender.send(segment);
                }
            }
        }
    }
}

async fn process_audio_with_whisper_raw(
    ctx: Arc<WhisperContextWrapper>,
    params: &WhisperParams,
    pcmf32: &[f32],
    audio_duration_s: f32,
    t_start_process_us: i64,
    mut stream: TcpStream,
    _language: &str,
    _translate: bool,
) -> Result<TcpStream> {
    let n_processors = params.n_processors;
    let n_threads = params.n_threads;
    let beam_size = params.beam_size;
    let no_timestamps = params.no_timestamps;
    
    // Send status message first
    let status_msg = StatusMessage {
        msg_type: "status".to_string(),
        message: "Processing audio...".to_string(),
    };
    let json = serde_json::to_string(&status_msg)?;
    info!("Sending status message: {}", json);
    write_ws_frame(&mut stream, json.as_bytes(), WS_OPCODE_TEXT).await?;
    info!("Status message sent successfully");
    
    info!("Starting whisper_full_parallel with {} samples, {} processors", pcmf32.len(), n_processors);
    
    // Create channel for streaming segments from callback
    let (tx, mut rx) = mpsc::unbounded_channel::<SegmentData>();
    
    // Prepare callback data
    let callback_data = Box::new(CallbackData {
        sender: Arc::new(StdMutex::new(Some(tx))),
        ctx_ptr: ctx.ctx(),
        audio_duration_s,
        t_start_process_us,
        no_timestamps,
    });
    
    let callback_data_ptr = Box::into_raw(callback_data);
    
    // Cast pointer to usize (which is Send) to pass through spawn_blocking
    let callback_ptr_usize = callback_data_ptr as usize;
    
    info!("Setting up callback and calling whisper_full_parallel");
    
    // Wrap stream in Arc<Mutex> for sharing between callback task and this function
    let stream_arc = Arc::new(tokio::sync::Mutex::new(stream));
    let stream_clone = stream_arc.clone();
    
    // Clone params for async task
    let params_clone = params.clone();
    
    // Clone for async task that will stream results
    let stream_handle = tokio::task::spawn(async move {
        let mut count = 0;
        while let Some(segment) = rx.recv().await {
            count += 1;
            debug!("Streaming segment {}: {} chars", segment.index, segment.text.len());
            
            let mut stream_guard = stream_clone.lock().await;
            if let Err(e) = send_segment_data(&mut *stream_guard, segment, &params_clone, audio_duration_s, t_start_process_us).await {
                error!("Failed to send segment: {}", e);
            }
            drop(stream_guard); // Release lock immediately
        }
        debug!("Segment streaming complete, {} segments sent", count);
    });
    
    // Prepare data for spawn_blocking
    let ctx_clone = ctx.clone();
    let pcmf32_vec = pcmf32.to_vec();
    
    // Create "auto" language string for detection
    let lang_auto = std::ffi::CString::new("auto").unwrap();
    
    // Run whisper_full_parallel in blocking thread so callback can send data while processing
    let processing_handle = tokio::task::spawn_blocking(move || {
        unsafe {
            let strategy = if beam_size > 1 {
                WhisperSamplingStrategy::BeamSearch
            } else {
                WhisperSamplingStrategy::Greedy
            };
            
            info!("Getting default params for strategy");
            let mut wparams = crate::whisper_ffi::whisper_full_default_params(strategy);
            
            // Set all parameters (same as C++ version)
            wparams.print_realtime = false;
            wparams.print_progress = false;
            wparams.print_timestamps = !no_timestamps;
            wparams.print_special = false;
            wparams.no_context = false; // Keep context for better punctuation/capitalization
            wparams.single_segment = false;
            wparams.translate = false;
            wparams.no_timestamps = no_timestamps;
            wparams.language = lang_auto.as_ptr(); // Set to "auto" - whisper will auto-detect
            wparams.detect_language = false; // False when language is "auto" (C++ default)
            wparams.n_threads = n_threads; // Number of threads for processing
            wparams.offset_ms = 0;
            wparams.duration_ms = 0;
            wparams.token_timestamps = false;
            wparams.split_on_word = false;
            wparams.audio_ctx = 0;
            wparams.debug_mode = false;
            wparams.tdrz_enable = false;
            wparams.suppress_regex = std::ptr::null();
            wparams.initial_prompt = std::ptr::null();
            wparams.carry_initial_prompt = false;
            wparams.temperature = 0.0;
            wparams.temperature_inc = 0.2;
            wparams.entropy_thold = 2.40;
            wparams.logprob_thold = -1.00;
            wparams.no_speech_thold = 0.6;
            wparams.suppress_nst = false;
            wparams.suppress_blank = true;
            
            // Set callback
            wparams.new_segment_callback = segment_callback;
            wparams.new_segment_callback_user_data = callback_ptr_usize as *mut std::os::raw::c_void;
            
            info!("Callback set: callback={:?}, user_data={:?}", 
                  wparams.new_segment_callback as *const (), 
                  wparams.new_segment_callback_user_data);
            info!("Parameters: n_threads={}, detect_language={}, no_context={}, print_realtime={}", 
                  wparams.n_threads, wparams.detect_language, wparams.no_context, wparams.print_realtime);
            info!("Calling whisper_full_parallel with streaming callback in blocking thread...");
            let result = crate::whisper_ffi::whisper_full_parallel(
                ctx_clone.ctx(),
                wparams,
                pcmf32_vec.as_ptr(),
                pcmf32_vec.len() as i32,
                n_processors,
            );
            
            info!("whisper_full_parallel completed, returned: {}", result);
            
            // Clean up callback data and close channel
            let callback_data = Box::from_raw(callback_ptr_usize as *mut CallbackData);
            // Drop sender to signal completion
            if let Ok(mut sender_guard) = callback_data.sender.lock() {
                *sender_guard = None;
            }
            drop(callback_data);
            
            result
        }
    });
    
    // Wait for processing to complete
    let result = processing_handle.await
        .context("Failed to join processing task")?;
    
    // Wait for stream task to complete
    stream_handle.await
        .context("Failed to join stream task")?;
    
    info!("whisper_full_parallel returned: {}", result);
    
    if result != 0 {
        return Err(anyhow::anyhow!("Failed to process audio (return code: {})", result));
    }
    
    debug!("All segments sent successfully");
    
    // Send completion message (like C++ version)
    let complete_msg = CompleteMessage {
        msg_type: "complete".to_string(),
    };
    let json = serde_json::to_string(&complete_msg)?;
    info!("Sending completion message: {}", json);
    
    {
        let mut stream_guard = stream_arc.lock().await;
        if let Err(e) = write_ws_frame(&mut *stream_guard, json.as_bytes(), WS_OPCODE_TEXT).await {
            error!("Failed to send completion message: {}", e);
            return Err(e);
        }
        info!("Completion message sent successfully");
    } // Drop guard here
    
    // Give client time to receive the complete message
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Return stream ownership
    Ok(Arc::try_unwrap(stream_arc)
        .map_err(|_| anyhow::anyhow!("Failed to unwrap stream Arc"))?
        .into_inner())
}

async fn send_segment_data(
    stream: &mut TcpStream,
    segment: SegmentData,
    params: &WhisperParams,
    audio_duration_s: f32,
    t_start_process_us: i64,
) -> Result<()> {
    let mut segment_msg = SegmentMessage {
        msg_type: "segment".to_string(),
        index: segment.index,
        text: escape_json_string(&segment.text),
        start: None,
        end: None,
        progress: None,
        eta: None,
        speaker: None,
    };
    
    if !params.no_timestamps {
        segment_msg.start = Some(segment.t0 as f64 * 0.01);
        segment_msg.end = Some(segment.t1 as f64 * 0.01);
    }
    
    // Calculate progress and ETA
    if audio_duration_s > 0.0 && !params.no_timestamps {
        let current_time_s = segment.t1 as f32 * 0.01;
        let progress = (current_time_s / audio_duration_s * 100.0).min(100.0).max(0.0);
        segment_msg.progress = Some(progress as f64);
        
        // Calculate ETA
        if current_time_s > 0.0 && t_start_process_us > 0 {
            let t_now_us = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as i64;
            let elapsed_s = (t_now_us - t_start_process_us) as f32 / 1_000_000.0;
            let current_realtime_factor = current_time_s / elapsed_s;
            let remaining_audio_s = audio_duration_s - current_time_s;
            let eta_s = remaining_audio_s / current_realtime_factor;
            
            if eta_s > 0.0 && eta_s < 3600.0 {
                segment_msg.eta = Some(eta_s as f64);
            }
        }
    }
    
    let json = serde_json::to_string(&segment_msg)?;
    debug!("Sending segment {} JSON: {}", segment.index, json);
    if let Err(e) = write_ws_frame(stream, json.as_bytes(), WS_OPCODE_TEXT).await {
        error!("Failed to write WebSocket frame for segment {}: {}", segment.index, e);
        return Err(e);
    }
    debug!("Segment {} sent successfully", segment.index);
    
    Ok(())
}

async fn send_segment_raw(
    stream: &mut TcpStream,
    ctx: &WhisperContextWrapper,
    params: &WhisperParams,
    i: i32,
    audio_duration_s: f32,
    t_start_process_us: i64,
) -> Result<()> {
    let text = ctx.get_segment_text(i);
    
    // Get timestamps
    let (t0, t1) = if !params.no_timestamps || params.diarize {
        (ctx.get_segment_t0(i), ctx.get_segment_t1(i))
    } else {
        (0, 0)
    };
    
    let mut segment_msg = SegmentMessage {
        msg_type: "segment".to_string(),
        index: i,
        text: escape_json_string(&text),
        start: None,
        end: None,
        progress: None,
        eta: None,
        speaker: None,
    };
    
    if !params.no_timestamps {
        segment_msg.start = Some(t0 as f64 * 0.01);
        segment_msg.end = Some(t1 as f64 * 0.01);
    }
    
    // Calculate progress and ETA
    if audio_duration_s > 0.0 && !params.no_timestamps {
        let current_time_s = t1 as f32 * 0.01;
        let progress = (current_time_s / audio_duration_s * 100.0).min(100.0).max(0.0);
        segment_msg.progress = Some(progress as f64);
        
        // Calculate ETA
        if current_time_s > 0.0 && t_start_process_us > 0 {
            let t_now_us = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as i64;
            let elapsed_s = (t_now_us - t_start_process_us) as f32 / 1_000_000.0;
            let current_realtime_factor = current_time_s / elapsed_s;
            let remaining_audio_s = audio_duration_s - current_time_s;
            let eta_s = remaining_audio_s / current_realtime_factor;
            
            if eta_s > 0.0 && eta_s < 3600.0 {
                segment_msg.eta = Some(eta_s as f64);
            }
        }
    }
    
    let json = serde_json::to_string(&segment_msg)?;
    debug!("Sending segment {} JSON: {}", i, json);
    if let Err(e) = write_ws_frame(stream, json.as_bytes(), WS_OPCODE_TEXT).await {
        error!("Failed to write WebSocket frame for segment {}: {}", i, e);
        return Err(e);
    }
    debug!("Segment {} sent successfully", i);
    
    Ok(())
}

fn escape_json_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            _ => c.to_string(),
        })
        .collect()
}
