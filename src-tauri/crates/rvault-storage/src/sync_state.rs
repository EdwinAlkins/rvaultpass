//! Local bookkeeping for the sync client. Plaintext stays in the SQLCipher cache.

use rusqlite::{params, Connection};

#[derive(Debug, Clone)]
pub struct LocalRevision {
    pub object_id: String,
    pub object_type: String,
    pub server_version: i64,
    pub content_hash: String,
}

#[derive(Debug, Clone)]
pub struct LocalConflict {
    pub conflict_id: String,
    pub object_id: String,
    pub object_type: String,
    pub remote_version: i64,
    pub remote_device_id: String,
    pub remote_nonce: String,
    pub remote_ciphertext: String,
    pub remote_deleted: bool,
    pub local_canonical: String,
}

pub fn meta_get(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    meta_get_on(conn, key)
}

pub fn meta_set(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO sync_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| format!("Failed to store sync metadata: {}", e))?;
    Ok(())
}

pub fn meta_get_on(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    match conn.query_row(
        "SELECT value FROM sync_meta WHERE key = ?1",
        params![key],
        |row| row.get(0),
    ) {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Failed to read sync metadata: {}", e)),
    }
}

pub fn revision(conn: &Connection, object_id: &str) -> Result<Option<LocalRevision>, String> {
    match conn.query_row(
        "SELECT object_id, object_type, server_version, content_hash
         FROM sync_objects WHERE object_id = ?1",
        params![object_id],
        |row| {
            Ok(LocalRevision {
                object_id: row.get(0)?,
                object_type: row.get(1)?,
                server_version: row.get(2)?,
                content_hash: row.get(3)?,
            })
        },
    ) {
        Ok(revision) => Ok(Some(revision)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Failed to read sync revision: {}", e)),
    }
}

pub fn list_revisions(conn: &Connection) -> Result<Vec<LocalRevision>, String> {
    let mut stmt = conn
        .prepare("SELECT object_id, object_type, server_version, content_hash FROM sync_objects")
        .map_err(|e| format!("Failed to list sync revisions: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LocalRevision {
                object_id: row.get(0)?,
                object_type: row.get(1)?,
                server_version: row.get(2)?,
                content_hash: row.get(3)?,
            })
        })
        .map_err(|e| format!("Failed to list sync revisions: {}", e))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to list sync revisions: {}", e))
}

pub fn put_revision(conn: &Connection, revision: &LocalRevision) -> Result<(), String> {
    conn.execute(
        "INSERT INTO sync_objects (object_id, object_type, server_version, content_hash)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(object_id) DO UPDATE SET
            object_type = excluded.object_type,
            server_version = excluded.server_version,
            content_hash = excluded.content_hash",
        params![
            revision.object_id,
            revision.object_type,
            revision.server_version,
            revision.content_hash
        ],
    )
    .map_err(|e| format!("Failed to store sync revision: {}", e))?;
    Ok(())
}

pub fn delete_revision(conn: &Connection, object_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM sync_objects WHERE object_id = ?1",
        params![object_id],
    )
    .map_err(|e| format!("Failed to delete sync revision: {}", e))?;
    Ok(())
}

pub fn save_conflict(conn: &Connection, conflict: &LocalConflict) -> Result<(), String> {
    conn.execute(
        "INSERT INTO sync_conflicts (
            conflict_id, object_id, object_type, remote_version, remote_device_id,
            remote_nonce, remote_ciphertext, remote_deleted, local_canonical, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, strftime('%s','now'))
         ON CONFLICT(conflict_id) DO UPDATE SET
            remote_version = excluded.remote_version,
            remote_nonce = excluded.remote_nonce,
            remote_ciphertext = excluded.remote_ciphertext,
            remote_deleted = excluded.remote_deleted,
            local_canonical = excluded.local_canonical",
        params![
            conflict.conflict_id,
            conflict.object_id,
            conflict.object_type,
            conflict.remote_version,
            conflict.remote_device_id,
            conflict.remote_nonce,
            conflict.remote_ciphertext,
            if conflict.remote_deleted { 1 } else { 0 },
            conflict.local_canonical,
        ],
    )
    .map_err(|e| format!("Failed to store sync conflict: {}", e))?;
    Ok(())
}

pub fn list_conflicts(conn: &Connection) -> Result<Vec<LocalConflict>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT conflict_id, object_id, object_type, remote_version, remote_device_id,
                    remote_nonce, remote_ciphertext, remote_deleted, local_canonical
             FROM sync_conflicts ORDER BY created_at",
        )
        .map_err(|e| format!("Failed to list conflicts: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LocalConflict {
                conflict_id: row.get(0)?,
                object_id: row.get(1)?,
                object_type: row.get(2)?,
                remote_version: row.get(3)?,
                remote_device_id: row.get(4)?,
                remote_nonce: row.get(5)?,
                remote_ciphertext: row.get(6)?,
                remote_deleted: row.get::<_, i64>(7)? != 0,
                local_canonical: row.get(8)?,
            })
        })
        .map_err(|e| format!("Failed to list conflicts: {}", e))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to list conflicts: {}", e))
}

pub fn delete_conflict(conn: &Connection, conflict_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM sync_conflicts WHERE conflict_id = ?1",
        params![conflict_id],
    )
    .map_err(|e| format!("Failed to delete conflict: {}", e))?;
    Ok(())
}

pub fn delete_conflicts_for_object(conn: &Connection, object_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM sync_conflicts WHERE object_id = ?1",
        params![object_id],
    )
    .map_err(|e| format!("Failed to delete conflicts: {}", e))?;
    Ok(())
}
