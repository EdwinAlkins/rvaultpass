//! Per-device Ed25519 identity. The private key never leaves the client.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;

pub const AUTH_MESSAGE_PREFIX: &str = "rvaultpass-auth-v1";

#[derive(Clone)]
pub struct DeviceIdentity {
    pub device_id: String,
    signing_key: SigningKey,
}

impl DeviceIdentity {
    pub fn generate(device_id: String) -> Self {
        Self {
            device_id,
            signing_key: SigningKey::generate(&mut OsRng),
        }
    }

    pub fn from_seed(device_id: String, seed: &[u8; 32]) -> Self {
        Self {
            device_id,
            signing_key: SigningKey::from_bytes(seed),
        }
    }

    pub fn seed(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    pub fn public_key_b64(&self) -> String {
        B64.encode(self.signing_key.verifying_key().to_bytes())
    }

    pub fn sign_session(&self, timestamp: i64) -> String {
        let message = auth_message(&self.device_id, timestamp);
        B64.encode(self.signing_key.sign(message.as_bytes()).to_bytes())
    }
}

pub fn auth_message(device_id: &str, timestamp: i64) -> String {
    format!("{}\n{}\n{}", AUTH_MESSAGE_PREFIX, device_id, timestamp)
}

pub fn verify_session(
    public_key_b64: &str,
    device_id: &str,
    timestamp: i64,
    signature_b64: &str,
) -> Result<(), String> {
    let public_bytes = B64
        .decode(public_key_b64)
        .map_err(|_| "Invalid device public key".to_string())?;
    let public_bytes: [u8; 32] = public_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "Device public key must be 32 bytes".to_string())?;
    let verifying = VerifyingKey::from_bytes(&public_bytes)
        .map_err(|_| "Invalid device public key".to_string())?;
    let signature_bytes = B64
        .decode(signature_b64)
        .map_err(|_| "Invalid session signature".to_string())?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| "Invalid session signature".to_string())?;
    verifying
        .verify(auth_message(device_id, timestamp).as_bytes(), &signature)
        .map_err(|_| "Session signature rejected".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_identity_signs_a_session() {
        let identity = DeviceIdentity::generate("device-1".into());
        let signature = identity.sign_session(42);
        verify_session(&identity.public_key_b64(), "device-1", 42, &signature).unwrap();
        assert!(verify_session(&identity.public_key_b64(), "device-1", 43, &signature).is_err());
    }
}
