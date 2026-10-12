//! Apply a decrypted remote object to the local SQLCipher cache.

use rusqlite::{params, Connection};
use rvault_core::{EntryPayload, FolderPayload, TagPayload, VaultEntry};
use uuid::Uuid;

use crate::operations::get_entry;

fn parse_uuid(value: &str) -> Result<Vec<u8>, String> {
    Uuid::parse_str(value)
        .map(|uuid| uuid.as_bytes().to_vec())
        .map_err(|_| format!("Invalid UUID: {}", value))
}

pub fn apply_entry(conn: &Connection, payload: &EntryPayload) -> Result<(), String> {
    let id = parse_uuid(&payload.id)?;
    let folder_id = payload
        .folder_id
        .as_ref()
        .map(|value| parse_uuid(value))
        .transpose()?;
    conn.execute(
        "INSERT INTO entries (
            id, folder_id, title, url, username, username2, username3, password, notes,
            totp_secret, created_at, updated_at, is_favorite
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(id) DO UPDATE SET
            folder_id = excluded.folder_id,
            title = excluded.title,
            url = excluded.url,
            username = excluded.username,
            username2 = excluded.username2,
            username3 = excluded.username3,
            password = excluded.password,
            notes = excluded.notes,
            totp_secret = excluded.totp_secret,
            created_at = excluded.created_at,
            updated_at = excluded.updated_at,
            is_favorite = excluded.is_favorite",
        params![
            id,
            folder_id,
            payload.title,
            payload.url,
            payload.username,
            payload.username2,
            payload.username3,
            payload.password,
            payload.notes,
            payload.totp_secret,
            payload.created_at,
            payload.updated_at,
            if payload.is_favorite { 1 } else { 0 },
        ],
    )
    .map_err(|e| format!("Failed to apply remote entry: {}", e))?;

    conn.execute("DELETE FROM entry_tags WHERE entry_id = ?1", params![id])
        .map_err(|e| format!("Failed to reset entry tags: {}", e))?;
    for tag_id in &payload.tag_ids {
        let tag = parse_uuid(tag_id)?;
        conn.execute(
            "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
            params![id, tag],
        )
        .map_err(|e| format!("Failed to apply entry tag: {}", e))?;
    }
    Ok(())
}

pub fn apply_folder(conn: &Connection, payload: &FolderPayload) -> Result<(), String> {
    let id = parse_uuid(&payload.id)?;
    let parent_id = payload
        .parent_id
        .as_ref()
        .map(|value| parse_uuid(value))
        .transpose()?;
    conn.execute(
        "INSERT INTO folders (id, name, parent_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            parent_id = excluded.parent_id,
            updated_at = excluded.updated_at",
        params![id, payload.name, parent_id, payload.created_at, payload.updated_at],
    )
    .map_err(|e| format!("Failed to apply remote folder: {}", e))?;
    Ok(())
}

pub fn apply_tag(conn: &Connection, payload: &TagPayload) -> Result<(), String> {
    let id = parse_uuid(&payload.id)?;
    conn.execute(
        "INSERT INTO tags (id, name) VALUES (?1, ?2)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name",
        params![id, payload.name],
    )
    .map_err(|e| format!("Failed to apply remote tag: {}", e))?;
    Ok(())
}

pub fn delete_synced(conn: &Connection, object_type: &str, object_id: &str) -> Result<(), String> {
    let id = parse_uuid(object_id)?;
    match object_type {
        "entry" => {
            conn.execute("DELETE FROM entries WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete synced entry: {}", e))?;
        }
        "folder" => {
            conn.execute(
                "UPDATE entries SET folder_id = NULL WHERE folder_id = ?1",
                params![id],
            )
            .map_err(|e| format!("Failed to detach synced folder: {}", e))?;
            conn.execute("DELETE FROM folders WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete synced folder: {}", e))?;
        }
        "tag" => {
            conn.execute("DELETE FROM tags WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete synced tag: {}", e))?;
        }
        other => return Err(format!("Unknown sync object type: {}", other)),
    }
    Ok(())
}

/// Read one entry, including secrets, for the sync client.
pub fn read_entry_secret(conn: &Connection, entry_id: &str) -> Result<VaultEntry, String> {
    get_entry(conn, entry_id).map_err(|error| error.to_string())
}
