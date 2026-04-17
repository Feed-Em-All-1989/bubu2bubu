use crate::crypto::keys::KeyPair;
use crate::net::peer::{ServerConnection, decode_stego};
use crate::protocol::{ClientMsg, ServerMsg};
use crate::chat::profile::{self, PeerProfile};
use std::collections::HashMap;
use std::path::PathBuf;

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
    avatar: Option<String>,
    key_tag: String,
    stego_key: String,
    server_addr: String,
    connection: Option<ServerConnection>,
    messages: Vec<ChatMessage>,
    online_users: Vec<String>,
    peer_profiles: HashMap<String, PeerProfile>,
    data_dir: Option<PathBuf>,
}

impl ChatSession {
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        let (keypair, saved_name, saved_avatar) = match &data_dir {
            Some(dir) => match profile::load_profile(dir) {
                Some((name, avatar, secret_bytes)) => {
                    (KeyPair::from_secret_bytes(secret_bytes), name, avatar)
                }
                None => (KeyPair::generate(), String::new(), None),
            },
            None => (KeyPair::generate(), String::new(), None),
        };
        let key_tag = profile::compute_key_tag(keypair.public.as_bytes());

        Self {
            keypair,
            username: saved_name,
            avatar: saved_avatar,
            key_tag,
            stego_key: String::new(),
            server_addr: String::new(),
            connection: None,
            messages: Vec::new(),
            online_users: Vec::new(),
            peer_profiles: HashMap::new(),
            data_dir,
        }
    }

    pub fn set_username(&mut self, name: String) {
        self.username = name;
        self.save_profile();
        self.update_own_peer_profile();
    }

    pub async fn update_username(&mut self, name: String) -> Result<(), String> {
        self.username = name;
        self.save_profile();
        self.update_own_peer_profile();
        if self.connection.is_some() {
            self.send_profile().await?;
        }
        Ok(())
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

    pub fn key_tag(&self) -> &str {
        &self.key_tag
    }

    pub fn avatar(&self) -> &Option<String> {
        &self.avatar
    }

    pub async fn set_avatar_validated(&mut self, sanitized: String) -> Result<(), String> {
        self.avatar = Some(sanitized);
        self.save_profile();
        self.update_own_peer_profile();
        if self.connection.is_some() {
            self.send_profile().await?;
        }
        Ok(())
    }

    pub fn online_users(&self) -> &[String] {
        &self.online_users
    }

    pub fn peer_profiles(&self) -> Vec<PeerProfile> {
        let mut result = self.peer_profiles.clone();
        result.insert(self.username.clone(), PeerProfile {
            key_tag: self.key_tag.clone(),
            name: self.username.clone(),
            avatar: self.avatar.clone(),
        });
        result.into_values().collect()
    }

    fn save_profile(&self) {
        if let Some(ref dir) = self.data_dir {
            profile::save_profile(dir, &self.username, &self.avatar, &self.keypair.to_secret_bytes());
        }
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

        self.server_addr = addr.to_string();
        self.stego_key = room_key.clone();
        self.connection = Some(conn);
        self.online_users.clear();
        self.peer_profiles.clear();
        self.send_profile().await?;
        Ok(room_key)
    }

    pub async fn reconnect(&mut self) -> Result<String, String> {
        if self.server_addr.is_empty() {
            return Err("no server address stored".into());
        }
        if self.username.is_empty() {
            return Err("username not set".into());
        }

        self.connection = None;
        self.online_users.clear();
        self.peer_profiles.clear();

        let (conn, room_key) = ServerConnection::connect(
            &self.server_addr,
            self.keypair.secret.as_bytes(),
            &self.username,
        ).await?;

        self.connection = Some(conn);
        self.send_profile().await?;
        Ok(room_key)
    }

    fn update_own_peer_profile(&mut self) {
        self.peer_profiles.insert(self.username.clone(), PeerProfile {
            key_tag: self.key_tag.clone(),
            name: self.username.clone(),
            avatar: self.avatar.clone(),
        });
    }

    async fn send_profile(&mut self) -> Result<(), String> {
        self.update_own_peer_profile();
        let conn = self.connection.as_mut().ok_or("not connected")?;
        let msg = ClientMsg::SetProfile {
            key_tag: self.key_tag.clone(),
            name: self.username.clone(),
            avatar: self.avatar.clone(),
        };
        conn.send_msg(&msg).await
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
            ServerMsg::UserList { users } => {
                self.online_users = users;
                return Err("timeout".into());
            }
            ServerMsg::ProfileUpdate { key_tag, name, avatar } => {
                if key_tag == self.key_tag && name == self.username {
                    return Err("timeout".into());
                }
                let existing = self.peer_profiles.values()
                    .find(|p| p.key_tag == key_tag)
                    .cloned();
                let mut changes: Vec<String> = Vec::new();
                let display = if let Some(ref existing) = existing {
                    if existing.name != name {
                        changes.push(format!("changed their name to {}", name));
                    }
                    if existing.avatar != avatar {
                        changes.push("updated their profile picture".into());
                    }
                    existing.name.clone()
                } else {
                    name.clone()
                };
                if let Some(ref existing) = existing {
                    if existing.name != name {
                        self.peer_profiles.insert(existing.name.clone(), PeerProfile {
                            key_tag: key_tag.clone(),
                            name: name.clone(),
                            avatar: avatar.clone(),
                        });
                    }
                }
                self.peer_profiles.insert(name.clone(), PeerProfile {
                    key_tag,
                    name,
                    avatar,
                });
                if changes.is_empty() {
                    return Err("timeout".into());
                }
                ChatMessage {
                    id: generate_id(),
                    sender: "system".into(),
                    content: format!("{} {}", display, changes.join(" and ")),
                    from_self: false,
                    timestamp: now_secs(),
                    reply_to: None,
                    stego_image: None,
                }
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
