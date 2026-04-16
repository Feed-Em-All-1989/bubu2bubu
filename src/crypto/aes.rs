use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;
use hkdf::Hkdf;
use sha2::Sha256;
use rand::RngCore;

pub fn derive_keys(password: &str, _iterations: u32) -> ([u8; 32], [u8; 32], Vec<u8>) {
    let hk = Hkdf::<Sha256>::new(Some(b"bubu2bubu-stego"), password.as_bytes());

    let mut aes_key = [0u8; 32];
    hk.expand(b"aes-key", &mut aes_key).expect("hkdf expand");

    let mut xor_key = [0u8; 32];
    hk.expand(b"xor-key", &mut xor_key).expect("hkdf expand");

    let mut chaos_seed = vec![0u8; 16];
    hk.expand(b"chaos-seed", &mut chaos_seed).expect("hkdf expand");

    (aes_key, xor_key, chaos_seed)
}

pub fn encrypt_aes(data: &[u8], key: &[u8; 32]) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>), String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, data)
        .map_err(|e| e.to_string())?;

    let tag = if ciphertext.len() >= 16 {
        ciphertext[ciphertext.len() - 16..].to_vec()
    } else {
        vec![]
    };

    Ok((ciphertext, vec![], nonce_bytes.to_vec(), tag))
}

pub fn decrypt_aes(
    ciphertext: &[u8],
    key: &[u8; 32],
    _salt: &[u8],
    nonce: &[u8],
    _tag: &[u8],
) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(nonce);
    cipher.decrypt(nonce, ciphertext)
        .map_err(|e| e.to_string())
}

pub fn xor_cipher(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % 32])
        .collect()
}
