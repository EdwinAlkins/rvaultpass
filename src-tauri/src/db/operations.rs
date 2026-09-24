//! Database operations for entries, folders, tags, and search
//!
//! This module provides all CRUD operations for the vault database.

use rusqlite::{Connection, params};
use uuid::Uuid;
use chrono::Utc;
use secrecy::ExposeSecret;

use crate::models::{VaultEntry, VaultFolder, VaultTag, PasswordGenerationParams, TOTPResult};

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the active database connection from the vault state
fn get_connection() -> Result<Connection, String> {
    use crate::vault::VaultManager;
    VaultManager::get_connection()
}

/// Generate a UUID v4 as bytes
fn generate_uuid() -> Vec<u8> {
    let uuid = Uuid::new_v4();
    uuid.as_bytes().to_vec()
}

/// Convert UUID bytes to string
fn uuid_to_string(bytes: &[u8]) -> String {
    if bytes.len() == 16 {
        Uuid::from_slice(bytes)
            .map(|u| u.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

/// Parse UUID from string to bytes
fn parse_uuid(s: &str) -> Result<Vec<u8>, String> {
    Uuid::parse_str(s)
        .map(|u| u.as_bytes().to_vec())
        .map_err(|_| format!("Invalid UUID: {}", s))
}

// ============================================================================
// Entry CRUD Operations
// ============================================================================

/// Create a new entry
pub fn create_entry(
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
) -> Result<VaultEntry, String> {
    let conn = get_connection()?;
    let id = generate_uuid();
    let now = Utc::now().timestamp();

    let folder_id_bytes = folder_id.as_ref().map(|f| parse_uuid(f)).transpose()?;

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
    ).map_err(|e| format!("Failed to create entry: {}", e))?;

    // Update FTS index
    update_fts_index(&conn, &id, &title, &url, &username, &username2, &username3, &notes)?;

    // Associate tags if provided
    if let Some(tags) = tag_ids {
        for tag_id in &tags {
            let tag_bytes = parse_uuid(tag_id)?;
            conn.execute(
                "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
                params![id, tag_bytes],
            ).map_err(|e| format!("Failed to associate tag: {}", e))?;
        }
    }

    // Fetch the created entry with tags
    let id_str = uuid_to_string(&id);
    get_entry_by_id(&conn, &id_str)
}

/// Get all entries
pub fn get_all_entries() -> Result<Vec<VaultEntry>, String> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT id, folder_id, title, url, username, username2, username3, password, notes, totp_secret,
                created_at, updated_at, is_favorite
         FROM entries ORDER BY title"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt.query_map([], |row| Ok(map_entry_row(row)?))
        .map_err(|e| format!("Failed to query entries: {}", e))?;

    let mut entries: Vec<VaultEntry> = rows.collect::<Result<_, _>>()
        .map_err(|e| format!("Failed to collect entries: {}", e))?;

    // Load tags for each entry
    for entry in &mut entries {
        entry.tags = get_tags_for_entry(&conn, &entry.id)?;
    }

    Ok(entries)
}

/// Map a SELECT row (id … is_favorite) to VaultEntry
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

/// Get a single entry by ID
pub fn get_entry_by_id(conn: &Connection, id: &str) -> Result<VaultEntry, String> {
    let id_bytes = parse_uuid(id)?;

    let mut stmt = conn.prepare(
        "SELECT id, folder_id, title, url, username, username2, username3, password, notes, totp_secret,
                created_at, updated_at, is_favorite
         FROM entries WHERE id = ?1"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let entry = stmt
        .query_row(params![id_bytes], |row| Ok(map_entry_row(row)?))
        .map_err(|e| format!("Entry not found: {}", e))?;

    Ok(entry)
}

/// Get a single entry by ID (public)
pub fn get_entry(id: &str) -> Result<VaultEntry, String> {
    let conn = get_connection()?;
    let mut entry = get_entry_by_id(&conn, id)?;
    entry.tags = get_tags_for_entry(&conn, &entry.id)?;
    Ok(entry)
}

/// Update an entry
pub fn update_entry(
    entry_id: String,
    folder_id: Option<String>,
    title: String,
    url: Option<String>,
    username: Option<String>,
    username2: Option<String>,
    username3: Option<String>,
    password: Option<String>,
    notes: Option<String>,
    totp_secret: Option<String>,
    is_favorite: Option<bool>,
    tag_ids: Option<Vec<String>>,
) -> Result<VaultEntry, String> {
    let conn = get_connection()?;
    let id_bytes = parse_uuid(&entry_id)?;
    let now = Utc::now().timestamp();

    let folder_id_bytes = folder_id.as_ref().map(|f| parse_uuid(f)).transpose()?;

    conn.execute(
        "UPDATE entries SET folder_id = ?1, title = ?2, url = ?3, username = ?4, username2 = ?5, username3 = ?6,
         password = COALESCE(?7, password), notes = ?8, totp_secret = ?9, is_favorite = COALESCE(?10, is_favorite),
         updated_at = ?11 WHERE id = ?12",
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
            is_favorite.map(|f| if f { 1 } else { 0 }),
            now,
            id_bytes
        ],
    ).map_err(|e| format!("Failed to update entry: {}", e))?;

    // Update FTS index
    let title_val = title.clone();
    update_fts_index(&conn, &id_bytes, &title_val, &url, &username, &username2, &username3, &notes)?;

    // Update tags if provided
    if let Some(tags) = tag_ids {
        // Remove existing tags
        conn.execute(
            "DELETE FROM entry_tags WHERE entry_id = ?1",
            params![id_bytes],
        ).map_err(|e| format!("Failed to remove existing tags: {}", e))?;

        // Add new tags
        for tag_id in &tags {
            let tag_bytes = parse_uuid(tag_id)?;
            conn.execute(
                "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
                params![id_bytes, tag_bytes],
            ).map_err(|e| format!("Failed to associate tag: {}", e))?;
        }
    }

    get_entry_by_id(&conn, &entry_id)
}

/// Delete an entry
pub fn delete_entry(entry_id: &str) -> Result<(), String> {
    let conn = get_connection()?;
    let id_bytes = parse_uuid(entry_id)?;

    // Remove from FTS index
    conn.execute(
        "DELETE FROM entries_fts WHERE rowid = (SELECT rowid FROM entries WHERE id = ?1)",
        params![id_bytes],
    ).ok(); // Ignore errors for FTS cleanup

    conn.execute(
        "DELETE FROM entries WHERE id = ?1",
        params![id_bytes],
    ).map_err(|e| format!("Failed to delete entry: {}", e))?;

    Ok(())
}

// ============================================================================
// Folder CRUD Operations
// ============================================================================

/// Create a new folder
pub fn create_folder(name: String, parent_id: Option<String>) -> Result<VaultFolder, String> {
    let conn = get_connection()?;
    let id = generate_uuid();
    let now = Utc::now().timestamp();

    let parent_id_bytes = parent_id.as_ref().map(|p| parse_uuid(p)).transpose()?;

    conn.execute(
        "INSERT INTO folders (id, name, parent_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, name, parent_id_bytes, now, now],
    ).map_err(|e| format!("Failed to create folder: {}", e))?;

    Ok(VaultFolder {
        id: uuid_to_string(&id),
        name,
        parent_id: parent_id.as_ref().map(|p| p.clone()),
        created_at: chrono::DateTime::from_timestamp(now, 0).unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp(now, 0).unwrap_or_default(),
    })
}

/// Get all folders
pub fn get_all_folders() -> Result<Vec<VaultFolder>, String> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, parent_id, created_at, updated_at FROM folders ORDER BY name"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt.query_map([], |row| {
        let id_bytes: Vec<u8> = row.get(0)?;
        let parent_id_bytes: Option<Vec<u8>> = row.get(2)?;

        Ok(VaultFolder {
            id: uuid_to_string(&id_bytes),
            name: row.get(1)?,
            parent_id: parent_id_bytes.as_deref().map(uuid_to_string),
            created_at: chrono::DateTime::from_timestamp(row.get(3)?, 0).unwrap_or_default(),
            updated_at: chrono::DateTime::from_timestamp(row.get(4)?, 0).unwrap_or_default(),
        })
    }).map_err(|e| format!("Failed to query folders: {}", e))?;

    rows.collect::<Result<_, _>>().map_err(|e| format!("Failed to collect folders: {}", e))
}

/// Update a folder
pub fn update_folder(folder_id: &str, name: String, parent_id: Option<String>) -> Result<VaultFolder, String> {
    let conn = get_connection()?;
    let id_bytes = parse_uuid(folder_id)?;
    let now = Utc::now().timestamp();

    let parent_id_bytes = parent_id.as_ref().map(|p| parse_uuid(p)).transpose()?;

    conn.execute(
        "UPDATE folders SET name = ?1, parent_id = ?2, updated_at = ?3 WHERE id = ?4",
        params![name, parent_id_bytes, now, id_bytes],
    ).map_err(|e| format!("Failed to update folder: {}", e))?;

    Ok(VaultFolder {
        id: folder_id.to_string(),
        name,
        parent_id,
        created_at: chrono::DateTime::from_timestamp(now, 0).unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp(now, 0).unwrap_or_default(),
    })
}

/// Delete a folder (does not delete entries inside)
pub fn delete_folder(folder_id: &str) -> Result<(), String> {
    let conn = get_connection()?;
    let id_bytes = parse_uuid(folder_id)?;

    conn.execute(
        "UPDATE entries SET folder_id = NULL WHERE folder_id = ?1",
        params![id_bytes],
    ).map_err(|e| format!("Failed to unassign folder from entries: {}", e))?;

    conn.execute(
        "DELETE FROM folders WHERE id = ?1",
        params![id_bytes],
    ).map_err(|e| format!("Failed to delete folder: {}", e))?;

    Ok(())
}

// ============================================================================
// Tag CRUD Operations
// ============================================================================

/// Create a new tag
pub fn create_tag(name: String) -> Result<VaultTag, String> {
    let conn = get_connection()?;
    let id = generate_uuid();

    conn.execute(
        "INSERT INTO tags (id, name) VALUES (?1, ?2)",
        params![id, name],
    ).map_err(|e| format!("Failed to create tag: {}", e))?;

    Ok(VaultTag {
        id: uuid_to_string(&id),
        name,
    })
}

/// Get all tags
pub fn get_all_tags() -> Result<Vec<VaultTag>, String> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT id, name FROM tags ORDER BY name"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt.query_map([], |row| {
        let id_bytes: Vec<u8> = row.get(0)?;

        Ok(VaultTag {
            id: uuid_to_string(&id_bytes),
            name: row.get(1)?,
        })
    }).map_err(|e| format!("Failed to query tags: {}", e))?;

    rows.collect::<Result<_, _>>().map_err(|e| format!("Failed to collect tags: {}", e))
}

/// Delete a tag
pub fn delete_tag(tag_id: &str) -> Result<(), String> {
    let conn = get_connection()?;
    let id_bytes = parse_uuid(tag_id)?;

    conn.execute(
        "DELETE FROM tags WHERE id = ?1",
        params![id_bytes],
    ).map_err(|e| format!("Failed to delete tag: {}", e))?;

    Ok(())
}

/// Get tags for a specific entry
pub fn get_tags_for_entry(conn: &Connection, entry_id: &str) -> Result<Vec<VaultTag>, String> {
    let entry_bytes = parse_uuid(entry_id)?;

    let mut stmt = conn.prepare(
        "SELECT t.id, t.name FROM tags t
         INNER JOIN entry_tags et ON t.id = et.tag_id
         WHERE et.entry_id = ?1"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt.query_map(params![entry_bytes], |row| {
        let id_bytes: Vec<u8> = row.get(0)?;

        Ok(VaultTag {
            id: uuid_to_string(&id_bytes),
            name: row.get(1)?,
        })
    }).map_err(|e| format!("Failed to query tags: {}", e))?;

    rows.collect::<Result<_, _>>().map_err(|e| format!("Failed to collect tags: {}", e))
}

// ============================================================================
// Search Operations
// ============================================================================

/// Search entries using full-text search
pub fn search_entries(query: &str) -> Result<Vec<VaultEntry>, String> {
    let conn = get_connection()?;

    // Use FTS5 match query
    let mut stmt = conn.prepare(
        "SELECT e.id, e.folder_id, e.title, e.url, e.username, e.username2, e.username3, e.password, e.notes, e.totp_secret,
                e.created_at, e.updated_at, e.is_favorite
         FROM entries e
         INNER JOIN entries_fts fts ON e.rowid = fts.rowid
         WHERE entries_fts MATCH ?1
         ORDER BY rank"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;

    // Sanitize query for FTS5 (wrap phrases)
    let fts_query = format!("\"{}\"*", query.replace('"', ""));

    let rows = stmt
        .query_map(params![fts_query], |row| Ok(map_entry_row(row)?))
        .map_err(|e| format!("Failed to query entries: {}", e))?;

    let entries: Vec<VaultEntry> = rows.collect::<Result<_, _>>()
        .map_err(|e| format!("Failed to collect entries: {}", e))?;

    Ok(entries)
}

/// Update the FTS index for an entry
fn update_fts_index(
    conn: &Connection,
    id: &[u8],
    title: &str,
    url: &Option<String>,
    username: &Option<String>,
    username2: &Option<String>,
    username3: &Option<String>,
    notes: &Option<String>,
) -> Result<(), String> {
    let url_val = url.as_deref().unwrap_or("");
    let notes_val = notes.as_deref().unwrap_or("");

    // Concatenate login fields so username2/3 remain searchable without altering FTS schema
    let username_val = [username, username2, username3]
        .iter()
        .filter_map(|v| v.as_deref())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    // Delete existing FTS entry
    conn.execute(
        "DELETE FROM entries_fts WHERE rowid = (SELECT rowid FROM entries WHERE id = ?1)",
        params![id],
    ).ok();

    // Insert new FTS entry
    conn.execute(
        "INSERT INTO entries_fts (rowid, title, url, username, notes)
         SELECT rowid, ?2, ?3, ?4, ?5 FROM entries WHERE id = ?1",
        params![id, title, url_val, username_val, notes_val],
    ).map_err(|e| format!("Failed to update FTS index: {}", e))?;

    Ok(())
}

// ============================================================================
// Password Generator
// ============================================================================

/// Generate a random password
pub fn generate_password(params: &PasswordGenerationParams) -> Result<String, String> {
    use ring::rand::SystemRandom;

    const MIN_LEN: usize = 4;
    const MAX_LEN: usize = 1024;
    if !(MIN_LEN..=MAX_LEN).contains(&params.length) {
        return Err(format!(
            "Password length must be between {} and {}",
            MIN_LEN, MAX_LEN
        ));
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

/// Generate a passphrase using the EFF large Diceware word list (7776 words)
pub fn generate_passphrase(word_count: usize) -> Result<String, String> {
    const MIN_WORDS: usize = 3;
    const MAX_WORDS: usize = 20;
    if !(MIN_WORDS..=MAX_WORDS).contains(&word_count) {
        return Err(format!(
            "Word count must be between {} and {}",
            MIN_WORDS, MAX_WORDS
        ));
    }

    // EFF Large Wordlist — one word per line (7776 entries, ~12.9 bits/word)
    static WORDS: once_cell::sync::Lazy<Vec<&'static str>> = once_cell::sync::Lazy::new(|| {
        include_str!("../../assets/eff_large_wordlist.txt")
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
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
        // Rejection sampling to avoid modulo bias
        let idx = random_index(&rng, bound)?;
        words.push(WORDS[idx as usize]);
    }

    Ok(words.join("-"))
}

fn random_index(rng: &ring::rand::SystemRandom, bound: u32) -> Result<u32, String> {
    use ring::rand::SecureRandom;
    // Rejection sampling: discard values in the biased remainder range
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

// ============================================================================
// TOTP Generation
// ============================================================================

/// Generate a TOTP code from a secret
pub fn generate_totp(secret: &str) -> Result<TOTPResult, String> {
    use hmac::{Hmac, Mac};
    use sha1::Sha1;
    use secrecy::Secret;

    type HmacSha1 = Hmac<Sha1>;

    // Decode base32 secret (also accepts otpauth:// URIs)
    let secret_bytes = decode_base32(&extract_totp_secret(secret))?;
    let secret_key = Secret::new(secret_bytes);

    // Get current time step (30 second intervals)
    let epoch = Utc::now().timestamp() as u64;
    let time_step = epoch / 30;
    let time_remaining = 30 - (epoch % 30);

    // Convert time step to 8-byte big-endian
    let time_bytes = time_step.to_be_bytes();

    // Compute HMAC-SHA1
    let mut mac = HmacSha1::new_from_slice(secret_key.expose_secret())
        .map_err(|_| "Invalid TOTP secret")?;
    mac.update(&time_bytes);
    let result = mac.finalize();
    let hmac_result = result.into_bytes();

    // Dynamic truncation
    let offset = (hmac_result[19] & 0x0f) as usize;
    let code_int = ((hmac_result[offset] & 0x7f) as u32) << 24
        | (hmac_result[offset + 1] as u32) << 16
        | (hmac_result[offset + 2] as u32) << 8
        | (hmac_result[offset + 3] as u32);

    let code = format!("{:06}", code_int % 1_000_000);

    Ok(TOTPResult {
        code,
        time_remaining,
    })
}

/// Extract a base32 secret from a raw secret or otpauth:// URI
fn extract_totp_secret(input: &str) -> String {
    let trimmed = input.trim();
    if !trimmed.to_lowercase().starts_with("otpauth://") {
        return trimmed.to_string();
    }

    // otpauth://totp/Label?secret=BASE32&issuer=...
    if let Some(query) = trimmed.split('?').nth(1) {
        for pair in query.split('&') {
            let mut parts = pair.splitn(2, '=');
            if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                if key.eq_ignore_ascii_case("secret") {
                    return value.to_string();
                }
            }
        }
    }

    trimmed.to_string()
}

/// Decode a base32 string to bytes
fn decode_base32(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim().to_uppercase().replace(" ", "");
    let mut bits = Vec::new();

    for c in input.chars() {
        let val = match c {
            'A'..='Z' => c as u8 - b'A',
            '2'..='7' => c as u8 - b'2' + 26,
            '=' => break, // Padding
            _ => continue, // Skip invalid
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
        }
    }

    Ok(bytes)
}
