use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf};
use base64::Engine;
use image::GenericImageView;

const MAX_AVATAR_DIM: u32 = 128;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerProfile {
    pub key_tag: String,
    pub name: String,
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StoredProfile {
    name: String,
    avatar: Option<String>,
    secret_key: String,
}

pub fn compute_key_tag(public_key_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(public_key_bytes);
    let hash = hasher.finalize();
    hex::encode(&hash[..4])
}

pub fn validate_avatar(base64_data: &str) -> Result<String, String> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|e| format!("invalid base64: {}", e))?;

    let is_png = raw.starts_with(&[0x89, 0x50, 0x4E, 0x47]);
    let is_jpeg = raw.starts_with(&[0xFF, 0xD8, 0xFF]);
    if !is_png && !is_jpeg {
        return Err("only PNG and JPEG images are allowed".into());
    }

    let img = image::load_from_memory(&raw)
        .map_err(|e| format!("invalid image: {}", e))?;

    let (w, h) = img.dimensions();
    let img = if w > MAX_AVATAR_DIM || h > MAX_AVATAR_DIM {
        img.resize(MAX_AVATAR_DIM, MAX_AVATAR_DIM, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("failed to encode: {}", e))?;

    Ok(base64::engine::general_purpose::STANDARD.encode(&buf))
}

pub fn profile_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("bubu2bubu")
}

pub fn save_profile(data_dir: &Path, name: &str, avatar: &Option<String>, secret_key: &[u8; 32]) {
    let dir = profile_dir(data_dir);
    let _ = std::fs::create_dir_all(&dir);
    let stored = StoredProfile {
        name: name.to_string(),
        avatar: avatar.clone(),
        secret_key: hex::encode(secret_key),
    };
    if let Ok(json) = serde_json::to_string_pretty(&stored) {
        let _ = std::fs::write(dir.join("profile.json"), json);
    }
}

pub fn load_profile(data_dir: &Path) -> Option<(String, Option<String>, [u8; 32])> {
    let path = profile_dir(data_dir).join("profile.json");
    let data = std::fs::read_to_string(path).ok()?;
    let stored: StoredProfile = serde_json::from_str(&data).ok()?;
    let key_bytes = hex::decode(&stored.secret_key).ok()?;
    if key_bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&key_bytes);
    Some((stored.name, stored.avatar, arr))
}
