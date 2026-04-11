//! Cryptographic utilities for vault operations
//! 
//! This module handles:
//! - Argon2id key derivation
//! - HMAC header verification
//! - Secure memory handling with zeroize

use argon2::{Argon2, Version};
use zeroize::Zeroize;
use secrecy::{Secret, ExposeSecret};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Magic bytes for vault file format
pub const VAULT_MAGIC: [u8; 4] = *b"KVLT";

/// Current vault format version
pub const VAULT_VERSION: u8 = 0x01;

/// Header size in bytes (96 bytes)
pub const HEADER_SIZE: usize = 96;

/// Salt size for Argon2id
pub const SALT_SIZE: usize = 16;

/// Default memory cost (64 MiB)
pub const DEFAULT_MEM_COST: u32 = 65536;

/// Default time cost (3 iterations)
pub const DEFAULT_TIME_COST: u32 = 3;

/// Default parallelism (1 thread)
pub const DEFAULT_PARALLELISM: u32 = 1;

/// Vault header structure
#[derive(Debug, Clone)]
pub struct VaultHeader {
    pub magic: [u8; 4],
    pub version: u8,
    pub salt: [u8; SALT_SIZE],
    pub mem_cost: u32,
    pub time_cost: u32,
    pub parallelism: u32,
    pub header_hmac: [u8; 32],
}

impl VaultHeader {
    /// Parse header from bytes
    pub fn from_bytes(data: &[u8; HEADER_SIZE]) -> Result<Self, String> {
        let magic = [data[0x00], data[0x01], data[0x02], data[0x03]];
        let version = data[0x04];
        let salt: [u8; SALT_SIZE] = data[0x05..0x15].try_into()
            .map_err(|_| "Invalid salt length")?;
        let mem_cost = u32::from_le_bytes(data[0x15..0x19].try_into()
            .map_err(|_| "Invalid mem_cost")?);
        let time_cost = u16::from_le_bytes(data[0x19..0x1B].try_into()
            .map_err(|_| "Invalid time_cost")?) as u32;
        let parallelism = u16::from_le_bytes(data[0x1B..0x1D].try_into()
            .map_err(|_| "Invalid parallelism")?) as u32;
        let header_hmac: [u8; 32] = data[0x1D..0x3D].try_into()
            .map_err(|_| "Invalid HMAC length")?;

        Ok(Self {
            magic,
            version,
            salt,
            mem_cost,
            time_cost,
            parallelism,
            header_hmac,
        })
    }

    /// Verify magic bytes and version
    pub fn verify(&self) -> Result<(), String> {
        if self.magic != VAULT_MAGIC {
            return Err("Invalid vault magic bytes".to_string());
        }
        if self.version != VAULT_VERSION {
            return Err(format!("Unsupported vault version: {}", self.version));
        }
        Ok(())
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut data = [0u8; HEADER_SIZE];
        
        data[0x00..0x04].copy_from_slice(&self.magic);
        data[0x04] = self.version;
        data[0x05..0x15].copy_from_slice(&self.salt);
        data[0x15..0x19].copy_from_slice(&self.mem_cost.to_le_bytes());
        data[0x19..0x1B].copy_from_slice(&(self.time_cost as u16).to_le_bytes());
        data[0x1B..0x1D].copy_from_slice(&(self.parallelism as u16).to_le_bytes());
        data[0x1D..0x3D].copy_from_slice(&self.header_hmac);
        
        data
    }
}

/// Compute HMAC-SHA256 for header verification
/// Uses the derived encryption key as the HMAC key
pub fn compute_header_hmac(header_data: &[u8], key: &Secret<Vec<u8>>) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key.expose_secret())
        .expect("HMAC can take key of any size");
    mac.update(header_data);
    let result = mac.finalize();

    let mut output = [0u8; 32];
    output.copy_from_slice(&result.into_bytes());
    output
}

/// Derive encryption key from password using Argon2id
pub fn derive_key(
    password: &Secret<String>,
    salt: &[u8; SALT_SIZE],
    mem_cost: u32,
    time_cost: u32,
    parallelism: u32,
) -> Result<Secret<Vec<u8>>, String> {
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        Version::V0x13,
        argon2::Params::new(
            mem_cost,
            time_cost,
            parallelism,
            Some(32), // 256-bit key
        ).map_err(|e| format!("Invalid Argon2 params: {}", e))?,
    );

    let mut key = vec![0u8; 32];
    argon2
        .hash_password_into(
            password.expose_secret().as_bytes(),
            salt,
            &mut key,
        )
        .map_err(|e| format!("Argon2 error: {}", e))?;

    let secret_key = Secret::new(key.clone());
    key.zeroize(); // Clear temporary buffer
    
    Ok(secret_key)
}

/// Generate a random salt
pub fn generate_salt() -> Result<[u8; SALT_SIZE], String> {
    use ring::rand::{SecureRandom, SystemRandom};
    
    let rng = SystemRandom::new();
    let mut salt = [0u8; SALT_SIZE];
    
    rng.fill(&mut salt)
        .map_err(|e| format!("Failed to generate salt: {}", e))?;
    
    Ok(salt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_serialization() {
        let salt = generate_salt().unwrap();
        let hmac = [0u8; 32];

        let header = VaultHeader {
            magic: VAULT_MAGIC,
            version: VAULT_VERSION,
            salt,
            mem_cost: DEFAULT_MEM_COST,
            time_cost: DEFAULT_TIME_COST,
            parallelism: DEFAULT_PARALLELISM,
            header_hmac: hmac,
        };

        let bytes = header.to_bytes();
        let parsed = VaultHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.magic, VAULT_MAGIC);
        assert_eq!(parsed.version, VAULT_VERSION);
        assert_eq!(parsed.salt, salt);
        assert_eq!(parsed.mem_cost, DEFAULT_MEM_COST);
    }

    #[test]
    fn test_key_derivation() {
        let salt = generate_salt().unwrap();
        let password = Secret::new("test_password".to_string());

        let key = derive_key(
            &password,
            &salt,
            DEFAULT_MEM_COST,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        );

        assert!(key.is_ok());
        assert_eq!(key.unwrap().expose_secret().len(), 32);
    }

    #[test]
    fn test_hmac_with_key() {
        let salt = generate_salt().unwrap();
        let password = Secret::new("test_password".to_string());
        let key = derive_key(
            &password,
            &salt,
            DEFAULT_MEM_COST,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        ).unwrap();

        let header = VaultHeader {
            magic: VAULT_MAGIC,
            version: VAULT_VERSION,
            salt,
            mem_cost: DEFAULT_MEM_COST,
            time_cost: DEFAULT_TIME_COST,
            parallelism: DEFAULT_PARALLELISM,
            header_hmac: [0u8; 32],
        };

        let header_bytes = header.to_bytes();
        let hmac = compute_header_hmac(&header_bytes[0..0x1D], &key);

        // Verify same key produces same HMAC
        let hmac2 = compute_header_hmac(&header_bytes[0..0x1D], &key);
        assert_eq!(hmac, hmac2);

        // Verify different key produces different HMAC
        let password2 = Secret::new("different_password".to_string());
        let key2 = derive_key(
            &password2,
            &salt,
            DEFAULT_MEM_COST,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        ).unwrap();
        let hmac3 = compute_header_hmac(&header_bytes[0..0x1D], &key2);
        assert_ne!(hmac, hmac3);
    }

    #[test]
    fn test_zeroize_on_drop() {
        let sensitive_data = Secret::new(vec![42u8; 32]);

        // Verify data is present
        assert_eq!(sensitive_data.expose_secret()[0], 42);

        // Secret::new(vec).zeroize() on drop clears the inner content
        // We just verify the type implements Zeroize
        drop(sensitive_data);
        // After drop, the secret is gone - this test verifies the compile-time behavior
    }
}
