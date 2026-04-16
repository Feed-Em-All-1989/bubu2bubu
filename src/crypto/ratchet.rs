use hkdf::Hkdf;
use sha2::Sha256;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;
use rand::RngCore;

pub struct Ratchet {
    chain_key: [u8; 32],
}

impl Ratchet {
    pub fn new(shared_secret: &[u8; 32]) -> Self {
        Self {
            chain_key: *shared_secret,
        }
    }

    pub fn from_noise_session(session_bytes: &[u8]) -> Self {
        let hk = Hkdf::<Sha256>::new(Some(b"bubu2bubu-ratchet-init"), session_bytes);
        let mut chain_key = [0u8; 32];
        hk.expand(b"ratchet-chain", &mut chain_key).expect("hkdf expand");
        Self { chain_key }
    }

    fn advance(&mut self) -> [u8; 32] {
        let hk = Hkdf::<Sha256>::new(Some(&self.chain_key), b"ratchet-step");

        let mut message_key = [0u8; 32];
        hk.expand(b"message-key", &mut message_key).expect("hkdf expand");

        let mut next_chain = [0u8; 32];
        hk.expand(b"chain-key", &mut next_chain).expect("hkdf expand");

        self.chain_key = next_chain;
        message_key
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let message_key = self.advance();
        let cipher = Aes256Gcm::new_from_slice(&message_key).map_err(|e| e.to_string())?;

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, plaintext).map_err(|e| e.to_string())?;

        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    pub fn decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < 12 {
            return Err("data too short".into());
        }
        let message_key = self.advance();
        let cipher = Aes256Gcm::new_from_slice(&message_key).map_err(|e| e.to_string())?;

        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        cipher.decrypt(nonce, ciphertext).map_err(|e| e.to_string())
    }

    pub fn stego_password(&self) -> String {
        hex::encode(self.chain_key)
    }
}
