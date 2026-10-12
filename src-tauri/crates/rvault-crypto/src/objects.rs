//! XChaCha20-Poly1305 envelopes. The server stores the output and cannot open it.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::rngs::OsRng;
use rand::RngCore;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

pub const KEY_WRAP_AAD: &[u8] = b"rvaultpass-vault-key-v1";

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyEnvelope {
    pub version: u8,
    pub kdf: String,
    pub salt: String,
    pub mem_cost: u32,
    pub time_cost: u32,
    pub parallelism: u32,
    pub nonce: String,
    pub wrapped_key: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SealedObject {
    pub nonce: String,
    pub ciphertext: String,
}

fn random_nonce() -> [u8; 24] {
    let mut nonce = [0u8; 24];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

pub fn seal(key: &[u8], nonce: &[u8; 24], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    if key.len() != 32 {
        return Err("Encryption key must be 32 bytes".to_string());
    }
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| "Invalid encryption key".to_string())?;
    cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| "Failed to seal object".to_string())
}

pub fn open(key: &[u8], nonce: &[u8; 24], aad: &[u8], ciphertext: &[u8]) -> Result<Secret<Vec<u8>>, String> {
    if key.len() != 32 {
        return Err("Encryption key must be 32 bytes".to_string());
    }
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| "Invalid encryption key".to_string())?;
    let plain = cipher
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| "Failed to open object".to_string())?;
    Ok(Secret::new(plain))
}

impl std::fmt::Debug for KeyEnvelope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("KeyEnvelope")
            .field("version", &self.version)
            .field("kdf", &self.kdf)
            .field("salt", &"<redacted>")
            .field("wrapped_key", &"<redacted>")
            .finish()
    }
}

impl std::fmt::Debug for SealedObject {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SealedObject")
            .field("nonce", &"<redacted>")
            .field("ciphertext", &"<redacted>")
            .finish()
    }
}

pub fn object_aad(object_id: &str, object_type: &str, deleted: bool) -> Vec<u8> {
    format!(
        "v1|{}|{}|{}",
        object_id,
        object_type,
        if deleted { "1" } else { "0" }
    )
    .into_bytes()
}

pub fn seal_object(
    data_key: &[u8],
    object_id: &str,
    object_type: &str,
    deleted: bool,
    plaintext: &[u8],
) -> Result<SealedObject, String> {
    let nonce = random_nonce();
    let aad = object_aad(object_id, object_type, deleted);
    let ciphertext = seal(data_key, &nonce, &aad, plaintext)?;
    Ok(SealedObject {
        nonce: B64.encode(nonce),
        ciphertext: B64.encode(ciphertext),
    })
}

pub fn open_object(
    data_key: &[u8],
    object_id: &str,
    object_type: &str,
    deleted: bool,
    nonce_b64: &str,
    ciphertext_b64: &str,
) -> Result<Secret<Vec<u8>>, String> {
    let nonce_vec = B64
        .decode(nonce_b64)
        .map_err(|_| "Invalid object nonce".to_string())?;
    let nonce: [u8; 24] = nonce_vec
        .as_slice()
        .try_into()
        .map_err(|_| "Object nonce must be 24 bytes".to_string())?;
    let ciphertext = B64
        .decode(ciphertext_b64)
        .map_err(|_| "Invalid object ciphertext".to_string())?;
    open(
        data_key,
        &nonce,
        &object_aad(object_id, object_type, deleted),
        &ciphertext,
    )
}

pub fn generate_data_key() -> Secret<Vec<u8>> {
    let mut key = vec![0u8; 32];
    OsRng.fill_bytes(&mut key);
    let secret = Secret::new(key.clone());
    key.zeroize();
    secret
}

pub fn wrap_data_key(
    kek: &[u8],
    data_key: &[u8],
    salt: &[u8],
    mem_cost: u32,
    time_cost: u32,
    parallelism: u32,
) -> Result<KeyEnvelope, String> {
    let nonce = random_nonce();
    let wrapped = seal(kek, &nonce, KEY_WRAP_AAD, data_key)?;
    Ok(KeyEnvelope {
        version: 1,
        kdf: "argon2id".to_string(),
        salt: B64.encode(salt),
        mem_cost,
        time_cost,
        parallelism,
        nonce: B64.encode(nonce),
        wrapped_key: B64.encode(wrapped),
    })
}

pub fn unwrap_data_key(kek: &[u8], envelope: &KeyEnvelope) -> Result<Secret<Vec<u8>>, String> {
    if envelope.version != 1 || envelope.kdf != "argon2id" {
        return Err("Unsupported key envelope".to_string());
    }
    let nonce_vec = B64
        .decode(&envelope.nonce)
        .map_err(|_| "Invalid key nonce".to_string())?;
    let nonce: [u8; 24] = nonce_vec
        .as_slice()
        .try_into()
        .map_err(|_| "Key nonce must be 24 bytes".to_string())?;
    let wrapped = B64
        .decode(&envelope.wrapped_key)
        .map_err(|_| "Invalid wrapped key".to_string())?;
    let key = open(kek, &nonce, KEY_WRAP_AAD, &wrapped)?;
    if key.expose_secret().len() != 32 {
        return Err("Unwrapped vault key has an unexpected length".to_string());
    }
    Ok(key)
}

pub fn unwrap_data_key_with_password(
    password: &Secret<String>,
    envelope: &KeyEnvelope,
) -> Result<Secret<Vec<u8>>, String> {
    let salt_vec = B64
        .decode(&envelope.salt)
        .map_err(|_| "Invalid key salt".to_string())?;
    let salt: [u8; crate::SALT_SIZE] = salt_vec
        .as_slice()
        .try_into()
        .map_err(|_| "Key salt must be 16 bytes".to_string())?;
    crate::validate_kdf_params(envelope.mem_cost, envelope.time_cost, envelope.parallelism)?;
    let kek = crate::derive_key(
        password,
        &salt,
        envelope.mem_cost,
        envelope.time_cost,
        envelope.parallelism,
    )?;
    unwrap_data_key(kek.expose_secret(), envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_roundtrip_and_aad_binding() {
        let key = generate_data_key();
        let sealed = seal_object(key.expose_secret(), "id-1", "entry", false, b"secret").unwrap();
        let opened = open_object(
            key.expose_secret(),
            "id-1",
            "entry",
            false,
            &sealed.nonce,
            &sealed.ciphertext,
        )
        .unwrap();
        assert_eq!(opened.expose_secret(), b"secret");
        assert!(open_object(
            key.expose_secret(),
            "id-2",
            "entry",
            false,
            &sealed.nonce,
            &sealed.ciphertext,
        )
        .is_err());
    }

    #[test]
    fn known_vector_is_stable() {
        let key = [7u8; 32];
        let nonce = [9u8; 24];
        let aad = b"v1|11111111-1111-1111-1111-111111111111|entry|0";
        let ciphertext = seal(&key, &nonce, aad, br#"{"title":"demo"}"#).unwrap();
        assert_eq!(B64.encode(ciphertext), "xVCW4wjGHEDOMGxZBRwlloAvMXGDQt3wwlgp++BhHmc=");
    }

    #[test]
    fn argon2_vector_matches_the_published_parameters() {
        let password = Secret::new("password".to_string());
        let key = crate::derive_key(&password, &[1u8; 16], 8192, 1, 1).unwrap();
        assert_eq!(
            B64.encode(key.expose_secret()),
            "GClVEXyykJF0geD14cGzUn0oayWjmvtvuMF0j3Ll/PA="
        );
    }

    #[test]
    fn wrap_roundtrip() {
        let kek = [7u8; 32];
        let data_key = generate_data_key();
        let envelope = wrap_data_key(kek.as_slice(), data_key.expose_secret(), &[1u8; 16], 65536, 3, 1)
            .unwrap();
        let opened = unwrap_data_key(kek.as_slice(), &envelope).unwrap();
        assert_eq!(opened.expose_secret(), data_key.expose_secret());
    }
}
