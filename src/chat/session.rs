use tokio::sync::mpsc;
use crate::crypto::keys::KeyPair;
use crate::net::peer::{PeerConnection, PeerEvent};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChatMessage {
    pub content: String,
    pub from_self: bool,
    pub timestamp: u64,
}

pub struct ChatSession {
    keypair: KeyPair,
    connection: Option<PeerConnection>,
    messages: Vec<ChatMessage>,
    event_tx: mpsc::Sender<PeerEvent>,
    #[allow(dead_code)]
    event_rx: mpsc::Receiver<PeerEvent>,
}

impl ChatSession {
    pub fn new() -> Self {
        let keypair = KeyPair::generate();
        let (event_tx, event_rx) = mpsc::channel(256);
        Self {
            keypair,
            connection: None,
            messages: Vec::new(),
            event_tx,
            event_rx,
        }
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.keypair.public.as_bytes())
    }

    pub async fn host(&mut self, port: u16) -> Result<(), String> {
        let conn = PeerConnection::listen(
            port,
            self.keypair.secret.as_bytes(),
            self.event_tx.clone(),
        ).await?;
        self.connection = Some(conn);
        Ok(())
    }

    pub async fn join(&mut self, addr: &str) -> Result<(), String> {
        let conn = PeerConnection::connect(
            addr,
            self.keypair.secret.as_bytes(),
            self.event_tx.clone(),
        ).await?;
        self.connection = Some(conn);
        Ok(())
    }

    pub async fn send(&mut self, text: &str) -> Result<(), String> {
        let conn = self.connection.as_mut().ok_or("not connected")?;
        conn.send_message(text.as_bytes()).await?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.messages.push(ChatMessage {
            content: text.to_string(),
            from_self: true,
            timestamp,
        });
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<ChatMessage, String> {
        let conn = self.connection.as_mut().ok_or("not connected")?;
        let data = conn.try_recv().ok_or("timeout")??;
        let content = String::from_utf8(data).map_err(|_| "invalid utf8")?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let msg = ChatMessage {
            content,
            from_self: false,
            timestamp,
        };
        self.messages.push(msg.clone());
        Ok(msg)
    }

    pub fn history(&self) -> &[ChatMessage] {
        &self.messages
    }
}