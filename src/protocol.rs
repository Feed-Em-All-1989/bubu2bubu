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
    #[serde(rename = "set_profile")]
    SetProfile {
        key_tag: String,
        name: String,
        avatar: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    #[serde(rename = "welcome")]
    Welcome { room_key: String },
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
    #[serde(rename = "user_list")]
    UserList { users: Vec<String> },
    #[serde(rename = "profile_update")]
    ProfileUpdate {
        key_tag: String,
        name: String,
        avatar: Option<String>,
    },
}
