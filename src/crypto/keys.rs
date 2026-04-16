use x25519_dalek::{StaticSecret, PublicKey};
use rand::rngs::OsRng;

pub struct KeyPair {
    pub secret: StaticSecret,
    pub public: PublicKey,
}

impl KeyPair {
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    pub fn shared_secret(&self, their_public: &PublicKey) -> [u8; 32] {
        let shared = self.secret.diffie_hellman(their_public);
        *shared.as_bytes()
    }
}
