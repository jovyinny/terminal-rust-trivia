pub mod protocol;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Write a length-prefixed JSON message to a stream
pub async fn write_message<W, M>(writer: &mut W, message: &M) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
    M: Serialize,
{
    let json = serde_json::to_string(message)?;
    let bytes = json.as_bytes();
    let len = bytes.len() as u32;
    
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(bytes).await?;
    writer.flush().await?;
    
    Ok(())
}

/// Read a length-prefixed JSON message from a stream
pub async fn read_message<R, M>(reader: &mut R) -> Result<M>
where
    R: AsyncReadExt + Unpin,
    M: for<'de> Deserialize<'de>,
{
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    
    if len > 1_000_000 {
        anyhow::bail!("Message too large: {} bytes", len);
    }
    
    let mut buffer = vec![0u8; len];
    reader.read_exact(&mut buffer).await?;
    
    let message = serde_json::from_slice(&buffer)?;
    Ok(message)
}
