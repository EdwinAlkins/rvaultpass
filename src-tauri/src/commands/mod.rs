//! Tauri commands for IPC communication
//!
//! This module exposes typed commands to the frontend
//! All validation and security checks happen here

use serde::{Deserialize, Serialize};
use crate::vault::VaultManager;
use crate::db::operations;
use crate::models::{VaultEntry, VaultFolder, VaultTag, PasswordGenerationParams, TOTPResult};

/// Response wrapper for all commands
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> CommandResult<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

// ============================================================================
// Vault Lifecycle Commands
// ============================================================================

/// Create a new vault
#[tauri::command]
pub async fn create_vault(
    vault_path: String,
    master_password: String,
) -> CommandResult<bool> {
    tracing::info!("Creating vault at: {}", vault_path);

    match VaultManager::create_vault(&vault_path, &master_password) {
        Ok(_) => {
            tracing::info!("Vault created successfully");
            CommandResult::ok(true)
        }
        Err(e) => {
            tracing::error!("Failed to create vault: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Open an existing vault
#[tauri::command]
pub async fn open_vault(
    vault_path: String,
    master_password: String,
) -> CommandResult<bool> {
    tracing::info!("Opening vault at: {}", vault_path);

    match VaultManager::open_vault(&vault_path, &master_password) {
        Ok(_) => {
            tracing::info!("Vault opened successfully");
            CommandResult::ok(true)
        }
        Err(e) => {
            tracing::error!("Failed to open vault: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Lock the vault (clear encryption key from memory)
#[tauri::command]
pub async fn lock_vault() -> CommandResult<bool> {
    tracing::info!("Locking vault");

    match VaultManager::lock_vault() {
        Ok(_) => {
            tracing::info!("Vault locked successfully");
            CommandResult::ok(true)
        }
        Err(e) => {
            tracing::error!("Failed to lock vault: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Save vault (atomic write)
#[tauri::command]
pub async fn save_vault() -> CommandResult<bool> {
    tracing::info!("Saving vault");

    match VaultManager::save_vault() {
        Ok(_) => {
            tracing::info!("Vault saved successfully");
            CommandResult::ok(true)
        }
        Err(e) => {
            tracing::error!("Failed to save vault: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Check if vault is currently open
#[tauri::command]
pub async fn is_vault_open() -> CommandResult<bool> {
    CommandResult::ok(VaultManager::is_vault_open())
}

// ============================================================================
// Entry CRUD Commands
// ============================================================================

/// Create a new entry
#[tauri::command]
pub async fn create_entry(
    folder_id: Option<String>,
    title: String,
    url: Option<String>,
    username: Option<String>,
    password: String,
    notes: Option<String>,
    totp_secret: Option<String>,
    is_favorite: bool,
    tag_ids: Option<Vec<String>>,
) -> CommandResult<VaultEntry> {
    tracing::info!("Creating entry: {}", title);

    match operations::create_entry(folder_id, title, url, username, password, notes, totp_secret, is_favorite, tag_ids) {
        Ok(entry) => {
            tracing::info!("Entry created: {}", entry.id);
            CommandResult::ok(entry)
        }
        Err(e) => {
            tracing::error!("Failed to create entry: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Get all entries
#[tauri::command]
pub async fn get_all_entries() -> CommandResult<Vec<VaultEntry>> {
    match operations::get_all_entries() {
        Ok(entries) => CommandResult::ok(entries),
        Err(e) => {
            tracing::error!("Failed to get entries: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Get a single entry by ID
#[tauri::command]
pub async fn get_entry(entry_id: String) -> CommandResult<VaultEntry> {
    match operations::get_entry(&entry_id) {
        Ok(entry) => CommandResult::ok(entry),
        Err(e) => {
            tracing::error!("Failed to get entry: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Update an entry
#[tauri::command]
pub async fn update_entry(
    entry_id: String,
    folder_id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    username: Option<String>,
    password: Option<String>,
    notes: Option<String>,
    totp_secret: Option<String>,
    is_favorite: Option<bool>,
    tag_ids: Option<Vec<String>>,
) -> CommandResult<VaultEntry> {
    tracing::info!("Updating entry: {}", entry_id);

    // Fetch existing entry to get current values
    let existing = match operations::get_entry(&entry_id) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Failed to get existing entry: {}", e);
            return CommandResult::err(e);
        }
    };

    match operations::update_entry(
        entry_id,
        folder_id.or(existing.folder_id),
        title.unwrap_or(existing.title),
        url.or(existing.url),
        username.or(existing.username),
        password,
        notes.or(existing.notes),
        totp_secret.or(existing.totp_secret),
        is_favorite,
        tag_ids,
    ) {
        Ok(entry) => {
            tracing::info!("Entry updated: {}", entry.id);
            CommandResult::ok(entry)
        }
        Err(e) => {
            tracing::error!("Failed to update entry: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Delete an entry
#[tauri::command]
pub async fn delete_entry(entry_id: String) -> CommandResult<bool> {
    tracing::info!("Deleting entry: {}", entry_id);

    match operations::delete_entry(&entry_id) {
        Ok(_) => {
            tracing::info!("Entry deleted");
            CommandResult::ok(true)
        }
        Err(e) => {
            tracing::error!("Failed to delete entry: {}", e);
            CommandResult::err(e)
        }
    }
}

// ============================================================================
// Folder CRUD Commands
// ============================================================================

/// Create a new folder
#[tauri::command]
pub async fn create_folder(name: String, parent_id: Option<String>) -> CommandResult<VaultFolder> {
    tracing::info!("Creating folder: {}", name);

    match operations::create_folder(name, parent_id) {
        Ok(folder) => {
            tracing::info!("Folder created: {}", folder.id);
            CommandResult::ok(folder)
        }
        Err(e) => {
            tracing::error!("Failed to create folder: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Get all folders
#[tauri::command]
pub async fn get_all_folders() -> CommandResult<Vec<VaultFolder>> {
    match operations::get_all_folders() {
        Ok(folders) => CommandResult::ok(folders),
        Err(e) => {
            tracing::error!("Failed to get folders: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Update a folder
#[tauri::command]
pub async fn update_folder(
    folder_id: String,
    name: String,
    parent_id: Option<String>,
) -> CommandResult<VaultFolder> {
    tracing::info!("Updating folder: {}", folder_id);

    match operations::update_folder(&folder_id, name, parent_id) {
        Ok(folder) => {
            tracing::info!("Folder updated: {}", folder.id);
            CommandResult::ok(folder)
        }
        Err(e) => {
            tracing::error!("Failed to update folder: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Delete a folder
#[tauri::command]
pub async fn delete_folder(folder_id: String) -> CommandResult<bool> {
    tracing::info!("Deleting folder: {}", folder_id);

    match operations::delete_folder(&folder_id) {
        Ok(_) => {
            tracing::info!("Folder deleted");
            CommandResult::ok(true)
        }
        Err(e) => {
            tracing::error!("Failed to delete folder: {}", e);
            CommandResult::err(e)
        }
    }
}

// ============================================================================
// Tag CRUD Commands
// ============================================================================

/// Create a new tag
#[tauri::command]
pub async fn create_tag(name: String) -> CommandResult<VaultTag> {
    tracing::info!("Creating tag: {}", name);

    match operations::create_tag(name) {
        Ok(tag) => {
            tracing::info!("Tag created: {}", tag.id);
            CommandResult::ok(tag)
        }
        Err(e) => {
            tracing::error!("Failed to create tag: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Get all tags
#[tauri::command]
pub async fn get_all_tags() -> CommandResult<Vec<VaultTag>> {
    match operations::get_all_tags() {
        Ok(tags) => CommandResult::ok(tags),
        Err(e) => {
            tracing::error!("Failed to get tags: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Delete a tag
#[tauri::command]
pub async fn delete_tag(tag_id: String) -> CommandResult<bool> {
    tracing::info!("Deleting tag: {}", tag_id);

    match operations::delete_tag(&tag_id) {
        Ok(_) => {
            tracing::info!("Tag deleted");
            CommandResult::ok(true)
        }
        Err(e) => {
            tracing::error!("Failed to delete tag: {}", e);
            CommandResult::err(e)
        }
    }
}

// ============================================================================
// Search Commands
// ============================================================================

/// Search entries using full-text search
#[tauri::command]
pub async fn search_entries(query: String) -> CommandResult<Vec<VaultEntry>> {
    match operations::search_entries(&query) {
        Ok(entries) => CommandResult::ok(entries),
        Err(e) => {
            tracing::error!("Failed to search entries: {}", e);
            CommandResult::err(e)
        }
    }
}

// ============================================================================
// Password Generator Commands
// ============================================================================

/// Generate a password
#[tauri::command]
pub async fn generate_password(params: PasswordGenerationParams) -> CommandResult<String> {
    let password = operations::generate_password(&params);
    CommandResult::ok(password)
}

/// Generate a Diceware passphrase
#[tauri::command]
pub async fn generate_passphrase(word_count: usize) -> CommandResult<String> {
    let passphrase = operations::generate_passphrase(word_count);
    CommandResult::ok(passphrase)
}

// ============================================================================
// TOTP Commands
// ============================================================================

/// Generate a TOTP code
#[tauri::command]
pub async fn generate_totp(secret: String) -> CommandResult<TOTPResult> {
    match operations::generate_totp(&secret) {
        Ok(result) => CommandResult::ok(result),
        Err(e) => {
            tracing::error!("Failed to generate TOTP: {}", e);
            CommandResult::err(e)
        }
    }
}
