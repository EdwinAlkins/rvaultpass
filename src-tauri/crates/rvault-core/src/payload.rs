//! Canonical plaintext objects. Field order is part of the sync contract.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::models::{VaultEntry, VaultFolder, VaultTag};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntryPayload {
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
    pub created_at: i64,
    pub updated_at: i64,
    pub is_favorite: bool,
    pub tag_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FolderPayload {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagPayload {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncPlaintext {
    Entry(EntryPayload),
    Folder(FolderPayload),
    Tag(TagPayload),
}

impl SyncPlaintext {
    pub fn object_type(&self) -> &'static str {
        match self {
            Self::Entry(_) => "entry",
            Self::Folder(_) => "folder",
            Self::Tag(_) => "tag",
        }
    }

    pub fn object_id(&self) -> &str {
        match self {
            Self::Entry(entry) => &entry.id,
            Self::Folder(folder) => &folder.id,
            Self::Tag(tag) => &tag.id,
        }
    }
}

pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|e| format!("Failed to encode sync object: {}", e))
}

pub fn content_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{:02x}", byte)).collect()
}

pub fn entry_payload(entry: &VaultEntry) -> EntryPayload {
    let mut tag_ids: Vec<String> = entry.tags.iter().map(|tag| tag.id.clone()).collect();
    tag_ids.sort();
    EntryPayload {
        id: entry.id.clone(),
        folder_id: entry.folder_id.clone(),
        title: entry.title.clone(),
        url: entry.url.clone(),
        username: entry.username.clone(),
        username2: entry.username2.clone(),
        username3: entry.username3.clone(),
        password: entry.password.clone(),
        notes: entry.notes.clone(),
        totp_secret: entry.totp_secret.clone(),
        created_at: entry.created_at.timestamp(),
        updated_at: entry.updated_at.timestamp(),
        is_favorite: entry.is_favorite,
        tag_ids,
    }
}

pub fn folder_payload(folder: &VaultFolder) -> FolderPayload {
    FolderPayload {
        id: folder.id.clone(),
        name: folder.name.clone(),
        parent_id: folder.parent_id.clone(),
        created_at: folder.created_at.timestamp(),
        updated_at: folder.updated_at.timestamp(),
    }
}

pub fn tag_payload(tag: &VaultTag) -> TagPayload {
    TagPayload {
        id: tag.id.clone(),
        name: tag.name.clone(),
    }
}

pub fn datetime_from_unix(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_ids_are_sorted_in_the_canonical_payload() {
        let entry = VaultEntry {
            id: "e".into(),
            folder_id: None,
            title: "t".into(),
            url: None,
            username: None,
            username2: None,
            username3: None,
            password: "secret".into(),
            notes: None,
            totp_secret: None,
            created_at: datetime_from_unix(10),
            updated_at: datetime_from_unix(11),
            is_favorite: false,
            tags: vec![
                VaultTag {
                    id: "b".into(),
                    name: "b".into(),
                },
                VaultTag {
                    id: "a".into(),
                    name: "a".into(),
                },
            ],
        };
        let payload = entry_payload(&entry);
        assert_eq!(payload.tag_ids, vec!["a".to_string(), "b".to_string()]);
        let bytes = canonical_json(&payload).unwrap();
        assert!(bytes.windows(8).any(|window| window == b"password"));
    }
}
