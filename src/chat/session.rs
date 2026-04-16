use crate::crypto::keys::KeyPair;
use crate::net::peer::{ServerConnection, decode_stego};
use crate::protocol::ServerMsg;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChatMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub from_self: bool,
    pub timestamp: u64,
    pub reply_to: Option<String>,
    pub stego_image: Option<String>,
}

pub struct ChatSession {
    keypair: KeyPair,
    username: String,
    stego_key: String,
    connection: Option<ServerConnection>,
    messages: Vec<ChatMessage>,
}

impl ChatSession {
    pub fn new() -> Self {
        Self {
            keypair: KeyPair::generate(),
            username: String::new(),
            stego_key: String::new(),
            connection: None,
            messages: Vec::new(),
        }
    }

    pub fn set_username(&mut self, name: String) {
        self.username = name;
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn stego_key(&self) -> &str {
        &self.stego_key
    }

    pub fn set_encryption_key(&mut self, key: String) {
        self.stego_key = key;
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.keypair.public.as_bytes())
    }

    pub async fn connect(&mut self, addr: &str) -> Result<String, String> {
        if self.username.is_empty() {
            return Err("username not set".into());
        }

        let (conn, room_key) = ServerConnection::connect(
            addr,
            self.keypair.secret.as_bytes(),
            &self.username,
        ).await?;

        self.stego_key = room_key.clone();
        self.connection = Some(conn);
        Ok(room_key)
    }

    pub async fn send(&mut self, text: &str, reply_to: Option<String>) -> Result<ChatMessage, String> {
        let conn = self.connection.as_mut().ok_or("not connected")?;
        let id = generate_id();
        let stego_image = conn.send_chat(text.as_bytes(), &id, reply_to.clone(), &self.stego_key).await?;

        let msg = ChatMessage {
            id,
            sender: self.username.clone(),
            content: text.to_string(),
            from_self: true,
            timestamp: now_secs(),
            reply_to,
            stego_image: Some(stego_image),
        };
        self.messages.push(msg.clone());
        Ok(msg)
    }

    pub fn recv(&mut self) -> Result<ChatMessage, String> {
        let conn = self.connection.as_mut().ok_or("not connected")?;
        let server_msg = conn.try_recv().ok_or("timeout")??;

        let msg = match server_msg {
            ServerMsg::Welcome { .. } => {
                return Err("timeout".into());
            }
            ServerMsg::Chat { sender, id, reply_to, image, metadata } => {
                match decode_stego(&image, &metadata, &self.stego_key) {
                    Ok(plaintext) => {
                        let content = String::from_utf8(plaintext)
                            .unwrap_or_else(|_| "[could not decrypt]".into());
                        ChatMessage {
                            id,
                            sender,
                            content,
                            from_self: false,
                            timestamp: now_secs(),
                            reply_to,
                            stego_image: Some(image),
                        }
                    }
                    Err(_) => {
                        ChatMessage {
                            id,
                            sender,
                            content: "[could not decrypt]".into(),
                            from_self: false,
                            timestamp: now_secs(),
                            reply_to,
                            stego_image: Some(image),
                        }
                    }
                }
            }
            ServerMsg::Joined { name, online } => {
                ChatMessage {
                    id: generate_id(),
                    sender: "system".into(),
                    content: format!("{} joined ({} online)", name, online),
                    from_self: false,
                    timestamp: now_secs(),
                    reply_to: None,
                    stego_image: None,
                }
            }
            ServerMsg::Left { name, online } => {
                ChatMessage {
                    id: generate_id(),
                    sender: "system".into(),
                    content: format!("{} left ({} online)", name, online),
                    from_self: false,
                    timestamp: now_secs(),
                    reply_to: None,
                    stego_image: None,
                }
            }
        };
        self.messages.push(msg.clone());
        Ok(msg)
    }

    pub fn history(&self) -> &[ChatMessage] {
        &self.messages
    }
}

fn generate_id() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let r: u32 = rand::random();
    format!("{}-{:08x}", ts, r)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
