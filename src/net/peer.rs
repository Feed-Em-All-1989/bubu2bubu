use tokio::net::{TcpListener, TcpStream};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use crate::crypto::noise::{
    build_initiator, build_responder,
    NoiseInitiator, NoiseResponder,
};
use crate::stego::encoder::{self, StegoConfig, StegoMetadata};
use crate::stego::decoder;
use crate::net::protocol::{send_frame, recv_frame};

const MAX_NOISE_PAYLOAD: usize = 65000;

#[derive(Debug)]
pub enum PeerEvent {
    Connected,
    Message(Vec<u8>),
    Disconnected,
}

enum Transport {
    Initiator(NoiseInitiator),
    Responder(NoiseResponder),
}

impl Transport {
    fn encrypt(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        match self {
            Transport::Initiator(t) => t.encrypt(data),
            Transport::Responder(t) => t.encrypt(data),
        }
    }

    fn decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        match self {
            Transport::Initiator(t) => t.decrypt(data),
            Transport::Responder(t) => t.decrypt(data),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StegoPacket {
    image: String,
    metadata: StegoMetadata,
}

pub struct PeerConnection {
    transport: Arc<Mutex<Transport>>,
    writer: OwnedWriteHalf,
    incoming_rx: mpsc::Receiver<Result<Vec<u8>, String>>,
}

impl PeerConnection {
    pub async fn listen(
        port: u16,
        local_key: &[u8; 32],
        event_tx: mpsc::Sender<PeerEvent>,
    ) -> Result<Self, String> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
            .await
            .map_err(|e| e.to_string())?;

        let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;

        let mut responder = build_responder(local_key)?;

        let msg1 = recv_frame(&mut stream).await?;
        responder.read_message(&msg1)?;

        let msg2 = responder.write_message(&[])?;
        send_frame(&mut stream, &msg2).await?;

        let msg3 = recv_frame(&mut stream).await?;
        responder.read_message(&msg3)?;

        let transport = responder.into_transport()?;
        let _ = event_tx.send(PeerEvent::Connected).await;

        Self::split_and_spawn(stream, Transport::Responder(transport))
    }

    pub async fn connect(
        addr: &str,
        local_key: &[u8; 32],
        event_tx: mpsc::Sender<PeerEvent>,
    ) -> Result<Self, String> {
        let mut stream = TcpStream::connect(addr).await.map_err(|e| e.to_string())?;

        let (_, mut initiator) = build_initiator(local_key)?;

        let msg1 = initiator.write_message(&[])?;
        send_frame(&mut stream, &msg1).await?;

        let msg2 = recv_frame(&mut stream).await?;
        initiator.read_message(&msg2)?;

        let msg3 = initiator.write_message(&[])?;
        send_frame(&mut stream, &msg3).await?;

        let transport = initiator.into_transport()?;
        let _ = event_tx.send(PeerEvent::Connected).await;

        Self::split_and_spawn(stream, Transport::Initiator(transport))
    }

    fn split_and_spawn(stream: TcpStream, transport: Transport) -> Result<Self, String> {
        let (read_half, write_half) = stream.into_split();
        let transport = Arc::new(Mutex::new(transport));
        let (tx, rx) = mpsc::channel::<Result<Vec<u8>, String>>(256);

        let recv_transport = transport.clone();
        tokio::spawn(async move {
            let mut reader = read_half;
            loop {
                let enc_header = match recv_frame(&mut reader).await {
                    Ok(data) => data,
                    Err(_) => break,
                };
                let header = match recv_transport.lock().await.decrypt(&enc_header) {
                    Ok(data) => data,
                    Err(e) => {
                        let _ = tx.send(Err(format!("noise header: {}", e))).await;
                        break;
                    }
                };
                if header.len() < 4 {
                    let _ = tx.send(Err("bad header".into())).await;
                    break;
                }
                let num_chunks = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;

                let mut packed = Vec::new();
                let mut failed = false;
                for _ in 0..num_chunks {
                    let encrypted = match recv_frame(&mut reader).await {
                        Ok(data) => data,
                        Err(_) => { failed = true; break; }
                    };
                    match recv_transport.lock().await.decrypt(&encrypted) {
                        Ok(chunk) => packed.extend_from_slice(&chunk),
                        Err(e) => {
                            let _ = tx.send(Err(format!("noise chunk: {}", e))).await;
                            failed = true;
                            break;
                        }
                    }
                }
                if failed { break; }

                let result = decode_stego_packet(&packed);
                if tx.send(result).await.is_err() { break; }
            }
        });

        Ok(Self {
            transport,
            writer: write_half,
            incoming_rx: rx,
        })
    }

    pub async fn send_message(&mut self, plaintext: &[u8]) -> Result<(), String> {
        let config = StegoConfig::default();
        let (png_bytes, metadata) = encoder::encode(plaintext, "bubu2bubu-stego", &config)?;

        let packet = StegoPacket {
            image: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &png_bytes,
            ),
            metadata,
        };
        let packed = serde_json::to_vec(&packet).map_err(|e| e.to_string())?;

        let num_chunks = ((packed.len() + MAX_NOISE_PAYLOAD - 1) / MAX_NOISE_PAYLOAD) as u32;
        let header = num_chunks.to_be_bytes();
        let enc_header = self.transport.lock().await.encrypt(&header)?;
        send_frame(&mut self.writer, &enc_header).await?;

        for chunk in packed.chunks(MAX_NOISE_PAYLOAD) {
            let encrypted = self.transport.lock().await.encrypt(chunk)?;
            send_frame(&mut self.writer, &encrypted).await?;
        }
        Ok(())
    }

    pub fn try_recv(&mut self) -> Option<Result<Vec<u8>, String>> {
        self.incoming_rx.try_recv().ok()
    }
}

fn decode_stego_packet(packed: &[u8]) -> Result<Vec<u8>, String> {
    let packet: StegoPacket = serde_json::from_slice(packed).map_err(|e| e.to_string())?;
    let png_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &packet.image,
    ).map_err(|e| e.to_string())?;
    decoder::decode(&png_bytes, "bubu2bubu-stego", &packet.metadata)
}