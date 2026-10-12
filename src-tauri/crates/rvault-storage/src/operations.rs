//! Database operations for entries, folders, tags, and search.

use std::collections::HashMap;

use chrono::Utc;
use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};
use rusqlite::{params, Connection};
use rvault_core::{EntryPatch, EntrySummary, PasswordGenerationParams, TOTPResult, VaultEntry, VaultFolder, VaultTag};
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use uuid::Uuid;

use crate::error::StorageError;

type HmacSha1 = Hmac<Sha1>;
type HmacSha256 = Hmac<Sha256>;
type HmacSha512 = Hmac<Sha512>;

fn generate_uuid() -> Vec<u8> {
    Uuid::new_v4().as_bytes().to_vec()
}

fn uuid_to_string(bytes: &[u8]) -> String {
    Uuid::from_slice(bytes)
        .map(|uuid| uuid.to_string())
        .unwrap_or_default()
}

fn parse_uuid(value: &str) -> Result<Vec<u8>, StorageError> {
    Uuid::parse_str(value)
        .map(|uuid| uuid.as_bytes().to_vec())
        .map_err(|_| StorageError::Invalid(format!("Invalid UUID: {value}")))
}

fn sql_error(message: impl Into<String>) -> StorageError {
    StorageError::internal(message.into())
}

fn ensure_changed(count: usize) -> Result<(), StorageError> {
    if count == 0 {
        Err(StorageError::NotFound("Record not found".to_string()))
    } else {
        Ok(())
    }
}

fn in_transaction<T>(
    conn: &Connection,
    write: impl FnOnce(&Connection) -> Result<T, StorageError>,
) -> Result<T, StorageError> {
    if conn.is_autocommit() {
        let tx = conn
            .unchecked_transaction()
            .map_err(|error| sql_error(error.to_string()))?;
        let value = write(&tx)?;
        tx.commit().map_err(|error| sql_error(error.to_string()))?;
        Ok(value)
    } else {
        write(conn)
    }
}

fn apply_patch<T: Clone>(patch: Option<Option<T>>, current: Option<T>) -> Option<T> {
    match patch {
        None => current,
        Some(value) => value,
    }
}

pub fn create_entry(
    conn: &Connection,
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
) -> Result<VaultEntry, StorageError> {
    in_transaction(conn, |conn| {
        let id = generate_uuid();
        let now = Utc::now().timestamp();
        let folder_id_bytes = folder_id.as_ref().map(|value| parse_uuid(value)).transpose()?;
        conn.execute(
            "INSERT INTO entries (id, folder_id, title, url, username, username2, username3, password, notes, totp_secret, created_at, updated_at, is_favorite)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                folder_id_bytes,
                title,
                url,
                username,
                username2,
                username3,
                password,
                notes,
                totp_secret,
                now,
                now,
                if is_favorite { 1 } else { 0 }
            ],
        )
        .map_err(|error| sql_error(format!("Failed to create entry: {error}")))?;
        if let Some(tags) = tag_ids {
            replace_tags(conn, &id, &tags)?;
        }
        get_entry_on(conn, &uuid_to_string(&id))
    })
}

pub fn update_entry(conn: &Connection, entry_id: &str, patch: EntryPatch) -> Result<VaultEntry, StorageError> {
    in_transaction(conn, |conn| {
        let existing = get_entry_on(conn, entry_id)?;
        let id_bytes = parse_uuid(entry_id)?;
        let now = Utc::now().timestamp();
        let folder_id = apply_patch(patch.folder_id, existing.folder_id);
        let folder_id_bytes = folder_id.as_ref().map(|value| parse_uuid(value)).transpose()?;
        let title = patch.title.unwrap_or(existing.title);
        let url = apply_patch(patch.url, existing.url);
        let username = apply_patch(patch.username, existing.username);
        let username2 = apply_patch(patch.username2, existing.username2);
        let username3 = apply_patch(patch.username3, existing.username3);
        let password = patch.password.unwrap_or(existing.password);
        let notes = apply_patch(patch.notes, existing.notes);
        let totp_secret = apply_patch(patch.totp_secret, existing.totp_secret);
        let is_favorite = patch.is_favorite.unwrap_or(existing.is_favorite);
        let count = conn
            .execute(
                "UPDATE entries SET folder_id = ?1, title = ?2, url = ?3, username = ?4, username2 = ?5, username3 = ?6,
                        password = ?7, notes = ?8, totp_secret = ?9, updated_at = ?10, is_favorite = ?11
                 WHERE id = ?12",
                params![
                    folder_id_bytes,
                    title,
                    url,
                    username,
                    username2,
                    username3,
                    password,
                    notes,
                    totp_secret,
                    now,
                    if is_favorite { 1 } else { 0 },
                    id_bytes
                ],
            )
            .map_err(|error| sql_error(format!("Failed to update entry: {error}")))?;
        ensure_changed(count)?;
        if let Some(tags) = patch.tag_ids {
            replace_tags(conn, &id_bytes, &tags)?;
        }
        get_entry_on(conn, entry_id)
    })
}

fn replace_tags(conn: &Connection, entry_id: &[u8], tag_ids: &[String]) -> Result<(), StorageError> {
    conn.execute("DELETE FROM entry_tags WHERE entry_id = ?1", params![entry_id])
        .map_err(|error| sql_error(format!("Failed to remove existing tags: {error}")))?;
    for tag_id in tag_ids {
        let tag_bytes = parse_uuid(tag_id)?;
        let attached = conn
            .execute(
                "INSERT INTO entry_tags (entry_id, tag_id) SELECT ?1, id FROM tags WHERE id = ?2",
                params![entry_id, tag_bytes],
            )
            .map_err(|error| sql_error(format!("Failed to associate tag: {error}")))?;
        if attached == 0 {
            return Err(StorageError::Invalid(format!("Unknown tag: {tag_id}")));
        }
    }
    Ok(())
}

pub fn delete_entry(conn: &Connection, entry_id: &str) -> Result<(), StorageError> {
    in_transaction(conn, |conn| {
        let id_bytes = parse_uuid(entry_id)?;
        let count = conn
            .execute("DELETE FROM entries WHERE id = ?1", params![id_bytes])
            .map_err(|error| sql_error(format!("Failed to delete entry: {error}")))?;
        ensure_changed(count)
    })
}

pub fn get_entry(conn: &Connection, entry_id: &str) -> Result<VaultEntry, StorageError> {
    get_entry_on(conn, entry_id)
}

fn get_entry_on(conn: &Connection, entry_id: &str) -> Result<VaultEntry, StorageError> {
    let id_bytes = parse_uuid(entry_id)?;
    let mut entry = conn
        .query_row(
            "SELECT id, folder_id, title, url, username, username2, username3, password, notes, totp_secret,
                    created_at, updated_at, is_favorite
             FROM entries WHERE id = ?1",
            params![id_bytes],
            |row| Ok(map_entry_row(row)?),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound("Entry not found".to_string()),
            other => sql_error(format!("Entry not found: {other}")),
        })?;
    entry.tags = tags_for_entries(conn, &[entry.id.clone()])?
        .remove(&entry.id)
        .unwrap_or_default();
    Ok(entry)
}

fn map_entry_row(row: &rusqlite::Row<'_>) -> Result<VaultEntry, rusqlite::Error> {
    let id_bytes: Vec<u8> = row.get(0)?;
    let folder_id_bytes: Option<Vec<u8>> = row.get(1)?;
    Ok(VaultEntry {
        id: uuid_to_string(&id_bytes),
        folder_id: folder_id_bytes.as_deref().map(uuid_to_string),
        title: row.get(2)?,
        url: row.get(3)?,
        username: row.get(4)?,
        username2: row.get(5)?,
        username3: row.get(6)?,
        password: row.get(7)?,
        notes: row.get(8)?,
        totp_secret: row.get(9)?,
        created_at: chrono::DateTime::from_timestamp(row.get(10)?, 0).unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp(row.get(11)?, 0).unwrap_or_default(),
        is_favorite: row.get::<_, i32>(12)? != 0,
        tags: Vec::new(),
    })
}

pub fn list_entries(conn: &Connection) -> Result<Vec<EntrySummary>, StorageError> {
    load_summaries(conn, None)
}

pub fn search_entries(conn: &Connection, query: &str) -> Result<Vec<EntrySummary>, StorageError> {
    let fts_query = format!("\"{}\"*", query.replace('"', ""));
    load_summaries(conn, Some(&fts_query))
}

fn load_summaries(conn: &Connection, fts_query: Option<&str>) -> Result<Vec<EntrySummary>, StorageError> {
    let sql = if fts_query.is_some() {
        "SELECT e.id, e.folder_id, e.title, e.url, e.username, e.username2, e.username3,
                e.created_at, e.updated_at, e.is_favorite,
                CASE WHEN length(e.password) > 0 THEN 1 ELSE 0 END,
                CASE WHEN e.totp_secret IS NOT NULL AND length(e.totp_secret) > 0 THEN 1 ELSE 0 END,
                CASE WHEN e.notes IS NOT NULL AND length(e.notes) > 0 THEN 1 ELSE 0 END,
                t.id, t.name
         FROM entries e
         INNER JOIN entries_fts fts ON e.rowid = fts.rowid
         LEFT JOIN entry_tags et ON et.entry_id = e.id
         LEFT JOIN tags t ON t.id = et.tag_id
         WHERE entries_fts MATCH ?1
         ORDER BY rank, e.id"
    } else {
        "SELECT e.id, e.folder_id, e.title, e.url, e.username, e.username2, e.username3,
                e.created_at, e.updated_at, e.is_favorite,
                CASE WHEN length(e.password) > 0 THEN 1 ELSE 0 END,
                CASE WHEN e.totp_secret IS NOT NULL AND length(e.totp_secret) > 0 THEN 1 ELSE 0 END,
                CASE WHEN e.notes IS NOT NULL AND length(e.notes) > 0 THEN 1 ELSE 0 END,
                t.id, t.name
         FROM entries e
         LEFT JOIN entry_tags et ON et.entry_id = e.id
         LEFT JOIN tags t ON t.id = et.tag_id
         ORDER BY e.title, e.id"
    };
    let mut stmt = conn
        .prepare(sql)
        .map_err(|error| sql_error(format!("Failed to prepare entry list: {error}")))?;
    let mut rows = Vec::new();
    let mut collect = |row: &rusqlite::Row<'_>| -> rusqlite::Result<()> {
        rows.push(summary_row(row)?);
        Ok(())
    };
    if let Some(query) = fts_query {
        let mapped = stmt.query_map(params![query], |row| {
            collect(row)?;
            Ok(())
        });
        mapped
            .map_err(|error| sql_error(format!("Failed to query entries: {error}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| sql_error(format!("Failed to collect entries: {error}")))?;
    } else {
        let mapped = stmt.query_map([], |row| {
            collect(row)?;
            Ok(())
        });
        mapped
            .map_err(|error| sql_error(format!("Failed to query entries: {error}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| sql_error(format!("Failed to collect entries: {error}")))?;
    }
    Ok(group_summaries(rows))
}

struct SummaryRow {
    summary: EntrySummary,
    tag_id: Option<Vec<u8>>,
    tag_name: Option<String>,
}

fn summary_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SummaryRow> {
    let id_bytes: Vec<u8> = row.get(0)?;
    let folder_id_bytes: Option<Vec<u8>> = row.get(1)?;
    Ok(SummaryRow {
        summary: EntrySummary {
            id: uuid_to_string(&id_bytes),
            folder_id: folder_id_bytes.as_deref().map(uuid_to_string),
            title: row.get(2)?,
            url: row.get(3)?,
            username: row.get(4)?,
            username2: row.get(5)?,
            username3: row.get(6)?,
            created_at: chrono::DateTime::from_timestamp(row.get(7)?, 0).unwrap_or_default(),
            updated_at: chrono::DateTime::from_timestamp(row.get(8)?, 0).unwrap_or_default(),
            is_favorite: row.get::<_, i32>(9)? != 0,
            tags: Vec::new(),
            has_password: row.get::<_, i32>(10)? != 0,
            has_totp: row.get::<_, i32>(11)? != 0,
            has_notes: row.get::<_, i32>(12)? != 0,
        },
        tag_id: row.get(13)?,
        tag_name: row.get(14)?,
    })
}

fn group_summaries(rows: Vec<SummaryRow>) -> Vec<EntrySummary> {
    let mut order = Vec::new();
    let mut grouped: HashMap<String, EntrySummary> = HashMap::new();
    for row in rows {
        let entry = grouped.entry(row.summary.id.clone()).or_insert_with(|| {
            order.push(row.summary.id.clone());
            row.summary.clone()
        });
        if let (Some(id), Some(name)) = (row.tag_id, row.tag_name) {
            entry.tags.push(VaultTag {
                id: uuid_to_string(&id),
                name,
            });
        }
    }
    order.into_iter().filter_map(|id| grouped.remove(&id)).collect()
}

fn tags_for_entries(conn: &Connection, ids: &[String]) -> Result<HashMap<String, Vec<VaultTag>>, StorageError> {
    let mut tags = HashMap::new();
    for id in ids {
        let entry_bytes = parse_uuid(id)?;
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.name FROM tags t
                 INNER JOIN entry_tags et ON t.id = et.tag_id
                 WHERE et.entry_id = ?1",
            )
            .map_err(|error| sql_error(format!("Failed to prepare tags: {error}")))?;
        let rows = stmt
            .query_map(params![entry_bytes], |row| {
                let id_bytes: Vec<u8> = row.get(0)?;
                Ok(VaultTag {
                    id: uuid_to_string(&id_bytes),
                    name: row.get(1)?,
                })
            })
            .map_err(|error| sql_error(format!("Failed to query tags: {error}")))?;
        let collected = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| sql_error(format!("Failed to collect tags: {error}")))?;
        tags.insert(id.clone(), collected);
    }
    Ok(tags)
}

pub fn create_folder(conn: &Connection, name: String, parent_id: Option<String>) -> Result<VaultFolder, StorageError> {
    in_transaction(conn, |conn| {
        if let Some(parent) = &parent_id {
            ensure_folder(conn, parent)?;
        }
        let id = generate_uuid();
        let now = Utc::now().timestamp();
        let parent_bytes = parent_id.as_ref().map(|value| parse_uuid(value)).transpose()?;
        conn.execute(
            "INSERT INTO folders (id, name, parent_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, parent_bytes, now, now],
        )
        .map_err(|error| sql_error(format!("Failed to create folder: {error}")))?;
        Ok(VaultFolder {
            id: uuid_to_string(&id),
            name,
            parent_id,
            created_at: chrono::DateTime::from_timestamp(now, 0).unwrap_or_default(),
            updated_at: chrono::DateTime::from_timestamp(now, 0).unwrap_or_default(),
        })
    })
}

pub fn list_folders(conn: &Connection) -> Result<Vec<VaultFolder>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT id, name, parent_id, created_at, updated_at FROM folders ORDER BY name")
        .map_err(|error| sql_error(format!("Failed to prepare folders: {error}")))?;
    let rows = stmt
        .query_map([], |row| {
            let id_bytes: Vec<u8> = row.get(0)?;
            let parent_id_bytes: Option<Vec<u8>> = row.get(2)?;
            Ok(VaultFolder {
                id: uuid_to_string(&id_bytes),
                name: row.get(1)?,
                parent_id: parent_id_bytes.as_deref().map(uuid_to_string),
                created_at: chrono::DateTime::from_timestamp(row.get(3)?, 0).unwrap_or_default(),
                updated_at: chrono::DateTime::from_timestamp(row.get(4)?, 0).unwrap_or_default(),
            })
        })
        .map_err(|error| sql_error(format!("Failed to query folders: {error}")))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| sql_error(format!("Failed to collect folders: {error}")))
}

pub fn update_folder(
    conn: &Connection,
    folder_id: &str,
    name: String,
    parent_id: Option<String>,
) -> Result<VaultFolder, StorageError> {
    in_transaction(conn, |conn| {
        ensure_folder(conn, folder_id)?;
        if parent_creates_cycle(conn, folder_id, parent_id.as_deref())? {
            return Err(StorageError::Conflict(
                "Folder parent would create a cycle".to_string(),
            ));
        }
        let id_bytes = parse_uuid(folder_id)?;
        let parent_bytes = parent_id.as_ref().map(|value| parse_uuid(value)).transpose()?;
        let now = Utc::now().timestamp();
        let count = conn
            .execute(
                "UPDATE folders SET name = ?1, parent_id = ?2, updated_at = ?3 WHERE id = ?4",
                params![name, parent_bytes, now, id_bytes],
            )
            .map_err(|error| sql_error(format!("Failed to update folder: {error}")))?;
        ensure_changed(count)?;
        Ok(VaultFolder {
            id: folder_id.to_string(),
            name,
            parent_id,
            created_at: chrono::DateTime::from_timestamp(now, 0).unwrap_or_default(),
            updated_at: chrono::DateTime::from_timestamp(now, 0).unwrap_or_default(),
        })
    })
}

pub fn delete_folder(conn: &Connection, folder_id: &str) -> Result<(), StorageError> {
    in_transaction(conn, |conn| {
        let id_bytes = parse_uuid(folder_id)?;
        ensure_folder(conn, folder_id)?;
        let children: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM folders WHERE parent_id = ?1",
                params![id_bytes],
                |row| row.get(0),
            )
            .map_err(|error| sql_error(error.to_string()))?;
        if children > 0 {
            return Err(StorageError::Conflict(
                "Folder still has subfolders".to_string(),
            ));
        }
        conn.execute(
            "UPDATE entries SET folder_id = NULL WHERE folder_id = ?1",
            params![id_bytes],
        )
        .map_err(|error| sql_error(format!("Failed to unassign folder from entries: {error}")))?;
        let count = conn
            .execute("DELETE FROM folders WHERE id = ?1", params![id_bytes])
            .map_err(|error| sql_error(format!("Failed to delete folder: {error}")))?;
        ensure_changed(count)
    })
}

fn ensure_folder(conn: &Connection, folder_id: &str) -> Result<(), StorageError> {
    let id_bytes = parse_uuid(folder_id)?;
    let found: Result<i64, _> = conn.query_row(
        "SELECT 1 FROM folders WHERE id = ?1",
        params![id_bytes],
        |row| row.get(0),
    );
    match found {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(StorageError::NotFound("Folder not found".to_string()))
        }
        Err(error) => Err(sql_error(error.to_string())),
    }
}

fn parent_creates_cycle(conn: &Connection, folder_id: &str, parent_id: Option<&str>) -> Result<bool, StorageError> {
    let mut current = parent_id.map(str::to_string);
    while let Some(id) = current {
        if id == folder_id {
            return Ok(true);
        }
        let bytes = parse_uuid(&id)?;
        current = conn
            .query_row(
                "SELECT parent_id FROM folders WHERE id = ?1",
                params![bytes],
                |row| row.get::<_, Option<Vec<u8>>>(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound("Folder not found".to_string()),
                other => sql_error(other.to_string()),
            })?
            .map(|value| uuid_to_string(&value));
    }
    Ok(false)
}

pub fn create_tag(conn: &Connection, name: String) -> Result<VaultTag, StorageError> {
    in_transaction(conn, |conn| {
        let id = generate_uuid();
        conn.execute("INSERT INTO tags (id, name) VALUES (?1, ?2)", params![id, name])
            .map_err(|error| sql_error(format!("Failed to create tag: {error}")))?;
        Ok(VaultTag {
            id: uuid_to_string(&id),
            name,
        })
    })
}

pub fn list_tags(conn: &Connection) -> Result<Vec<VaultTag>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT id, name FROM tags ORDER BY name")
        .map_err(|error| sql_error(format!("Failed to prepare tags: {error}")))?;
    let rows = stmt
        .query_map([], |row| {
            let id_bytes: Vec<u8> = row.get(0)?;
            Ok(VaultTag {
                id: uuid_to_string(&id_bytes),
                name: row.get(1)?,
            })
        })
        .map_err(|error| sql_error(format!("Failed to query tags: {error}")))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| sql_error(format!("Failed to collect tags: {error}")))
}

pub fn delete_tag(conn: &Connection, tag_id: &str) -> Result<(), StorageError> {
    in_transaction(conn, |conn| {
        let id_bytes = parse_uuid(tag_id)?;
        let count = conn
            .execute("DELETE FROM tags WHERE id = ?1", params![id_bytes])
            .map_err(|error| sql_error(format!("Failed to delete tag: {error}")))?;
        ensure_changed(count)
    })
}

pub fn generate_password(params: &PasswordGenerationParams) -> Result<String, String> {
    use ring::rand::SystemRandom;

    const MIN_LEN: usize = 4;
    const MAX_LEN: usize = 1024;
    if !(MIN_LEN..=MAX_LEN).contains(&params.length) {
        return Err(format!("Password length must be between {MIN_LEN} and {MAX_LEN}"));
    }
    let rng = SystemRandom::new();
    let mut charset = String::new();
    if params.use_uppercase {
        charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    }
    if params.use_lowercase {
        charset.push_str("abcdefghijklmnopqrstuvwxyz");
    }
    if params.use_numbers {
        charset.push_str("0123456789");
    }
    if params.use_symbols {
        charset.push_str("!@#$%^&*()-_=+[]{}|;:,.<>?");
    }
    if charset.is_empty() {
        charset.push_str("abcdefghijklmnopqrstuvwxyz0123456789");
    }
    if params.exclude_ambiguous {
        for c in ['l', '1', 'I', 'O', '0', 'o'] {
            charset = charset.replace(c, "");
        }
    }
    let charset: Vec<char> = charset.chars().collect();
    if charset.is_empty() {
        return Err("Character set is empty after applying options".to_string());
    }
    let bound = charset.len() as u32;
    let mut password = String::with_capacity(params.length);
    for _ in 0..params.length {
        let idx = random_index(&rng, bound)?;
        password.push(charset[idx as usize]);
    }
    Ok(password)
}

pub fn generate_passphrase(word_count: usize) -> Result<String, String> {
    const MIN_WORDS: usize = 3;
    const MAX_WORDS: usize = 20;
    if !(MIN_WORDS..=MAX_WORDS).contains(&word_count) {
        return Err(format!("Word count must be between {MIN_WORDS} and {MAX_WORDS}"));
    }
    static WORDS: once_cell::sync::Lazy<Vec<&'static str>> = once_cell::sync::Lazy::new(|| {
        include_str!("../../../assets/eff_large_wordlist.txt")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect()
    });
    if WORDS.len() < 7776 {
        return Err("Diceware word list is incomplete".to_string());
    }
    use ring::rand::SystemRandom;
    let rng = SystemRandom::new();
    let mut words = Vec::with_capacity(word_count);
    let bound = WORDS.len() as u32;
    for _ in 0..word_count {
        let idx = random_index(&rng, bound)?;
        words.push(WORDS[idx as usize]);
    }
    Ok(words.join("-"))
}

fn random_index(rng: &ring::rand::SystemRandom, bound: u32) -> Result<u32, String> {
    use ring::rand::SecureRandom;
    let threshold = (u32::MAX / bound) * bound;
    loop {
        let mut buf = [0u8; 4];
        rng.fill(&mut buf)
            .map_err(|_| "Failed to generate random bytes".to_string())?;
        let val = u32::from_le_bytes(buf);
        if val < threshold {
            return Ok(val % bound);
        }
    }
}

#[derive(Clone, Copy)]
enum TotpAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

struct TotpSpec {
    secret: Vec<u8>,
    digits: u32,
    period: u64,
    algorithm: TotpAlgorithm,
}

pub fn generate_totp(secret: &str) -> Result<TOTPResult, String> {
    generate_totp_at(secret, Utc::now().timestamp().max(0) as u64)
}

pub fn generate_totp_at(secret: &str, epoch: u64) -> Result<TOTPResult, String> {
    let spec = parse_totp(secret)?;
    let counter = epoch / spec.period;
    let code_int = hotp(&spec.secret, counter, spec.digits, spec.algorithm)?;
    let width = spec.digits as usize;
    Ok(TOTPResult {
        code: format!("{code_int:0width$}"),
        time_remaining: spec.period - (epoch % spec.period),
    })
}

fn parse_totp(input: &str) -> Result<TotpSpec, String> {
    let trimmed = input.trim();
    if !trimmed.to_ascii_lowercase().starts_with("otpauth://") {
        return Ok(TotpSpec {
            secret: decode_base32(trimmed)?,
            digits: 6,
            period: 30,
            algorithm: TotpAlgorithm::Sha1,
        });
    }
    let query = trimmed.split('?').nth(1).unwrap_or("");
    let mut secret = None;
    let mut digits = 6u32;
    let mut period = 30u64;
    let mut algorithm = TotpAlgorithm::Sha1;
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        match key.to_ascii_lowercase().as_str() {
            "secret" => secret = Some(value.to_string()),
            "digits" => {
                digits = value.parse().map_err(|_| "Invalid TOTP digits".to_string())?;
            }
            "period" => {
                period = value.parse().map_err(|_| "Invalid TOTP period".to_string())?;
            }
            "algorithm" => {
                algorithm = match value.to_ascii_uppercase().as_str() {
                    "SHA1" => TotpAlgorithm::Sha1,
                    "SHA256" => TotpAlgorithm::Sha256,
                    "SHA512" => TotpAlgorithm::Sha512,
                    _ => return Err("Unsupported TOTP algorithm".to_string()),
                };
            }
            _ => {}
        }
    }
    if !(6..=8).contains(&digits) {
        return Err("TOTP digits must be between 6 and 8".to_string());
    }
    if period == 0 {
        return Err("TOTP period must be positive".to_string());
    }
    let secret = secret.ok_or_else(|| "TOTP URI has no secret".to_string())?;
    Ok(TotpSpec {
        secret: decode_base32(&secret)?,
        digits,
        period,
        algorithm,
    })
}

fn hotp(key: &[u8], counter: u64, digits: u32, algorithm: TotpAlgorithm) -> Result<u32, String> {
    let time_bytes = counter.to_be_bytes();
    let hmac_result = match algorithm {
        TotpAlgorithm::Sha1 => mac_bytes::<HmacSha1>(key, &time_bytes)?,
        TotpAlgorithm::Sha256 => mac_bytes::<HmacSha256>(key, &time_bytes)?,
        TotpAlgorithm::Sha512 => mac_bytes::<HmacSha512>(key, &time_bytes)?,
    };
    let offset = (hmac_result[hmac_result.len() - 1] & 0x0f) as usize;
    let code_int = ((hmac_result[offset] & 0x7f) as u32) << 24
        | (hmac_result[offset + 1] as u32) << 16
        | (hmac_result[offset + 2] as u32) << 8
        | (hmac_result[offset + 3] as u32);
    Ok(code_int % 10u32.pow(digits))
}

fn mac_bytes<M: Mac + KeyInit>(key: &[u8], message: &[u8]) -> Result<Vec<u8>, String> {
    let mut mac = <M as KeyInit>::new_from_slice(key).map_err(|_| "Invalid TOTP secret".to_string())?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn decode_base32(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim().to_uppercase().replace(' ', "");
    if input.is_empty() {
        return Err("TOTP secret is empty".to_string());
    }
    let mut bits = Vec::new();
    for c in input.chars() {
        let val = match c {
            'A'..='Z' => c as u8 - b'A',
            '2'..='7' => c as u8 - b'2' + 26,
            '=' => break,
            _ => continue,
        };
        bits.push(val);
    }
    let mut bytes = Vec::new();
    let mut buffer = 0u32;
    let mut bits_in_buffer = 0;
    for val in bits {
        buffer = (buffer << 5) | (val as u32);
        bits_in_buffer += 5;
        while bits_in_buffer >= 8 {
            bits_in_buffer -= 8;
            bytes.push((buffer >> bits_in_buffer) as u8);
            buffer &= (1 << bits_in_buffer) - 1;
        }
    }
    if bytes.is_empty() {
        return Err("TOTP secret is empty".to_string());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_new_vault, init_connection};
    use rvault_core::EntryPatch;
    use secrecy::Secret;
    use std::sync::atomic::{AtomicU64, Ordering};

    static DB_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_conn() -> (std::path::PathBuf, Connection) {
        let id = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("rvault-ops-{}-{id}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let key = Secret::new(vec![9u8; 32]);
        create_new_vault(&path, &key).unwrap();
        let conn = init_connection(&path, &key).unwrap();
        (path, conn)
    }

    #[test]
    fn clearing_tags_removes_every_association() {
        let (path, conn) = temp_conn();
        let tag = create_tag(&conn, "work".into()).unwrap();
        let entry = create_entry(
            &conn, None, "Mail".into(), None, None, None, None, "secret".into(), None, None, false,
            Some(vec![tag.id.clone()]),
        )
        .unwrap();
        assert_eq!(entry.tags.len(), 1);
        let cleared = update_entry(
            &conn,
            &entry.id,
            EntryPatch { tag_ids: Some(Vec::new()), ..EntryPatch::default() },
        )
        .unwrap();
        assert!(cleared.tags.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn null_patch_clears_url_notes_and_totp() {
        let (path, conn) = temp_conn();
        let entry = create_entry(
            &conn, None, "Mail".into(), Some("https://example.com".into()), None, None, None,
            "secret".into(), Some("note".into()), Some("JBSWY3DPEHPK3PXP".into()), false, None,
        )
        .unwrap();
        let updated = update_entry(
            &conn,
            &entry.id,
            EntryPatch {
                url: Some(None),
                notes: Some(None),
                totp_secret: Some(None),
                ..EntryPatch::default()
            },
        )
        .unwrap();
        assert!(updated.url.is_none());
        assert!(updated.notes.is_none());
        assert!(updated.totp_secret.is_none());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn renamed_title_is_absent_from_the_old_fts_term() {
        let (path, conn) = temp_conn();
        let entry = create_entry(
            &conn, None, "Alpha".into(), None, None, None, None, "secret".into(), None, None, false, None,
        )
        .unwrap();
        update_entry(
            &conn,
            &entry.id,
            EntryPatch { title: Some("Bravo".into()), ..EntryPatch::default() },
        )
        .unwrap();
        assert!(search_entries(&conn, "Alpha").unwrap().is_empty());
        assert_eq!(search_entries(&conn, "Bravo").unwrap().len(), 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn failed_tag_association_rolls_back_the_entry() {
        let (path, conn) = temp_conn();
        let err = create_entry(
            &conn, None, "Mail".into(), None, None, None, None, "secret".into(), None, None, false,
            Some(vec!["not-a-uuid".into()]),
        );
        assert!(err.is_err());
        assert!(list_entries(&conn).unwrap().is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn deleting_a_parent_folder_is_refused() {
        let (path, conn) = temp_conn();
        let parent = create_folder(&conn, "Parent".into(), None).unwrap();
        create_folder(&conn, "Child".into(), Some(parent.id.clone())).unwrap();
        let err = delete_folder(&conn, &parent.id).unwrap_err();
        assert!(matches!(err, StorageError::Conflict(_)));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn totp_sha256_eight_digits_matches_the_reference_vector() {
        let mut result = generate_totp_at(
            "otpauth://totp/Test?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZA&algorithm=SHA256&digits=8&period=30",
            59,
        )
        .unwrap();
        assert_eq!(result.code, "46119246");
        assert_eq!(result.time_remaining, 1);
        result = generate_totp_at(
            "otpauth://totp/Test?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZA&algorithm=SHA256&digits=8&period=60",
            59,
        )
        .unwrap();
        assert_eq!(result.code, "18920136");
        assert_eq!(result.time_remaining, 1);
    }
}
