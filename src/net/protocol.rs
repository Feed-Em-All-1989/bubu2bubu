use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};

pub async fn send_frame<W: AsyncWrite + Unpin>(writer: &mut W, data: &[u8]) -> Result<(), String> {
    let len = (data.len() as u32).to_be_bytes();
    writer.write_all(&len).await.map_err(|e| e.to_string())?;
    writer.write_all(data).await.map_err(|e| e.to_string())?;
    writer.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn recv_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<u8>, String> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await.map_err(|e| e.to_string())?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > 16 * 1024 * 1024 {
        return Err("frame too large".into());
    }

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
    Ok(buf)
}