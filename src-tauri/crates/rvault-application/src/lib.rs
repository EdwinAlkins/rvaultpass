//! One open vault, one persistence policy.

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;
use rvault_core::{
    EntryEditData, EntryPatch, EntrySummary, PasswordGenerationParams, TOTPResult, VaultEntry,
    VaultFolder, VaultTag,
};
use rvault_storage::{operations, StorageError, VaultSession};

pub struct VaultService {
    inner: Mutex<Option<VaultSession>>,
}

impl VaultService {
    pub fn create(vault_path: impl Into<PathBuf>, master_password: &str) -> Result<Self, String> {
        Ok(Self {
            inner: Mutex::new(Some(VaultSession::create(vault_path, master_password)?)),
        })
    }

    pub fn open(vault_path: impl Into<PathBuf>, master_password: &str) -> Result<Self, String> {
        Ok(Self {
            inner: Mutex::new(Some(VaultSession::open(vault_path, master_password)?)),
        })
    }

    pub fn is_open(&self) -> bool {
        self.inner
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    pub fn accepts_password(&self, master_password: &str) -> Result<bool, String> {
        let guard = self.inner.lock().map_err(|_| "Vault session is unavailable".to_string())?;
        let session = guard.as_ref().ok_or("Vault is locked")?;
        session.accepts_password(master_password)
    }

    /// Save, then drop the session. A failed save leaves the vault open.
    pub fn close(&self) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "Vault session is unavailable".to_string())?;
        if let Some(session) = guard.as_mut() {
            session.save()?;
        }
        guard.take();
        Ok(())
    }

    pub fn save(&self) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "Vault session is unavailable".to_string())?;
        let session = guard.as_mut().ok_or("No vault is currently open")?;
        session.save()
    }

    pub fn mutate<T>(
        &self,
        write: impl FnOnce(&Connection) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| StorageError::internal("Vault session is unavailable"))?;
        let session = guard
            .as_mut()
            .ok_or_else(|| StorageError::internal("Vault is locked"))?;
        let was_dirty = session.is_dirty();
        session.mark_dirty();
        let result = {
            let conn = session.connection().map_err(StorageError::internal)?;
            let tx = conn
                .unchecked_transaction()
                .map_err(|error| StorageError::internal(error.to_string()))?;
            match write(&tx) {
                Ok(value) => tx
                    .commit()
                    .map(|_| value)
                    .map_err(|error| StorageError::internal(error.to_string())),
                Err(error) => Err(error),
            }
        };
        match result {
            Ok(value) => match session.save() {
                Ok(()) => Ok(value),
                Err(error) => Err(StorageError::Internal(format!(
                    "Changes are in the working database but not durable: {error}"
                ))),
            },
            Err(error) => {
                if !was_dirty {
                    session.clear_dirty();
                }
                Err(error)
            }
        }
    }

    pub fn read<T>(&self, read: impl FnOnce(&Connection) -> Result<T, StorageError>) -> Result<T, StorageError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| StorageError::internal("Vault session is unavailable"))?;
        let session = guard
            .as_ref()
            .ok_or_else(|| StorageError::internal("Vault is locked"))?;
        let conn = session.connection().map_err(StorageError::internal)?;
        read(&conn)
    }

    pub fn create_entry(
        &self,
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
    ) -> Result<EntrySummary, StorageError> {
        self.mutate(|conn| {
            operations::create_entry(
                conn,
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
            .map(|entry| entry.to_summary())
        })
    }

    pub fn update_entry(&self, entry_id: String, patch: EntryPatch) -> Result<EntrySummary, StorageError> {
        self.mutate(|conn| {
            operations::update_entry(conn, &entry_id, patch).map(|entry| entry.to_summary())
        })
    }

    pub fn delete_entry(&self, entry_id: &str) -> Result<(), StorageError> {
        self.mutate(|conn| operations::delete_entry(conn, entry_id))
    }

    pub fn list_entries(&self) -> Result<Vec<EntrySummary>, StorageError> {
        self.read(operations::list_entries)
    }

    pub fn get_entry(&self, entry_id: &str) -> Result<VaultEntry, StorageError> {
        self.read(|conn| operations::get_entry(conn, entry_id))
    }

    pub fn edit_entry(&self, entry_id: &str) -> Result<EntryEditData, StorageError> {
        self.get_entry(entry_id).map(|entry| entry.to_edit_data())
    }

    pub fn search(&self, query: &str) -> Result<Vec<EntrySummary>, StorageError> {
        self.read(|conn| operations::search_entries(conn, query))
    }

    pub fn reveal_password(&self, entry_id: &str) -> Result<String, StorageError> {
        self.get_entry(entry_id).map(|entry| entry.password)
    }

    pub fn create_folder(&self, name: String, parent_id: Option<String>) -> Result<VaultFolder, StorageError> {
        self.mutate(|conn| operations::create_folder(conn, name, parent_id))
    }

    pub fn list_folders(&self) -> Result<Vec<VaultFolder>, StorageError> {
        self.read(operations::list_folders)
    }

    pub fn update_folder(
        &self,
        folder_id: &str,
        name: String,
        parent_id: Option<String>,
    ) -> Result<VaultFolder, StorageError> {
        let folder_id = folder_id.to_string();
        self.mutate(|conn| operations::update_folder(conn, &folder_id, name, parent_id))
    }

    pub fn delete_folder(&self, folder_id: &str) -> Result<(), StorageError> {
        let folder_id = folder_id.to_string();
        self.mutate(|conn| operations::delete_folder(conn, &folder_id))
    }

    pub fn create_tag(&self, name: String) -> Result<VaultTag, StorageError> {
        self.mutate(|conn| operations::create_tag(conn, name))
    }

    pub fn list_tags(&self) -> Result<Vec<VaultTag>, StorageError> {
        self.read(operations::list_tags)
    }

    pub fn delete_tag(&self, tag_id: &str) -> Result<(), StorageError> {
        let tag_id = tag_id.to_string();
        self.mutate(|conn| operations::delete_tag(conn, &tag_id))
    }

    pub fn generate_totp(&self, entry_id: &str) -> Result<TOTPResult, StorageError> {
        let entry = self.get_entry(entry_id)?;
        let secret = entry
            .totp_secret
            .filter(|value| !value.is_empty())
            .ok_or_else(|| StorageError::Invalid("Entry has no TOTP secret".to_string()))?;
        operations::generate_totp(&secret).map_err(StorageError::Invalid)
    }
}

pub fn generate_password(params: &PasswordGenerationParams) -> Result<String, String> {
    operations::generate_password(params)
}

pub fn generate_passphrase(word_count: usize) -> Result<String, String> {
    operations::generate_passphrase(word_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn concurrent_mutations_keep_both_entries() {
        let path = std::env::temp_dir().join(format!("rvault-concurrent-{}.vault", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let service = Arc::new(VaultService::create(&path, "session_pass").unwrap());
        let first = {
            let service = Arc::clone(&service);
            thread::spawn(move || {
                service.create_entry(
                    None, "One".into(), None, None, None, None, "alpha".into(), None, None, false, None,
                )
            })
        };
        let second = {
            let service = Arc::clone(&service);
            thread::spawn(move || {
                service.create_entry(
                    None, "Two".into(), None, None, None, None, "beta".into(), None, None, false, None,
                )
            })
        };
        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();
        let titles: Vec<_> = service.list_entries().unwrap().into_iter().map(|entry| entry.title).collect();
        assert!(titles.contains(&"One".to_string()));
        assert!(titles.contains(&"Two".to_string()));
        service.close().unwrap();
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("vault.lock"));
    }
}
