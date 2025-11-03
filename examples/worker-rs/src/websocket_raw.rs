// Raw WebSocket frame parser - similar to C++ read_ws_frame
// Reads raw bytes without UTF-8 validation for TEXT frames

use anyhow::{Context, Result};
use log::{info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub const WS_OPCODE_CONT: u8 = 0x0;
pub const WS_OPCODE_TEXT: u8 = 0x1;
pub const WS_OPCODE_BINARY: u8 = 0x2;
pub const WS_OPCODE_CLOSE: u8 = 0x8;
pub const WS_OPCODE_PING: u8 = 0x9;
pub const WS_OPCODE_PONG: u8 = 0xA;

pub struct WebSocketFrame {
    pub opcode: u8,
    pub payload: Vec<u8>,
    pub fin: bool,
}

pub async fn read_ws_frame(stream: &mut TcpStream) -> Result<Option<WebSocketFrame>> {
    // Read header (2 bytes)
    let mut header = [0u8; 2];
    let n = stream.read_exact(&mut header).await;
    if n.is_err() {
        return Ok(None); // Connection closed or error
    }
    
    let fin = (header[0] & 0x80) != 0;
    let opcode = header[0] & 0x0F;
    let masked = (header[1] & 0x80) != 0;
    let mut payload_len = (header[1] & 0x7F) as u64;
    
    // Read extended payload length if needed
    if payload_len == 126 {
        let mut len_bytes = [0u8; 2];
        stream.read_exact(&mut len_bytes).await?;
        payload_len = u16::from_be_bytes(len_bytes) as u64;
    } else if payload_len == 127 {
        let mut len_bytes = [0u8; 8];
        stream.read_exact(&mut len_bytes).await?;
        payload_len = u64::from_be_bytes(len_bytes);
    }
    
    // Read mask if present (client must mask frames)
    let mut mask = [0u8; 4];
    if masked {
        stream.read_exact(&mut mask).await?;
    }
    
    // Read payload
    let mut payload = vec![0u8; payload_len as usize];
    if payload_len > 0 {
        stream.read_exact(&mut payload).await?;
        
        // Unmask payload if masked
        if masked {
            for i in 0..payload.len() {
                payload[i] ^= mask[i % 4];
            }
        }
    }
    
    Ok(Some(WebSocketFrame {
        opcode,
        payload,
        fin,
    }))
}

pub async fn write_ws_frame(stream: &mut TcpStream, data: &[u8], opcode: u8) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    
    let mut frame = Vec::new();
    
    // FIN bit set, opcode
    frame.push(0x80 | opcode);
    
    // Payload length
    if data.len() < 126 {
        frame.push(data.len() as u8);
    } else if data.len() < 65536 {
        frame.push(126);
        frame.extend_from_slice(&(data.len() as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(data.len() as u64).to_be_bytes());
    }
    
    // Payload (server doesn't mask)
    frame.extend_from_slice(data);
    
    // Write frame and flush to ensure it's sent immediately
    stream.write_all(&frame).await?;
    stream.flush().await?;
    
    Ok(())
}

