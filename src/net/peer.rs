use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use crate::crypto::noise::build_initiator;
use crate::net::Transport;
use crate::net::protocol::{send_frame, recv_frame, send_noise_msg, recv_noise_msg};
use crate::protocol::{ClientMsg, ServerMsg};
use crate::stego::encoder::{self, StegoConfig, StegoMetadata};
use crate::stego::decoder;

pub struct ServerConnection {
    transport: Arc<Mutex<Transport>>,
    writer: OwnedWriteHalf,
    incoming_rx: mpsc::Receiver<Result<ServerMsg, String>>,
}

impl ServerConnection {
    pub async fn connect(
        addr: &str,
        local_key: &[u8; 32],
        username: &str,
    ) -> Result<(Self, String), String> {
        let mut stream = TcpStream::connect(addr).await.map_err(|e| e.to_string())?;

        let (_, mut initiator) = build_initiator(local_key)?;

        let msg1 = initiator.write_message(&[])?;
        send_frame(&mut stream, &msg1).await?;

        let msg2 = recv_frame(&mut stream).await?;
        initiator.read_message(&msg2)?;

        let msg3 = initiator.write_message(&[])?;
        send_frame(&mut stream, &msg3).await?;

        let noise = initiator.into_transport()?;
        let (mut read_half, write_half) = stream.into_split();
        let transport = Arc::new(Mutex::new(Transport::Initiator(noise)));

        let join = ClientMsg::Join { name: username.to_string() };
        let join_data = serde_json::to_vec(&join).map_err(|e| e.to_string())?;
        let mut writer = write_half;
        send_noise_msg(&mut writer, &transport, &join_data).await?;

        let welcome_data = recv_noise_msg(&mut read_half, &transport).await?;
        let welcome: ServerMsg = serde_json::from_slice(&welcome_data).map_err(|e| e.to_string())?;
        let room_key = match welcome {
            ServerMsg::Welcome { room_key } => room_key,
            _ => return Err("expected welcome message".into()),
        };

        let (tx, rx) = mpsc::channel::<Result<ServerMsg, String>>(256);
        let recv_transport = transport.clone();

        tokio::spawn(async move {
            let mut reader = read_half;
            loop {
                let raw = match recv_noise_msg(&mut reader, &recv_transport).await {
                    Ok(data) => data,
                    Err(_) => {
                        let _ = tx.send(Err("disconnected".into())).await;
                        break;
                    }
                };
                match serde_json::from_slice::<ServerMsg>(&raw) {
                    Ok(msg) => {
                        if tx.send(Ok(msg)).await.is_err() { break; }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string())).await;
                    }
                }
            }
        });

        let conn = Self {
            transport,
            writer,
            incoming_rx: rx,
        };
        Ok((conn, room_key))
    }

    pub async fn send_chat(
        &mut self,
        plaintext: &[u8],
        id: &str,
        reply_to: Option<String>,
        stego_password: &str,
    ) -> Result<String, String> {
        let config = StegoConfig::default();
        let (png_bytes, metadata) = encoder::encode(plaintext, stego_password, &config).await?;
        let image = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &png_bytes,
        );

        let msg = ClientMsg::Chat {
            id: id.to_string(),
            reply_to,
            image: image.clone(),
            metadata,
        };
        let data = serde_json::to_vec(&msg).map_err(|e| e.to_string())?;
        send_noise_msg(&mut self.writer, &self.transport, &data).await?;

        Ok(image)
    }

    pub async fn send_msg(&mut self, msg: &ClientMsg) -> Result<(), String> {
        let data = serde_json::to_vec(msg).map_err(|e| e.to_string())?;
        send_noise_msg(&mut self.writer, &self.transport, &data).await
    }

    pub fn try_recv(&mut self) -> Option<Result<ServerMsg, String>> {
        self.incoming_rx.try_recv().ok()
    }
}

pub fn decode_stego(image_b64: &str, metadata: &StegoMetadata, stego_password: &str) -> Result<Vec<u8>, String> {
    let png_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        image_b64,
    ).map_err(|e| e.to_string())?;
    decoder::decode(&png_bytes, stego_password, metadata)
}
