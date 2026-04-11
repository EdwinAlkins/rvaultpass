//! Database module for vault operations
//!
//! This module handles:
//! - SQLCipher database initialization
//! - Database migrations
//! - Secure connection management

pub mod operations;

use rusqlite::Connection;
use secrecy::{Secret, ExposeSecret};
use std::path::Path;

/// Initialize a new encrypted database connection
pub fn init_connection(
    vault_path: &Path,
    encryption_key: &Secret<Vec<u8>>,
) -> Result<Connection, String> {
    // Open connection
    let conn = Connection::open(vault_path)
        .map_err(|e| format!("Failed to open vault database: {}", e))?;

    // Set encryption key using hex literal for SQLCipher
    let key_hex = bytes_to_hex(encryption_key.expose_secret());
    conn.pragma_update(None, "key", format!("x'{}'", key_hex))
        .map_err(|e| format!("Failed to set encryption key: {}", e))?;

    // Verify encryption is working
    let cipher_provider: String = conn.pragma_query_value(None, "cipher_provider", |row| row.get(0))
        .map_err(|e| format!("Failed to verify cipher: {}", e))?;

    tracing::info!("SQLCipher provider: {}", cipher_provider);

    Ok(conn)
}

/// Create a new vault file with initial schema
pub fn create_new_vault(vault_path: &Path, encryption_key: &Secret<Vec<u8>>) -> Result<(), String> {
    // Create parent directory if needed
    if let Some(parent) = vault_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create vault directory: {}", e))?;
    }
    
    let conn = init_connection(vault_path, encryption_key)?;
    
    // Run initial schema
    run_migrations(&conn)?;
    
    Ok(())
}

/// Run database migrations
pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    // Migration 001: Create core tables
    let migration_001 = include_str!("../../migrations/001_create_core_tables.up.sql");
    
    conn.execute_batch(migration_001)
        .map_err(|e| format!("Migration 001 failed: {}", e))?;
    
    tracing::info!("Database migrations completed successfully");
    
    Ok(())
}

/// Convert bytes to hex string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Test if a vault file can be opened with the given key
pub fn test_vault_connection(vault_path: &Path, encryption_key: &Secret<Vec<u8>>) -> Result<bool, String> {
    let conn = init_connection(vault_path, encryption_key)?;
    
    // Try to query the meta table to verify decryption works
    let result: Result<String, _> = conn.query_row(
        "SELECT value FROM meta WHERE key = 'version'",
        [],
        |row| row.get(0),
    );
    
    match result {
        Ok(_) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(true), // Table might not exist yet
        Err(e) => Err(format!("Failed to verify vault: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{derive_key, generate_salt, DEFAULT_MEM_COST, DEFAULT_TIME_COST, DEFAULT_PARALLELISM};

    #[test]
    fn test_create_and_open_vault() {
        let temp_dir = std::env::temp_dir();
        let vault_path = temp_dir.join("test_vault_db.vault");

        // Clean up if exists
        let _ = std::fs::remove_file(&vault_path);

        let salt = generate_salt().unwrap();
        let password = Secret::new("test_password".to_string());
        let key = derive_key(
            &password,
            &salt,
            DEFAULT_MEM_COST,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        ).unwrap();

        // Create the DB first (without header)
        let result = create_new_vault(&vault_path, &key);
        if let Err(ref e) = result {
            eprintln!("create_new_vault error: {}", e);
        }
        assert!(result.is_ok());

        // Test connection
        let test_result = test_vault_connection(&vault_path, &key);
        if let Err(ref e) = test_result {
            eprintln!("test_vault_connection error: {}", e);
        }
        assert!(test_result.is_ok());
        assert!(test_result.unwrap());

        // Clean up
        let _ = std::fs::remove_file(&vault_path);
    }
}
