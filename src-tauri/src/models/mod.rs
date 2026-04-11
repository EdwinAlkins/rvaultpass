//! Data models for vault entries, folders, and tags

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// A vault entry (password, note, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub id: String,
    pub folder_id: Option<String>,
    pub title: String,
    pub url: Option<String>,
    pub username: Option<String>,
    pub password: String,
    pub notes: Option<String>,
    pub totp_secret: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_favorite: bool,
    pub tags: Vec<VaultTag>,
}

/// A folder for organizing entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultFolder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A tag for categorizing entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultTag {
    pub id: String,
    pub name: String,
}

/// Password generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordGenerationParams {
    pub length: usize,
    pub use_uppercase: bool,
    pub use_lowercase: bool,
    pub use_numbers: bool,
    pub use_symbols: bool,
    pub exclude_ambiguous: bool,
}

/// TOTP generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TOTPResult {
    pub code: String,
    pub time_remaining: u64,
}
