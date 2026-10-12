//! Open, lock, and save a `.vault` file. Callers keep the session; this module
//! does not store a process-global vault.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use rvault_crypto::{
    compute_header_hmac, constant_time_eq, derive_key, generate_data_key, generate_salt,
    split_master_key, unwrap_data_key, validate_kdf_params, validate_master_password,
    wrap_data_key, KeyEnvelope, SplitKeys, VaultHeader, DEFAULT_MEM_COST, DEFAULT_PARALLELISM,
    DEFAULT_TIME_COST, HEADER_SIZE, VAULT_MAGIC, VAULT_VERSION, VAULT_VERSION_V1,
};
use rusqlite::params;
use secrecy::{ExposeSecret, Secret};

use crate::db;

static OPEN_VAULTS: std::sync::LazyLock<Mutex<HashSet<PathBuf>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));
static FAIL_BEFORE_REPLACE: AtomicBool = AtomicBool::new(false);

/// Working copy of an unlocked vault.
///
/// Dropping a clean session deletes its temporary database. Dropping a dirty
/// session renames that database beside the vault so it can be recovered.
pub struct VaultSession {
    vault_path: PathBuf,
    db_path: PathBuf,
    encryption_key: Secret<Vec<u8>>,
    kek: Secret<Vec<u8>>,
    data_key: Secret<Vec<u8>>,
    header: VaultHeader,
    dirty: bool,
    #[allow(dead_code)]
    lock: VaultLock,
}

struct VaultLock {
    file: File,
    path: PathBuf,
}

impl Drop for VaultLock {
    fn drop(&mut self) {
        if let Ok(mut open) = OPEN_VAULTS.lock() {
            open.remove(&self.path);
        }
        unlock_exclusive(&self.file);
    }
}

impl Drop for VaultSession {
    fn drop(&mut self) {
        if self.dirty {
            let recovery = recovery_path(&self.vault_path);
            if fs::rename(&self.db_path, &recovery).is_err() {
                let _ = fs::remove_file(&self.db_path);
            }
        } else {
            let _ = fs::remove_file(&self.db_path);
        }
    }
}

impl VaultSession {
    /// Create a new vault file and leave it open.
    pub fn create(vault_path: impl Into<PathBuf>, master_password: &str) -> Result<Self, String> {
        validate_master_password(master_password)?;
        let vault_path = vault_path.into();
        let lock = VaultLock::acquire(&vault_path)?;
        if vault_path.exists() {
            return Err("Vault file already exists".to_string());
        }

        let salt = generate_salt()?;
        let password = Secret::new(master_password.to_string());
        let master = derive_key(
            &password,
            &salt,
            DEFAULT_MEM_COST,
            DEFAULT_TIME_COST,
            DEFAULT_PARALLELISM,
        )
        .map_err(unlock_error)?;
        let keys = split_master_key(&master).map_err(unlock_error)?;
        let mut header = VaultHeader {
            magic: VAULT_MAGIC,
            version: VAULT_VERSION,
            salt,
            mem_cost: DEFAULT_MEM_COST,
            time_cost: DEFAULT_TIME_COST,
            parallelism: DEFAULT_PARALLELISM,
            header_hmac: [0u8; 32],
        };
        header.header_hmac = header_mac(&header, &keys.kek);

        let db_path = working_db_path(&vault_path);
        db::create_new_vault(&db_path, &keys.sqlcipher)?;
        let data_key = {
            let conn = db::init_connection(&db_path, &keys.sqlcipher)?;
            ensure_data_key(&conn, keys.kek.expose_secret(), &header)?.0
        };

        let mut session = Self {
            vault_path,
            db_path,
            encryption_key: keys.sqlcipher,
            kek: keys.kek,
            data_key,
            header,
            dirty: true,
            lock,
        };
        session.save()?;
        Ok(session)
    }

    /// Open an existing vault. Schema upgrades and a missing data key are saved immediately.
    pub fn open(vault_path: impl Into<PathBuf>, master_password: &str) -> Result<Self, String> {
        let vault_path = vault_path.into();
        if !vault_path.exists() {
            return Err("Vault file not found".to_string());
        }
        let lock = VaultLock::acquire(&vault_path)?;
        let vault_data = fs::read(&vault_path).map_err(|e| format!("Failed to read vault file: {e}"))?;
        if vault_data.len() < HEADER_SIZE {
            return Err("Vault file too small".to_string());
        }
        let header_array: [u8; HEADER_SIZE] = vault_data[0..HEADER_SIZE]
            .try_into()
            .map_err(|_| "Failed to parse vault header".to_string())?;
        let header = VaultHeader::from_bytes(&header_array)?;
        header.verify()?;
        validate_kdf_params(header.mem_cost, header.time_cost, header.parallelism)?;

        let keys = keys_for_header(master_password, &header)?;
        let computed_hmac = compute_header_hmac(&vault_data[0..0x1D], &keys.kek);
        if !constant_time_eq(&computed_hmac, &header.header_hmac) {
            return Err("Unable to unlock vault".to_string());
        }

        let db_data = &vault_data[HEADER_SIZE..];
        if db_data.is_empty() {
            return Err("Vault file has no database payload".to_string());
        }
        let db_path = working_db_path(&vault_path);
        fs::write(&db_path, db_data).map_err(|e| format!("Failed to extract database: {e}"))?;

        db::test_vault_connection(&db_path, &keys.sqlcipher)?;
        let (schema_changed, data_key, key_created) = {
            let conn = db::init_connection(&db_path, &keys.sqlcipher)?;
            let schema_changed = db::run_migrations(&conn)?;
            let (data_key, key_created) = ensure_data_key(&conn, keys.kek.expose_secret(), &header)?;
            (schema_changed, data_key, key_created)
        };

        let mut session = Self {
            vault_path,
            db_path,
            encryption_key: keys.sqlcipher,
            kek: keys.kek,
            data_key,
            header,
            dirty: schema_changed || key_created,
            lock,
        };
        if session.dirty {
            session.save()?;
        }
        Ok(session)
    }

    /// True when `master_password` derives the key already holding this session open.
    pub fn accepts_password(&self, master_password: &str) -> Result<bool, String> {
        let keys = keys_for_header(master_password, &self.header)?;
        Ok(constant_time_eq(
            keys.sqlcipher.expose_secret(),
            self.encryption_key.expose_secret(),
        ) && constant_time_eq(keys.kek.expose_secret(), self.kek.expose_secret()))
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Replace the vault file with a consistent snapshot of the working database.
    pub fn save(&mut self) -> Result<(), String> {
        let snapshot_path = self.db_path.with_extension("snapshot");
        let conn = self.connection()?;
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        write_snapshot(&conn, &snapshot_path, &self.encryption_key)?;
        drop(conn);
        verify_snapshot(&snapshot_path, &self.encryption_key)?;

        let db_content = fs::read(&snapshot_path)
            .map_err(|e| format!("Failed to read snapshot: {e}"))?;
        let _ = fs::remove_file(&snapshot_path);

        let mut vault_content = self.header.to_bytes().to_vec();
        vault_content.extend_from_slice(&db_content);

        let tmp_path = self.vault_path.with_extension("vault.tmp");
        let pending = self.vault_path.with_extension("vault.bak.pending");
        write_synced(&tmp_path, &vault_content)?;
        if self.vault_path.exists() {
            fs::copy(&self.vault_path, &pending)
                .map_err(|e| format!("Failed to preserve the current vault: {e}"))?;
            sync_file(&pending)?;
        }

        if FAIL_BEFORE_REPLACE.swap(false, Ordering::SeqCst) {
            return Err("Simulated interruption before replace".to_string());
        }

        fs::rename(&tmp_path, &self.vault_path)
            .map_err(|e| format!("Failed to complete save: {e}"))?;
        if let Some(parent) = self.vault_path.parent() {
            sync_dir(parent)?;
        }
        rotate_backups(&self.vault_path, &pending)?;
        self.dirty = false;
        Ok(())
    }

    pub fn connection(&self) -> Result<rusqlite::Connection, String> {
        db::init_connection(&self.db_path, &self.encryption_key)
    }

    pub fn data_key(&self) -> Secret<Vec<u8>> {
        Secret::new(self.data_key.expose_secret().clone())
    }
}

impl VaultLock {
    fn acquire(vault_path: &Path) -> Result<Self, String> {
        let path = vault_identity(vault_path);
        let mut open = OPEN_VAULTS
            .lock()
            .map_err(|_| "Vault lock is unavailable".to_string())?;
        if open.contains(&path) {
            return Err("Vault is already open".to_string());
        }
        if let Some(parent) = vault_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create vault directory: {e}"))?;
            }
        }
        let lock_path = vault_path.with_extension("vault.lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| format!("Failed to open vault lock: {e}"))?;
        try_lock_exclusive(&file)?;
        open.insert(path.clone());
        cleanup_stale_temps(vault_path);
        Ok(Self { file, path })
    }
}

fn keys_for_header(master_password: &str, header: &VaultHeader) -> Result<SplitKeys, String> {
    let password = Secret::new(master_password.to_string());
    let master = derive_key(
        &password,
        &header.salt,
        header.mem_cost,
        header.time_cost,
        header.parallelism,
    )
    .map_err(unlock_error)?;
    if header.version == VAULT_VERSION_V1 {
        let kek = Secret::new(master.expose_secret().clone());
        return Ok(SplitKeys {
            sqlcipher: master,
            kek,
        });
    }
    split_master_key(&master).map_err(unlock_error)
}

fn header_mac(header: &VaultHeader, kek: &Secret<Vec<u8>>) -> [u8; 32] {
    let bytes = header.to_bytes();
    compute_header_hmac(&bytes[0..0x1D], kek)
}

fn unlock_error(detail: String) -> String {
    tracing::info!("Vault unlock failed: {detail}");
    "Unable to unlock vault".to_string()
}

fn working_db_path(vault_path: &Path) -> PathBuf {
    let name = vault_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("vault");
    let mut nonce = [0u8; 4];
    if ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut nonce).is_err() {
        nonce = (std::process::id()).to_le_bytes();
    }
    let hex: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
    let file_name = format!("{name}.{}.{hex}.tmp.db", std::process::id());
    match vault_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(file_name),
        _ => PathBuf::from(file_name),
    }
}

fn recovery_path(vault_path: &Path) -> PathBuf {
    let primary = vault_path.with_extension("vault.recovery.db");
    if !primary.exists() {
        return primary;
    }
    vault_path.with_extension(format!("vault.recovery.{}.db", std::process::id()))
}

fn cleanup_stale_temps(vault_path: &Path) {
    let Some(parent) = vault_path.parent() else {
        return;
    };
    let Some(name) = vault_path.file_name().and_then(|value| value.to_str()) else {
        return;
    };
    let prefix = format!("{name}.");
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(text) = file_name.to_str() else {
            continue;
        };
        if text.starts_with(&prefix) && text.ends_with(".tmp.db") {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn write_snapshot(
    source: &rusqlite::Connection,
    snapshot_path: &Path,
    key: &Secret<Vec<u8>>,
) -> Result<(), String> {
    if snapshot_path.exists() {
        fs::remove_file(snapshot_path).map_err(|e| format!("Failed to replace snapshot: {e}"))?;
    }
    let mut destination = rusqlite::Connection::open(snapshot_path)
        .map_err(|e| format!("Failed to create snapshot: {e}"))?;
    let key_hex = bytes_to_hex(key.expose_secret());
    destination
        .pragma_update(None, "key", format!("x'{key_hex}'"))
        .map_err(|e| format!("Failed to key snapshot: {e}"))?;
    let _ = destination.pragma_update(None, "journal_mode", "DELETE");
    {
        let backup = rusqlite::backup::Backup::new(source, &mut destination)
            .map_err(|e| format!("Failed to start snapshot: {e}"))?;
        loop {
            match backup.step(-1) {
                Ok(rusqlite::backup::StepResult::Done) => break,
                Ok(_) => std::thread::sleep(std::time::Duration::from_millis(20)),
                Err(error) => return Err(format!("Failed to snapshot database: {error}")),
            }
        }
    }
    destination
        .close()
        .map_err(|(_, error)| format!("Failed to close snapshot: {error}"))?;
    Ok(())
}

fn verify_snapshot(snapshot_path: &Path, key: &Secret<Vec<u8>>) -> Result<(), String> {
    let conn = db::init_connection(snapshot_path, key)?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Snapshot verification failed: {e}"))?;
    Ok(())
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|e| format!("Failed to prepare save: {e}"))?;
    file.write_all(bytes)
        .map_err(|e| format!("Failed to prepare save: {e}"))?;
    file.sync_all()
        .map_err(|e| format!("Failed to flush save: {e}"))?;
    Ok(())
}

fn sync_file(path: &Path) -> Result<(), String> {
    let file = File::open(path).map_err(|e| format!("Failed to flush backup: {e}"))?;
    file.sync_all()
        .map_err(|e| format!("Failed to flush backup: {e}"))?;
    Ok(())
}

fn sync_dir(path: &Path) -> Result<(), String> {
    let file = File::open(path).map_err(|e| format!("Failed to flush vault directory: {e}"))?;
    file.sync_all()
        .map_err(|e| format!("Failed to flush vault directory: {e}"))?;
    Ok(())
}

fn rotate_backups(vault_path: &Path, pending: &Path) -> Result<(), String> {
    let bak1 = vault_path.with_extension("vault.bak.1");
    let bak2 = vault_path.with_extension("vault.bak.2");
    let bak3 = vault_path.with_extension("vault.bak.3");
    if bak2.exists() {
        fs::rename(&bak2, &bak3).map_err(|e| format!("Failed to rotate backup: {e}"))?;
    }
    if bak1.exists() {
        fs::rename(&bak1, &bak2).map_err(|e| format!("Failed to rotate backup: {e}"))?;
    }
    if pending.exists() {
        fs::rename(pending, &bak1).map_err(|e| format!("Failed to rotate backup: {e}"))?;
    }
    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn vault_identity(path: &Path) -> PathBuf {
    if path.exists() {
        return fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    }
    if let Some(parent) = path.parent() {
        if let Ok(parent) = fs::canonicalize(parent) {
            if let Some(name) = path.file_name() {
                return parent.join(name);
            }
        }
    }
    path.to_path_buf()
}

#[cfg(target_os = "linux")]
fn try_lock_exclusive(file: &File) -> Result<(), String> {
    use std::os::fd::AsRawFd;
    let lock = libc::flock {
        l_type: libc::F_WRLCK as i16,
        l_whence: libc::SEEK_SET as libc::c_short,
        l_start: 0,
        l_len: 0,
        l_pid: 0,
    };
    let rc = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_OFD_SETLK, &lock) };
    if rc == 0 {
        Ok(())
    } else {
        Err("Vault is already open".to_string())
    }
}

#[cfg(not(target_os = "linux"))]
fn try_lock_exclusive(file: &File) -> Result<(), String> {
    fs4::FileExt::try_lock_exclusive(file).map_err(|_| "Vault is already open".to_string())
}

#[cfg(target_os = "linux")]
fn unlock_exclusive(file: &File) {
    use std::os::fd::AsRawFd;
    let lock = libc::flock {
        l_type: libc::F_UNLCK as i16,
        l_whence: libc::SEEK_SET as libc::c_short,
        l_start: 0,
        l_len: 0,
        l_pid: 0,
    };
    unsafe {
        libc::fcntl(file.as_raw_fd(), libc::F_OFD_SETLK, &lock);
    }
}

#[cfg(not(target_os = "linux"))]
fn unlock_exclusive(file: &File) {
    let _ = fs4::FileExt::unlock(file);
}

fn ensure_data_key(
    conn: &rusqlite::Connection,
    kek: &[u8],
    header: &VaultHeader,
) -> Result<(Secret<Vec<u8>>, bool), String> {
    let existing = match conn.query_row(
        "SELECT value FROM meta WHERE key = 'vault_data_key_envelope'",
        [],
        |row| row.get::<_, String>(0),
    ) {
        Ok(value) => Some(value),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(error) => return Err(format!("Failed to read vault data key: {error}")),
    };

    if let Some(json) = existing {
        let envelope: KeyEnvelope = serde_json::from_str(&json)
            .map_err(|_| "Stored vault key envelope is invalid".to_string())?;
        return Ok((unwrap_data_key(kek, &envelope)?, false));
    }

    let data_key = generate_data_key();
    let envelope = wrap_data_key(
        kek,
        data_key.expose_secret(),
        &header.salt,
        header.mem_cost,
        header.time_cost,
        header.parallelism,
    )?;
    let json = serde_json::to_string(&envelope)
        .map_err(|_| "Failed to encode vault key envelope".to_string())?;
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('vault_data_key_envelope', ?1)",
        params![json],
    )
    .map_err(|error| format!("Failed to store vault data key: {error}"))?;
    Ok((data_key, true))
}

/// Test hook: the next save fails after the new bytes are durable on a temporary
/// file and before they replace the vault.
#[cfg(test)]
pub fn fail_next_save_before_replace() {
    FAIL_BEFORE_REPLACE.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_replace_leaves_the_previous_vault_readable() {
        let path = std::env::temp_dir().join(format!("rvault-interrupt-{}.vault", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut session = VaultSession::create(&path, "session_pass").unwrap();
        session
            .connection()
            .unwrap()
            .execute("INSERT INTO meta (key, value) VALUES ('kept', 'yes')", [])
            .unwrap();
        session.mark_dirty();
        session.save().unwrap();
        session
            .connection()
            .unwrap()
            .execute("INSERT INTO meta (key, value) VALUES ('lost', 'no')", [])
            .unwrap();
        session.mark_dirty();
        fail_next_save_before_replace();
        assert!(session.save().is_err());
        drop(session);

        let reopened = VaultSession::open(&path, "session_pass").unwrap();
        let lost: Result<String, _> = reopened.connection().unwrap().query_row(
            "SELECT value FROM meta WHERE key = 'lost'",
            [],
            |row| row.get(0),
        );
        assert!(lost.is_err());
        let kept: String = reopened
            .connection()
            .unwrap()
            .query_row("SELECT value FROM meta WHERE key = 'kept'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(kept, "yes");
        drop(reopened);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("vault.lock"));
        let _ = std::fs::remove_file(path.with_extension("vault.bak.1"));
        let _ = std::fs::remove_file(path.with_extension("vault.recovery.db"));
    }

    #[test]
    fn second_session_is_refused_while_the_first_is_open() {
        let path = std::env::temp_dir().join(format!("rvault-twice-{}.vault", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let session = VaultSession::create(&path, "session_pass").unwrap();
        let second = VaultSession::open(&path, "session_pass");
        let message = match second {
            Err(error) => error,
            Ok(_) => panic!("second open should fail"),
        };
        assert!(message.contains("already open"));
        drop(session);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("vault.lock"));
    }
}
