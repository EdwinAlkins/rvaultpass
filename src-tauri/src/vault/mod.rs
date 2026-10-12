//! Process-wide vault service used by the desktop commands.

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use rvault_application::VaultService;
use rvault_core::{
    EntryEditData, EntryPatch, EntrySummary, PasswordGenerationParams, TOTPResult, VaultFolder,
    VaultTag,
};
use rvault_storage::StorageError;

static SERVICE: Lazy<Mutex<Option<VaultService>>> = Lazy::new(|| Mutex::new(None));
static LAST_ACTIVITY: AtomicI64 = AtomicI64::new(0);
static IDLE_SECS: AtomicI64 = AtomicI64::new(900);
static LOCK_ON_BLUR: AtomicBool = AtomicBool::new(false);

/// Vault manager for high-level operations.
pub struct VaultManager;

impl VaultManager {
    pub fn create_vault(vault_path: &str, master_password: &str) -> Result<(), String> {
        let mut state = SERVICE.lock().map_err(|_| "Vault session is unavailable".to_string())?;
        if state.is_some() {
            return Err("A vault is already open".to_string());
        }
        *state = Some(VaultService::create(vault_path, master_password)?);
        touch();
        Ok(())
    }

    pub fn open_vault(vault_path: &str, master_password: &str) -> Result<(), String> {
        let mut state = SERVICE.lock().map_err(|_| "Vault session is unavailable".to_string())?;
        if state.is_some() {
            return Err("A vault is already open".to_string());
        }
        *state = Some(VaultService::open(vault_path, master_password)?);
        touch();
        Ok(())
    }

    pub fn lock_vault() -> Result<(), String> {
        let mut state = SERVICE.lock().map_err(|_| "Vault session is unavailable".to_string())?;
        if let Some(service) = state.as_ref() {
            service.close()?;
        }
        state.take();
        Ok(())
    }

    pub fn save_vault() -> Result<(), String> {
        with_service_string(|service| service.save().map_err(StorageError::internal))
    }

    pub fn is_vault_open() -> bool {
        SERVICE
            .lock()
            .map(|state| state.as_ref().is_some_and(|service| service.is_open()))
            .unwrap_or(false)
    }

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
    ) -> Result<EntrySummary, String> {
        with_service(|service| {
            service.create_entry(
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
            )
        })
    }

    pub fn update_entry(entry_id: String, patch: EntryPatch) -> Result<EntrySummary, String> {
        with_service(|service| service.update_entry(entry_id, patch))
    }

    pub fn delete_entry(entry_id: &str) -> Result<(), String> {
        let entry_id = entry_id.to_string();
        with_service(|service| service.delete_entry(&entry_id))
    }

    pub fn list_entries() -> Result<Vec<EntrySummary>, String> {
        with_service(|service| service.list_entries())
    }

    pub fn edit_entry(entry_id: &str) -> Result<EntryEditData, String> {
        let entry_id = entry_id.to_string();
        with_service(|service| service.edit_entry(&entry_id))
    }

    pub fn get_password(entry_id: &str) -> Result<String, String> {
        let entry_id = entry_id.to_string();
        with_service(|service| service.reveal_password(&entry_id))
    }

    pub fn get_entry_secret(entry_id: &str) -> Result<rvault_core::VaultEntry, String> {
        let entry_id = entry_id.to_string();
        with_service(|service| service.get_entry(&entry_id))
    }

    pub fn search(query: &str) -> Result<Vec<EntrySummary>, String> {
        let query = query.to_string();
        with_service(|service| service.search(&query))
    }

    pub fn create_folder(name: String, parent_id: Option<String>) -> Result<VaultFolder, String> {
        with_service(|service| service.create_folder(name, parent_id))
    }

    pub fn list_folders() -> Result<Vec<VaultFolder>, String> {
        with_service(|service| service.list_folders())
    }

    pub fn update_folder(
        folder_id: &str,
        name: String,
        parent_id: Option<String>,
    ) -> Result<VaultFolder, String> {
        let folder_id = folder_id.to_string();
        with_service(|service| service.update_folder(&folder_id, name, parent_id))
    }

    pub fn delete_folder(folder_id: &str) -> Result<(), String> {
        let folder_id = folder_id.to_string();
        with_service(|service| service.delete_folder(&folder_id))
    }

    pub fn create_tag(name: String) -> Result<VaultTag, String> {
        with_service(|service| service.create_tag(name))
    }

    pub fn list_tags() -> Result<Vec<VaultTag>, String> {
        with_service(|service| service.list_tags())
    }

    pub fn delete_tag(tag_id: &str) -> Result<(), String> {
        let tag_id = tag_id.to_string();
        with_service(|service| service.delete_tag(&tag_id))
    }

    pub fn generate_totp(entry_id: &str) -> Result<TOTPResult, String> {
        let entry_id = entry_id.to_string();
        with_service(|service| service.generate_totp(&entry_id))
    }

    pub fn generate_password(params: &PasswordGenerationParams) -> Result<String, String> {
        rvault_application::generate_password(params)
    }

    pub fn generate_passphrase(word_count: usize) -> Result<String, String> {
        rvault_application::generate_passphrase(word_count)
    }

    pub fn import<T>(
        import: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        with_service(|service| {
            service.mutate(|conn| import(conn).map_err(StorageError::internal))
        })
    }

    pub fn set_lock_preferences(idle_secs: i64, lock_on_blur: bool) {
        IDLE_SECS.store(idle_secs.max(0), Ordering::Relaxed);
        LOCK_ON_BLUR.store(lock_on_blur, Ordering::Relaxed);
        touch();
    }

    pub fn lock_on_blur_enabled() -> bool {
        LOCK_ON_BLUR.load(Ordering::Relaxed)
    }

    pub fn lock_if_idle() -> Result<(), String> {
        if !Self::is_vault_open() {
            return Ok(());
        }
        let idle = IDLE_SECS.load(Ordering::Relaxed);
        if idle <= 0 {
            return Ok(());
        }
        let last = LAST_ACTIVITY.load(Ordering::Relaxed);
        if unix_now().saturating_sub(last) >= idle {
            Self::lock_vault()
        } else {
            Ok(())
        }
    }
}

fn with_service<T>(operation: impl FnOnce(&VaultService) -> Result<T, StorageError>) -> Result<T, String> {
    touch();
    let state = SERVICE.lock().map_err(|_| "Vault session is unavailable".to_string())?;
    let service = state.as_ref().ok_or("No vault is currently open")?;
    operation(service).map_err(|error| error.to_string())
}

fn with_service_string<T>(operation: impl FnOnce(&VaultService) -> Result<T, StorageError>) -> Result<T, String> {
    with_service(operation)
}

fn touch() {
    LAST_ACTIVITY.store(unix_now(), Ordering::Relaxed);
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn cleanup(path: &str) {
        let p = Path::new(path);
        for suffix in [
            "",
            ".lock",
            ".tmp",
            ".bak.pending",
            ".recovery.db",
        ] {
            if suffix.is_empty() {
                let _ = std::fs::remove_file(p);
            }
        }
        let _ = std::fs::remove_file(p);
        let _ = std::fs::remove_file(p.with_extension("vault.lock"));
        let _ = std::fs::remove_file(p.with_extension("vault.tmp"));
        let _ = std::fs::remove_file(p.with_extension("vault.bak.pending"));
        let _ = std::fs::remove_file(p.with_extension("vault.bak.1"));
        let _ = std::fs::remove_file(p.with_extension("vault.bak.2"));
        let _ = std::fs::remove_file(p.with_extension("vault.bak.3"));
        let _ = std::fs::remove_file(p.with_extension("vault.recovery.db"));
    }

    #[test]
    fn test_full_vault_lifecycle() {
        let vault_path = std::env::temp_dir().join("test_lifecycle.vault");
        let vault_str = vault_path.to_str().unwrap();
        cleanup(vault_str);

        let result = VaultManager::create_vault(vault_str, "master_password_123");
        assert!(result.is_ok(), "Failed to create vault: {:?}", result);
        assert!(VaultManager::is_vault_open(), "Vault should be open after creation");
        assert!(vault_path.exists(), "Vault file should exist");

        let result = VaultManager::save_vault();
        assert!(result.is_ok(), "Failed to save vault: {:?}", result);

        let result = VaultManager::lock_vault();
        assert!(result.is_ok(), "Failed to lock vault: {:?}", result);
        assert!(!VaultManager::is_vault_open(), "Vault should be closed after lock");

        let result = VaultManager::open_vault(vault_str, "master_password_123");
        assert!(result.is_ok(), "Failed to reopen vault: {:?}", result);

        VaultManager::lock_vault().ok();
        let result = VaultManager::open_vault(vault_str, "wrong_password");
        assert!(result.is_err(), "Should fail to open with wrong password");
        assert!(!VaultManager::is_vault_open(), "Vault should not be open after failed auth");

        VaultManager::lock_vault().ok();
        cleanup(vault_str);
    }

    #[test]
    fn test_second_open_is_refused() {
        let vault_path = std::env::temp_dir().join("test_second_open.vault");
        let vault_str = vault_path.to_str().unwrap();
        cleanup(vault_str);
        VaultManager::create_vault(vault_str, "master_password_123").unwrap();
        let second = VaultManager::open_vault(vault_str, "master_password_123");
        assert!(second.unwrap_err().contains("already open"));
        assert!(VaultManager::is_vault_open());
        VaultManager::lock_vault().ok();
        cleanup(vault_str);
    }

    #[test]
    fn test_vault_backup_rotation() {
        let vault_path = std::env::temp_dir().join("test_backup.vault");
        let vault_str = vault_path.to_str().unwrap();
        cleanup(vault_str);
        VaultManager::create_vault(vault_str, "test_pass_1").unwrap();
        VaultManager::save_vault().unwrap();
        assert!(vault_path.with_extension("vault.bak.1").exists());
        VaultManager::save_vault().unwrap();
        assert!(vault_path.with_extension("vault.bak.2").exists());
        VaultManager::save_vault().unwrap();
        assert!(vault_path.with_extension("vault.bak.3").exists());
        VaultManager::lock_vault().ok();
        cleanup(vault_str);
    }

    #[test]
    fn test_create_duplicate_vault_fails() {
        let vault_path = std::env::temp_dir().join("test_dup.vault");
        let vault_str = vault_path.to_str().unwrap();
        cleanup(vault_str);
        assert!(VaultManager::create_vault(vault_str, "password-one").is_ok());
        VaultManager::lock_vault().unwrap();
        let result2 = VaultManager::create_vault(vault_str, "password-two");
        assert!(result2.is_err(), "Creating duplicate vault should fail");
        assert!(result2.unwrap_err().contains("already exists"));
        VaultManager::lock_vault().ok();
        cleanup(vault_str);
    }

    #[test]
    fn test_vault_data_persists_across_sessions() {
        let vault_path = std::env::temp_dir().join("test_persist.vault");
        let vault_str = vault_path.to_str().unwrap();
        cleanup(vault_str);
        VaultManager::create_vault(vault_str, "session_pass").unwrap();
        let created = VaultManager::create_entry(
            None,
            "Persisted".into(),
            None,
            None,
            None,
            None,
            "secret-value".into(),
            None,
            None,
            false,
            None,
        )
        .unwrap();
        VaultManager::lock_vault().unwrap();
        VaultManager::open_vault(vault_str, "session_pass").unwrap();
        let entries = VaultManager::list_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, created.id);
        assert_eq!(entries[0].title, "Persisted");
        VaultManager::lock_vault().ok();
        cleanup(vault_str);
    }

    #[test]
    fn test_lock_keeps_session_when_save_fails() {
        let dir = std::env::temp_dir().join(format!("rvault-lock-fail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let vault_path = dir.join("locked.vault");
        let vault_str = vault_path.to_str().unwrap();
        VaultManager::create_vault(vault_str, "session_pass").unwrap();
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&dir, perms).unwrap();
        let saved = VaultManager::save_vault();
        assert!(saved.is_err(), "save should fail when the directory is read-only");
        let locked = VaultManager::lock_vault();
        assert!(locked.is_err(), "lock must not discard unsaved changes");
        assert!(VaultManager::is_vault_open());
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(&dir, perms).unwrap();
        VaultManager::lock_vault().ok();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
