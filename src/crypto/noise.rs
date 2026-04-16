use snow::{Builder, TransportState};

const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

pub struct NoiseInitiator {
    state: TransportState,
}

pub struct NoiseResponder {
    state: TransportState,
}

pub fn build_initiator(local_key: &[u8; 32]) -> Result<(Vec<u8>, NoiseHandshakeInitiator), String> {
    let builder = Builder::new(NOISE_PATTERN.parse().map_err(|e: snow::Error| e.to_string())?)
        .local_private_key(local_key)
        .build_initiator()
        .map_err(|e| e.to_string())?;
    Ok((Vec::new(), NoiseHandshakeInitiator { state: builder }))
}

pub fn build_responder(local_key: &[u8; 32]) -> Result<NoiseHandshakeResponder, String> {
    let builder = Builder::new(NOISE_PATTERN.parse().map_err(|e: snow::Error| e.to_string())?)
        .local_private_key(local_key)
        .build_responder()
        .map_err(|e| e.to_string())?;
    Ok(NoiseHandshakeResponder { state: builder })
}

pub struct NoiseHandshakeInitiator {
    state: snow::HandshakeState,
}

impl NoiseHandshakeInitiator {
    pub fn write_message(&mut self, payload: &[u8]) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; 65535];
        let len = self.state.write_message(payload, &mut buf).map_err(|e| e.to_string())?;
        buf.truncate(len);
        Ok(buf)
    }

    pub fn read_message(&mut self, message: &[u8]) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; 65535];
        let len = self.state.read_message(message, &mut buf).map_err(|e| e.to_string())?;
        buf.truncate(len);
        Ok(buf)
    }

    pub fn is_handshake_finished(&self) -> bool {
        self.state.is_handshake_finished()
    }

    pub fn into_transport(self) -> Result<NoiseInitiator, String> {
        let transport = self.state.into_transport_mode().map_err(|e| e.to_string())?;
        Ok(NoiseInitiator { state: transport })
    }
}

pub struct NoiseHandshakeResponder {
    state: snow::HandshakeState,
}

impl NoiseHandshakeResponder {
    pub fn write_message(&mut self, payload: &[u8]) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; 65535];
        let len = self.state.write_message(payload, &mut buf).map_err(|e| e.to_string())?;
        buf.truncate(len);
        Ok(buf)
    }

    pub fn read_message(&mut self, message: &[u8]) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; 65535];
        let len = self.state.read_message(message, &mut buf).map_err(|e| e.to_string())?;
        buf.truncate(len);
        Ok(buf)
    }

    pub fn is_handshake_finished(&self) -> bool {
        self.state.is_handshake_finished()
    }

    pub fn into_transport(self) -> Result<NoiseResponder, String> {
        let transport = self.state.into_transport_mode().map_err(|e| e.to_string())?;
        Ok(NoiseResponder { state: transport })
    }
}

impl NoiseInitiator {
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; plaintext.len() + 64];
        let len = self.state.write_message(plaintext, &mut buf).map_err(|e| e.to_string())?;
        buf.truncate(len);
        Ok(buf)
    }

    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; ciphertext.len()];
        let len = self.state.read_message(ciphertext, &mut buf).map_err(|e| e.to_string())?;
        buf.truncate(len);
        Ok(buf)
    }
}

impl NoiseResponder {
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; plaintext.len() + 64];
        let len = self.state.write_message(plaintext, &mut buf).map_err(|e| e.to_string())?;
        buf.truncate(len);
        Ok(buf)
    }

    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; ciphertext.len()];
        let len = self.state.read_message(ciphertext, &mut buf).map_err(|e| e.to_string())?;
        buf.truncate(len);
        Ok(buf)
    }
}
