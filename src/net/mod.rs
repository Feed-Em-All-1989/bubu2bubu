pub mod peer;
pub mod protocol;

use crate::crypto::noise::{NoiseInitiator, NoiseResponder};

pub enum Transport {
    Initiator(NoiseInitiator),
    Responder(NoiseResponder),
}

impl Transport {
    pub fn encrypt(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        match self {
            Transport::Initiator(t) => t.encrypt(data),
            Transport::Responder(t) => t.encrypt(data),
        }
    }

    pub fn decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        match self {
            Transport::Initiator(t) => t.decrypt(data),
            Transport::Responder(t) => t.decrypt(data),
        }
    }
}
