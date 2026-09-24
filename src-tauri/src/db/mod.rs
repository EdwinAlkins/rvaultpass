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
    let conn = Connection::open(vault_path)
        .map_err(|e| format!("Failed to open vault database: {}", e))?;

    let key_hex = bytes_to_hex(encryption_key.expose_secret());
    conn.pragma_update(None, "key", format!("x'{}'", key_hex))
        .map_err(|e| format!("Failed to set encryption key: {}", e))?;

    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;

    // Force SQLCipher to decrypt/initialize before any schema work.
    // Without this, the first DDL batch can fail with "file is not a database".
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
        row.get::<_, i64>(0)
    })
    .map_err(|e| {
        format!(
            "Failed to unlock database (wrong password or corrupt file?): {}",
            e
        )
    })?;

    Ok(conn)
}

/// Create a new vault file with initial schema
pub fn create_new_vault(vault_path: &Path, encryption_key: &Secret<Vec<u8>>) -> Result<(), String> {
    if let Some(parent) = vault_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create vault directory: {}", e))?;
    }

    // Ensure we start from an empty file (avoid leftover plaintext/corrupt bytes)
    if vault_path.exists() {
        std::fs::remove_file(vault_path)
            .map_err(|e| format!("Failed to replace existing temp database: {}", e))?;
    }

    let conn = init_connection(vault_path, encryption_key)?;
    run_migrations(&conn)?;
    Ok(())
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type IN ('table', 'view') AND name = ?1 LIMIT 1",
        [name],
        |_| Ok(true),
    )
    .optional_row()
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({})", table))
        .map_err(|e| format!("Failed to inspect table {}: {}", table, e))?;

    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("Failed to read columns for {}: {}", table, e))?;

    for row in rows {
        let name = row.map_err(|e| format!("Failed to read column name: {}", e))?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Read schema version. Returns 0 only when the schema has not been created yet.
fn read_schema_version(conn: &Connection) -> Result<i32, String> {
    if !table_exists(conn, "meta")? {
        return Ok(0);
    }

    match conn.query_row(
        "SELECT CAST(value AS INTEGER) FROM meta WHERE key = 'version'",
        [],
        |row| row.get::<_, i32>(0),
    ) {
        Ok(v) => Ok(v),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(1),
        Err(e) => Err(format!("Failed to read schema version: {}", e)),
    }
}

fn set_schema_version(conn: &Connection, version: i32) -> Result<(), String> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('version', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [version.to_string()],
    )
    .map_err(|e| format!("Failed to set schema version: {}", e))?;
    Ok(())
}

/// Run database migrations based on actual schema state (not guessed from errors).
/// Returns true if the schema was modified (caller should persist the vault).
pub fn run_migrations(conn: &Connection) -> Result<bool, String> {
    let mut changed = false;
    let version = read_schema_version(conn)?;

    // Fresh database: create core schema once.
    // Never re-run 001 on an existing vault — that produced "file is not a database"
    // when version detection incorrectly fell back to 0.
    if version < 1 {
        if table_exists(conn, "entries")? {
            // Recover partial vaults that have tables but no usable version row.
            set_schema_version(conn, 1)?;
            changed = true;
        } else {
            let migration_001 = include_str!("../../migrations/001_create_core_tables.up.sql");
            conn.execute_batch(migration_001)
                .map_err(|e| format!("Migration 001 failed: {}", e))?;
            tracing::info!("Applied migration 001");
            changed = true;
        }
    }

    // Add username2 / username3 when missing (Dashlane import fields).
    if table_exists(conn, "entries")? && !column_exists(conn, "entries", "username2")? {
        conn.execute_batch(
            "ALTER TABLE entries ADD COLUMN username2 TEXT;
             ALTER TABLE entries ADD COLUMN username3 TEXT;",
        )
        .map_err(|e| format!("Migration 002 failed: {}", e))?;
        set_schema_version(conn, 2)?;
        tracing::info!("Applied migration 002 (username2/username3)");
        changed = true;
    } else if read_schema_version(conn)? < 2 && column_exists(conn, "entries", "username2")? {
        set_schema_version(conn, 2)?;
        changed = true;
    }

    tracing::info!("Database migrations completed successfully");
    Ok(changed)
}

/// Convert bytes to hex string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Test if a vault file can be opened with the given key
pub fn test_vault_connection(vault_path: &Path, encryption_key: &Secret<Vec<u8>>) -> Result<bool, String> {
    let conn = init_connection(vault_path, encryption_key)?;

    // Any successful decrypt + sqlite_master read already happened in init_connection.
    // Confirm core schema exists for existing vaults.
    if !table_exists(&conn, "meta")? && !table_exists(&conn, "entries")? {
        return Err("Database is empty or not initialized".to_string());
    }

    Ok(true)
}

/// Helper: rusqlite query_row → Option without duplicating error mapping
trait OptionalRow<T> {
    fn optional_row(self) -> Result<bool, String>;
}

impl OptionalRow<bool> for Result<bool, rusqlite::Error> {
    fn optional_row(self) -> Result<bool, String> {
        match self {
            Ok(_) => Ok(true),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(format!("Schema probe failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{
        derive_key, generate_salt, DEFAULT_MEM_COST, DEFAULT_PARALLELISM, DEFAULT_TIME_COST,
    };

    #[test]
    fn test_create_and_open_vault() {
        let temp_dir = std::env::temp_dir();
        let vault_path = temp_dir.join("test_vault_db.vault");

        let _ = std::fs::remove_file(&vault_path);

        let salt = generate_salt().unwrap();
        let password = Secret::new("test_password".to_string());
        let key = derive_key(
            &password,
            &salt,
            DEFAULT_MEM_COST,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        )
        .unwrap();

        let result = create_new_vault(&vault_path, &key);
        if let Err(ref e) = result {
            eprintln!("create_new_vault error: {}", e);
        }
        assert!(result.is_ok());

        let test_result = test_vault_connection(&vault_path, &key);
        if let Err(ref e) = test_result {
            eprintln!("test_vault_connection error: {}", e);
        }
        assert!(test_result.is_ok());
        assert!(test_result.unwrap());

        // Re-open and migrate again must be idempotent
        let conn = init_connection(&vault_path, &key).unwrap();
        let changed = run_migrations(&conn).unwrap();
        assert!(!changed, "second migration pass should be a no-op");

        let _ = std::fs::remove_file(&vault_path);
    }

    #[test]
    fn test_migration_adds_username_columns() {
        let temp_dir = std::env::temp_dir();
        let vault_path = temp_dir.join("test_vault_migrate_username.vault");
        let _ = std::fs::remove_file(&vault_path);

        let salt = generate_salt().unwrap();
        let password = Secret::new("test_password".to_string());
        let key = derive_key(
            &password,
            &salt,
            DEFAULT_MEM_COST,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        )
        .unwrap();

        // Create a v1-like schema without username2/username3
        let conn = init_connection(&vault_path, &key).unwrap();
        conn.execute_batch(
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO meta (key, value) VALUES ('version', '1');
             CREATE TABLE entries (
                id BLOB PRIMARY KEY,
                folder_id BLOB,
                title TEXT NOT NULL,
                url TEXT,
                username TEXT,
                password TEXT NOT NULL,
                notes TEXT,
                totp_secret TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                is_favorite INTEGER DEFAULT 0
             );",
        )
        .unwrap();
        drop(conn);

        let conn = init_connection(&vault_path, &key).unwrap();
        let changed = run_migrations(&conn).unwrap();
        assert!(changed);
        assert!(column_exists(&conn, "entries", "username2").unwrap());
        assert!(column_exists(&conn, "entries", "username3").unwrap());
        assert_eq!(read_schema_version(&conn).unwrap(), 2);

        let _ = std::fs::remove_file(&vault_path);
    }
}
