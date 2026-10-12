//! Local SQLCipher cache. Callers pass a connection from the open session.

mod apply;
mod db;
mod error;
pub mod operations;
pub mod session;
pub mod sync_state;

pub use apply::{apply_entry, apply_folder, apply_tag, delete_synced, read_entry_secret};
pub use db::{create_new_vault, init_connection, run_migrations, test_vault_connection};
pub use error::StorageError;
pub use session::VaultSession;
