use serde::{Serialize, Deserialize};
use crate::stego::encoder::StegoMetadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMsg {
    #[serde(rename = "join")]
    Join { name: String },
    #[serde(rename = "chat")]
    Chat {
        id: String,
        reply_to: Option<String>,
        image: String,
        metadata: StegoMetadata,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    #[serde(rename = "joined")]
    Joined { name: String, online: usize },
    #[serde(rename = "left")]
    Left { name: String, online: usize },
    #[serde(rename = "chat")]
    Chat {
        sender: String,
        id: String,
        reply_to: Option<String>,
        image: String,
        metadata: StegoMetadata,
    },
}
