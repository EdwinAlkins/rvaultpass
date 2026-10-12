//! Tauri commands for IPC communication
//!
//! This module exposes typed commands to the frontend
//! All validation and security checks happen here

use serde::{Deserialize, Serialize};
use crate::clipboard;
use crate::import::{self, ImportSummary};
use crate::models::{
    EntryEditData, EntryPatch, EntrySummary, PasswordGenerationParams, TOTPResult, VaultFolder,
    VaultTag,
};
use crate::vault::VaultManager;

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

/// Lock the vault (flush changes, clear clipboard, destroy session)
#[tauri::command]
pub async fn lock_vault() -> CommandResult<bool> {
    tracing::info!("Locking vault");

    clipboard::clear_pending();

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

/// Idle lock in seconds. Zero disables the idle lock.
#[tauri::command]
pub async fn set_lock_preferences(idle_secs: i64, lock_on_blur: bool) -> CommandResult<bool> {
    VaultManager::set_lock_preferences(idle_secs, lock_on_blur);
    CommandResult::ok(true)
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
    username2: Option<String>,
    username3: Option<String>,
    password: String,
    notes: Option<String>,
    totp_secret: Option<String>,
    is_favorite: bool,
    tag_ids: Option<Vec<String>>,
) -> CommandResult<EntrySummary> {
    tracing::info!("Creating entry: {}", title);

    match VaultManager::create_entry(
        folder_id,
        title,
        url,
        username,
        username2,
        username3,
        password,
        notes,
        totp_secret,
        is_favorite,
        tag_ids,
    ) {
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

/// Get all entries (summaries only — no secrets)
#[tauri::command]
pub async fn get_all_entries() -> CommandResult<Vec<EntrySummary>> {
    match VaultManager::list_entries() {
        Ok(entries) => CommandResult::ok(entries),
        Err(e) => {
            tracing::error!("Failed to get entries: {}", e);
            CommandResult::err(e)
        }
    }
}

/// Get editable metadata for an entry (no password / TOTP secret)
#[tauri::command]
pub async fn get_entry(entry_id: String) -> CommandResult<EntryEditData> {
    match VaultManager::edit_entry(&entry_id) {
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
    username2: Option<String>,
    username3: Option<String>,
    password: Option<String>,
    notes: Option<String>,
    totp_secret: Option<String>,
    is_favorite: Option<bool>,
    tag_ids: Option<Vec<String>>,
    ) -> CommandResult<EntrySummary> {
    tracing::info!("Updating entry: {}", entry_id);

    let password = password.filter(|value| !value.is_empty());
    let totp_secret = match totp_secret {
        Some(value) if value.is_empty() => Some(None),
        Some(value) => Some(Some(value)),
        None => None,
    };
    let patch = EntryPatch {
        folder_id: Some(folder_id),
        title,
        url: Some(url),
        username: Some(username),
        username2: Some(username2),
        username3: Some(username3),
        password,
        notes: Some(notes),
        totp_secret,
        is_favorite,
        tag_ids,
    };

    match VaultManager::update_entry(entry_id, patch) {
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

    match VaultManager::delete_entry(&entry_id) {
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

    match VaultManager::create_folder(name, parent_id) {
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
    match VaultManager::list_folders() {
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

    match VaultManager::update_folder(&folder_id, name, parent_id) {
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

    match VaultManager::delete_folder(&folder_id) {
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

    match VaultManager::create_tag(name) {
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
    match VaultManager::list_tags() {
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

    match VaultManager::delete_tag(&tag_id) {
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

/// Search entries using full-text search (summaries only)
#[tauri::command]
pub async fn search_entries(query: String) -> CommandResult<Vec<EntrySummary>> {
    match VaultManager::search(&query) {
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
    match VaultManager::generate_password(&params) {
        Ok(password) => CommandResult::ok(password),
        Err(e) => CommandResult::err(e),
    }
}

/// Generate a Diceware passphrase
#[tauri::command]
pub async fn generate_passphrase(word_count: usize) -> CommandResult<String> {
    match VaultManager::generate_passphrase(word_count) {
        Ok(passphrase) => CommandResult::ok(passphrase),
        Err(e) => CommandResult::err(e),
    }
}

// ============================================================================
// Secret access (never bulk-exported to frontend)
// ============================================================================

/// Reveal password briefly for display (caller must not persist)
#[tauri::command]
pub async fn reveal_password(entry_id: String) -> CommandResult<String> {
    match VaultManager::get_password(&entry_id) {
        Ok(password) => CommandResult::ok(password),
        Err(e) => CommandResult::err(e),
    }
}

/// Copy password to clipboard with auto-clear
#[tauri::command]
pub async fn copy_password(entry_id: String) -> CommandResult<bool> {
    match VaultManager::get_entry_secret(&entry_id) {
        Ok(entry) => match clipboard::copy_secret(&entry.password) {
            Ok(()) => CommandResult::ok(true),
            Err(e) => CommandResult::err(e),
        },
        Err(e) => CommandResult::err(e),
    }
}

/// Copy an arbitrary non-secret field via secure clipboard (username, url, notes…)
#[tauri::command]
pub async fn copy_entry_field(entry_id: String, field: String) -> CommandResult<bool> {
    let entry = match VaultManager::get_entry_secret(&entry_id) {
        Ok(e) => e,
        Err(e) => return CommandResult::err(e),
    };

    let value = match field.as_str() {
        "username" => entry.username.unwrap_or_default(),
        "username2" => entry.username2.unwrap_or_default(),
        "username3" => entry.username3.unwrap_or_default(),
        "url" => entry.url.unwrap_or_default(),
        "notes" => entry.notes.unwrap_or_default(),
        "password" => entry.password,
        _ => return CommandResult::err(format!("Unknown field: {}", field)),
    };

    if value.is_empty() {
        return CommandResult::err("Field is empty".to_string());
    }

    match clipboard::copy_secret(&value) {
        Ok(()) => CommandResult::ok(true),
        Err(e) => CommandResult::err(e),
    }
}

/// Generate TOTP for an entry (secret stays in Rust)
#[tauri::command]
pub async fn generate_totp_for_entry(entry_id: String) -> CommandResult<TOTPResult> {
    match VaultManager::generate_totp(&entry_id) {
        Ok(result) => CommandResult::ok(result),
        Err(e) => CommandResult::err(e),
    }
}

/// Copy current TOTP code to clipboard with auto-clear
#[tauri::command]
pub async fn copy_totp(entry_id: String) -> CommandResult<bool> {
    match VaultManager::generate_totp(&entry_id) {
        Ok(result) => match clipboard::copy_secret(&result.code) {
            Ok(()) => CommandResult::ok(true),
            Err(e) => CommandResult::err(e),
        },
        Err(e) => CommandResult::err(e),
    }
}

/// Copy generated text (password generator) via secure clipboard
#[tauri::command]
pub async fn copy_to_clipboard(text: String) -> CommandResult<bool> {
    if text.len() > 8192 {
        return CommandResult::err("Text too long to copy".to_string());
    }
    match clipboard::copy_secret(&text) {
        Ok(()) => CommandResult::ok(true),
        Err(e) => CommandResult::err(e),
    }
}

// ============================================================================
// Import Commands
// ============================================================================

/// Import entries from a Dashlane credentials.csv file
#[tauri::command]
pub async fn import_dashlane_csv(path: String) -> CommandResult<ImportSummary> {
    tracing::info!("Importing Dashlane CSV");

    match VaultManager::import(|conn| import::import_dashlane_csv(conn, std::path::Path::new(&path))) {
        Ok(summary) => {
            tracing::info!(
                "Dashlane import done: {} imported, {} skipped",
                summary.imported,
                summary.skipped
            );
            CommandResult::ok(summary)
        }
        Err(e) => {
            tracing::error!("Dashlane import failed: {}", e);
            CommandResult::err(e)
        }
    }
}

