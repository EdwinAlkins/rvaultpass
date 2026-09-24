//! Vault manager module
//! 
//! Handles vault lifecycle operations:
//! - Creation and opening
//! - Locking and unlocking
//! - Atomic saves with backup

use std::path::Path;
use std::sync::Mutex;
use secrecy::Secret;
use once_cell::sync::Lazy;

use crate::crypto::{
    self, VaultHeader, generate_salt, derive_key,
    DEFAULT_MEM_COST, DEFAULT_TIME_COST, DEFAULT_PARALLELISM,
    VAULT_MAGIC, VAULT_VERSION, HEADER_SIZE,
};
use crate::db;

/// Global vault state
static VAULT_STATE: Lazy<Mutex<Option<VaultInstance>>> = Lazy::new(|| Mutex::new(None));

/// Active vault instance
#[allow(dead_code)]
struct VaultInstance {
    /// Path to the original vault file (header + db)
    vault_path: String,
    /// Path to the extracted temporary database
    db_path: String,
    encryption_key: Secret<Vec<u8>>,
    header: VaultHeader,
}

/// Vault manager for high-level operations
pub struct VaultManager;

impl VaultManager {
    /// Create a new vault file
    pub fn create_vault(vault_path: &str, master_password: &str) -> Result<(), String> {
        let path = Path::new(vault_path);

        // Check if file already exists
        if path.exists() {
            return Err("Vault file already exists".to_string());
        }

        // Generate salt
        let salt = generate_salt()?;

        // Derive encryption key first (needed for HMAC)
        let password = Secret::new(master_password.to_string());
        let key = derive_key(&password, &salt, DEFAULT_MEM_COST, DEFAULT_TIME_COST, DEFAULT_PARALLELISM)?;

        // Create header (without HMAC initially)
        let header = VaultHeader {
            magic: VAULT_MAGIC,
            version: VAULT_VERSION,
            salt,
            mem_cost: DEFAULT_MEM_COST,
            time_cost: DEFAULT_TIME_COST,
            parallelism: DEFAULT_PARALLELISM,
            header_hmac: [0u8; 32],
        };

        // Compute HMAC over header data (excluding the HMAC field itself)
        let header_bytes = header.to_bytes();
        let hmac = crypto::compute_header_hmac(&header_bytes[0..0x1D], &key);

        let mut final_header = header;
        final_header.header_hmac = hmac;

        // Step 1: Create the database at a temporary path
        let tmp_path = path.with_extension("vault.tmp.db");
        db::create_new_vault(&tmp_path, &key)?;

        // Step 2: Read the database content
        let db_content = std::fs::read(&tmp_path)
            .map_err(|e| format!("Failed to read temporary database: {}", e))?;

        // Step 3: Write header + database content to final vault file
        let mut vault_content = final_header.to_bytes().to_vec();
        vault_content.extend_from_slice(&db_content);

        std::fs::write(path, vault_content)
            .map_err(|e| format!("Failed to write vault file: {}", e))?;

        // Keep the temp DB as the working copy (don't delete it)
        // Store in global state
        let vault_instance = VaultInstance {
            vault_path: vault_path.to_string(),
            db_path: tmp_path.to_string_lossy().to_string(),
            encryption_key: key,
            header: final_header,
        };

        let mut state = VAULT_STATE.lock().unwrap();
        *state = Some(vault_instance);

        Ok(())
    }
    
    /// Open an existing vault
    pub fn open_vault(vault_path: &str, master_password: &str) -> Result<(), String> {
        let path = Path::new(vault_path);

        // Check if file exists
        if !path.exists() {
            return Err("Vault file not found".to_string());
        }

        // Read the full vault file
        let vault_data = std::fs::read(path)
            .map_err(|e| format!("Failed to read vault file: {}", e))?;

        if vault_data.len() < HEADER_SIZE {
            return Err("Vault file too small".to_string());
        }

        // Parse header
        let header_array: [u8; HEADER_SIZE] = vault_data[0..HEADER_SIZE]
            .try_into()
            .map_err(|_| "Failed to parse vault header")?;

        let header = VaultHeader::from_bytes(&header_array)?;
        header.verify()?;

        // Bound KDF params BEFORE Argon2 to prevent DoS via malicious headers
        crypto::validate_kdf_params(header.mem_cost, header.time_cost, header.parallelism)?;

        // Derive encryption key first (needed for HMAC verification)
        let password = Secret::new(master_password.to_string());
        let key = derive_key(&password, &header.salt, header.mem_cost, header.time_cost, header.parallelism)?;

        // Verify HMAC
        let computed_hmac = crypto::compute_header_hmac(&vault_data[0..0x1D], &key);
        if computed_hmac != header.header_hmac {
            return Err("Vault header integrity check failed".to_string());
        }

        // Extract the database portion (everything after the 96-byte header)
        let db_data = &vault_data[HEADER_SIZE..];
        if db_data.is_empty() {
            return Err("Vault file has no database payload".to_string());
        }

        // Write the database to a temporary path for use
        let tmp_path = path.with_extension("vault.tmp.db");
        if tmp_path.exists() {
            let _ = std::fs::remove_file(&tmp_path);
        }
        std::fs::write(&tmp_path, db_data)
            .map_err(|e| format!("Failed to extract database: {}", e))?;

        // Unlock + migrate on a single connection path
        db::test_vault_connection(&tmp_path, &key)?;
        let schema_changed = {
            let conn = db::init_connection(&tmp_path, &key)?;
            db::run_migrations(&conn)?
        };

        // Store in global state
        let vault_instance = VaultInstance {
            vault_path: vault_path.to_string(),
            db_path: tmp_path.to_string_lossy().to_string(),
            encryption_key: key,
            header,
        };

        let mut state = VAULT_STATE.lock().unwrap();
        // Drop previous session if any (avoid stale temp DB / key)
        if let Some(prev) = state.take() {
            let _ = std::fs::remove_file(Path::new(&prev.db_path));
        }
        *state = Some(vault_instance);
        drop(state);

        // Persist schema upgrades into the .vault file immediately
        if schema_changed {
            VaultManager::save_vault()?;
        }

        Ok(())
    }
    
    /// Lock the vault (clear key from memory)
    pub fn lock_vault() -> Result<(), String> {
        let mut state = VAULT_STATE.lock().unwrap();
        if let Some(vault) = state.take() {
            // Clean up temporary database
            let db_path = Path::new(&vault.db_path);
            let _ = std::fs::remove_file(db_path);
        }
        Ok(())
    }
    
    /// Save vault with atomic write and backup rotation (.bak.1 to .bak.3)
    pub fn save_vault() -> Result<(), String> {
        let state = VAULT_STATE.lock().unwrap();
        let vault = state.as_ref()
            .ok_or("No vault is currently open")?;

        let vault_path = Path::new(&vault.vault_path);
        let db_path = Path::new(&vault.db_path);
        let tmp_path = vault_path.with_extension("vault.tmp");
        let bak1 = vault_path.with_extension("vault.bak.1");
        let bak2 = vault_path.with_extension("vault.bak.2");
        let bak3 = vault_path.with_extension("vault.bak.3");

        // Read the working database content
        let db_content = std::fs::read(db_path)
            .map_err(|e| format!("Failed to read working database: {}", e))?;

        // Build vault file: header + database
        let mut vault_content = vault.header.to_bytes().to_vec();
        vault_content.extend_from_slice(&db_content);

        // Rotate backups: .bak.2 -> .bak.3, .bak.1 -> .bak.2
        if bak2.exists() {
            std::fs::rename(&bak2, &bak3)
                .map_err(|e| format!("Failed to rotate backup: {}", e))?;
        }
        if bak1.exists() {
            std::fs::rename(&bak1, &bak2)
                .map_err(|e| format!("Failed to rotate backup: {}", e))?;
        }

        // Create .bak.1 from current vault file
        if vault_path.exists() {
            std::fs::copy(vault_path, &bak1)
                .map_err(|e| format!("Failed to create backup: {}", e))?;
        }

        // Write to tmp file
        std::fs::write(&tmp_path, vault_content)
            .map_err(|e| format!("Failed to prepare save: {}", e))?;

        // Atomic rename
        std::fs::rename(&tmp_path, vault_path)
            .map_err(|e| format!("Failed to complete save: {}", e))?;

        // Clean up tmp
        let _ = std::fs::remove_file(&tmp_path);

        Ok(())
    }
    
    /// Check if vault is currently open
    pub fn is_vault_open() -> bool {
        let state = VAULT_STATE.lock().unwrap();
        state.is_some()
    }

    /// Get the active database connection
    pub fn get_connection() -> Result<rusqlite::Connection, String> {
        let state = VAULT_STATE.lock().unwrap();
        let vault = state.as_ref()
            .ok_or("No vault is currently open")?;

        let db_path = Path::new(&vault.db_path);
        db::init_connection(db_path, &vault.encryption_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cleanup(path: &str) {
        let p = Path::new(path);
        let _ = std::fs::remove_file(p);
        let _ = std::fs::remove_file(p.with_extension("vault.tmp.db"));
        let _ = std::fs::remove_file(p.with_extension("vault.bak.1"));
        let _ = std::fs::remove_file(p.with_extension("vault.bak.2"));
        let _ = std::fs::remove_file(p.with_extension("vault.bak.3"));
    }

    #[test]
    fn test_full_vault_lifecycle() {
        let temp_dir = std::env::temp_dir();
        let vault_path = temp_dir.join("test_lifecycle.vault");
        let vault_str = vault_path.to_str().unwrap();

        // Ensure clean state
        cleanup(vault_str);

        // 1. Create vault
        let result = VaultManager::create_vault(vault_str, "master_password_123");
        assert!(result.is_ok(), "Failed to create vault: {:?}", result);
        assert!(VaultManager::is_vault_open(), "Vault should be open after creation");
        assert!(vault_path.exists(), "Vault file should exist");

        // 2. Save vault
        let result = VaultManager::save_vault();
        assert!(result.is_ok(), "Failed to save vault: {:?}", result);
        assert!(vault_path.exists(), "Vault file should still exist after save");

        // 3. Lock vault
        let result = VaultManager::lock_vault();
        assert!(result.is_ok(), "Failed to lock vault: {:?}", result);
        assert!(!VaultManager::is_vault_open(), "Vault should be closed after lock");

        // 4. Re-open vault with correct password
        let result = VaultManager::open_vault(vault_str, "master_password_123");
        assert!(result.is_ok(), "Failed to reopen vault: {:?}", result);
        assert!(VaultManager::is_vault_open(), "Vault should be open after reopening");

        // 5. Try to open with wrong password (should fail)
        VaultManager::lock_vault().ok();
        let result = VaultManager::open_vault(vault_str, "wrong_password");
        assert!(result.is_err(), "Should fail to open with wrong password");
        assert!(!VaultManager::is_vault_open(), "Vault should not be open after failed auth");

        // Cleanup
        VaultManager::lock_vault().ok();
        cleanup(vault_str);
    }

    #[test]
    fn test_vault_backup_rotation() {
        let temp_dir = std::env::temp_dir();
        let vault_path = temp_dir.join("test_backup.vault");
        let vault_str = vault_path.to_str().unwrap();

        cleanup(vault_str);

        // Create vault
        VaultManager::create_vault(vault_str, "test_pass").unwrap();

        // Save 3 times to trigger full rotation
        VaultManager::save_vault().unwrap();
        assert!(vault_path.with_extension("vault.bak.1").exists(), "First backup should exist");

        VaultManager::save_vault().unwrap();
        assert!(vault_path.with_extension("vault.bak.1").exists(), ".bak.1 should exist after 2 saves");
        assert!(vault_path.with_extension("vault.bak.2").exists(), ".bak.2 should exist after 2 saves");

        VaultManager::save_vault().unwrap();
        assert!(vault_path.with_extension("vault.bak.1").exists(), ".bak.1 should exist after 3 saves");
        assert!(vault_path.with_extension("vault.bak.2").exists(), ".bak.2 should exist after 3 saves");
        assert!(vault_path.with_extension("vault.bak.3").exists(), ".bak.3 should exist after 3 saves");

        // Cleanup
        VaultManager::lock_vault().ok();
        cleanup(vault_str);
    }

    #[test]
    fn test_create_duplicate_vault_fails() {
        let temp_dir = std::env::temp_dir();
        let vault_path = temp_dir.join("test_dup.vault");
        let vault_str = vault_path.to_str().unwrap();

        cleanup(vault_str);

        let result1 = VaultManager::create_vault(vault_str, "pass1");
        assert!(result1.is_ok());

        let result2 = VaultManager::create_vault(vault_str, "pass2");
        assert!(result2.is_err(), "Creating duplicate vault should fail");
        assert!(result2.unwrap_err().contains("already exists"));

        VaultManager::lock_vault().ok();
        cleanup(vault_str);
    }

    #[test]
    fn test_vault_data_persists_across_sessions() {
        let temp_dir = std::env::temp_dir();
        let vault_path = temp_dir.join("test_persist.vault");
        let vault_str = vault_path.to_str().unwrap();

        cleanup(vault_str);

        // Session 1: Create and populate
        VaultManager::create_vault(vault_str, "session_pass").unwrap();

        let conn = VaultManager::get_connection().unwrap();
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('test_key', 'test_value')",
            [],
        ).unwrap();
        drop(conn);

        // Save and lock
        VaultManager::save_vault().unwrap();
        VaultManager::lock_vault().unwrap();

        // Session 2: Re-open and verify data
        VaultManager::open_vault(vault_str, "session_pass").unwrap();
        let conn = VaultManager::get_connection().unwrap();

        let value: String = conn.query_row(
            "SELECT value FROM meta WHERE key = 'test_key'",
            [],
            |row| row.get(0),
        ).unwrap();

        assert_eq!(value, "test_value", "Data should persist across sessions");

        // Cleanup
        drop(conn);
        VaultManager::lock_vault().ok();
        cleanup(vault_str);
    }
}
