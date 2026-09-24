//! Data models for vault entries, folders, and tags

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// A vault entry (password, note, etc.) — backend-only, never sent in bulk to UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub id: String,
    pub folder_id: Option<String>,
    pub title: String,
    pub url: Option<String>,
    pub username: Option<String>,
    pub username2: Option<String>,
    pub username3: Option<String>,
    pub password: String,
    pub notes: Option<String>,
    pub totp_secret: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_favorite: bool,
    pub tags: Vec<VaultTag>,
}

/// Safe entry summary for the frontend (no secrets)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrySummary {
    pub id: String,
    pub folder_id: Option<String>,
    pub title: String,
    pub url: Option<String>,
    pub username: Option<String>,
    pub username2: Option<String>,
    pub username3: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_favorite: bool,
    pub tags: Vec<VaultTag>,
    pub has_password: bool,
    pub has_totp: bool,
    pub has_notes: bool,
}

/// Editable metadata without password / TOTP secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryEditData {
    pub id: String,
    pub folder_id: Option<String>,
    pub title: String,
    pub url: Option<String>,
    pub username: Option<String>,
    pub username2: Option<String>,
    pub username3: Option<String>,
    pub notes: Option<String>,
    pub is_favorite: bool,
    pub tags: Vec<VaultTag>,
    pub has_password: bool,
    pub has_totp: bool,
}

impl VaultEntry {
    pub fn to_summary(&self) -> EntrySummary {
        EntrySummary {
            id: self.id.clone(),
            folder_id: self.folder_id.clone(),
            title: self.title.clone(),
            url: self.url.clone(),
            username: self.username.clone(),
            username2: self.username2.clone(),
            username3: self.username3.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            is_favorite: self.is_favorite,
            tags: self.tags.clone(),
            has_password: !self.password.is_empty(),
            has_totp: self
                .totp_secret
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false),
            has_notes: self
                .notes
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false),
        }
    }

    pub fn to_edit_data(&self) -> EntryEditData {
        EntryEditData {
            id: self.id.clone(),
            folder_id: self.folder_id.clone(),
            title: self.title.clone(),
            url: self.url.clone(),
            username: self.username.clone(),
            username2: self.username2.clone(),
            username3: self.username3.clone(),
            notes: self.notes.clone(),
            is_favorite: self.is_favorite,
            tags: self.tags.clone(),
            has_password: !self.password.is_empty(),
            has_totp: self
                .totp_secret
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false),
        }
    }
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
