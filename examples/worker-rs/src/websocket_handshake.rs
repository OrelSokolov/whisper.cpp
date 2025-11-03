// WebSocket handshake implementation - similar to C++ version

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use sha1::{Sha1, Digest};
use std::str;

const WS_MAGIC_STRING: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub fn extract_ws_key(request: &str) -> Option<String> {
    let key_header = "Sec-WebSocket-Key:";
    let key_pos = request.find(key_header)?;
    let key_start = request[key_pos + key_header.len()..].trim_start();
    let key_end = key_start.find("\r\n")?;
    Some(key_start[..key_end].trim().to_string())
}

pub fn create_ws_accept_response(key: &str) -> String {
    let accept_key = format!("{}{}", key, WS_MAGIC_STRING);
    let mut hasher = Sha1::new();
    hasher.update(accept_key.as_bytes());
    let hash = hasher.finalize();
    let encoded = general_purpose::STANDARD.encode(&hash);
    
    format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\
         \r\n",
        encoded
    )
}

pub async fn perform_websocket_handshake(stream: &mut tokio::net::TcpStream) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    // Read handshake request
    let mut buffer = vec![0u8; 4096];
    let n = stream.read(&mut buffer).await?;
    if n == 0 {
        return Err(anyhow::anyhow!("Failed to receive handshake data"));
    }
    
    let request = str::from_utf8(&buffer[..n])
        .context("Invalid UTF-8 in handshake request")?;
    
    // Check if it's a WebSocket upgrade request
    if !request.contains("Upgrade: websocket") {
        return Err(anyhow::anyhow!("Not a WebSocket upgrade request"));
    }
    
    // Extract WebSocket key
    let ws_key = extract_ws_key(request)
        .ok_or_else(|| anyhow::anyhow!("Failed to extract WebSocket key"))?;
    
    // Create and send accept response
    let response = create_ws_accept_response(&ws_key);
    stream.write_all(response.as_bytes()).await?;
    
    Ok(())
}

