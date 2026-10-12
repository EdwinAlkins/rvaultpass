#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod import;
mod models;
mod vault;
mod clipboard;

use commands::{
    create_vault, open_vault, lock_vault, save_vault, is_vault_open,
    set_lock_preferences,
    create_entry, get_all_entries, get_entry, update_entry, delete_entry,
    create_folder, get_all_folders, update_folder, delete_folder,
    create_tag, get_all_tags, delete_tag,
    search_entries,
    generate_password, generate_passphrase,
    reveal_password, copy_password, copy_entry_field,
    generate_totp_for_entry, copy_totp, copy_to_clipboard,
    import_dashlane_csv,
};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    std::thread::spawn(|| {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            if let Err(error) = vault::VaultManager::lock_if_idle() {
                tracing::error!("Automatic lock failed: {error}");
            }
        }
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            // Vault lifecycle
            create_vault,
            open_vault,
            lock_vault,
            save_vault,
            is_vault_open,
            set_lock_preferences,
            // Entry CRUD
            create_entry,
            get_all_entries,
            get_entry,
            update_entry,
            delete_entry,
            // Folder CRUD
            create_folder,
            get_all_folders,
            update_folder,
            delete_folder,
            // Tag CRUD
            create_tag,
            get_all_tags,
            delete_tag,
            // Search
            search_entries,
            // Password generator
            generate_password,
            generate_passphrase,
            // Secret access
            reveal_password,
            copy_password,
            copy_entry_field,
            generate_totp_for_entry,
            copy_totp,
            copy_to_clipboard,
            // Import
            import_dashlane_csv,
        ])
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                if vault::VaultManager::lock_on_blur_enabled() {
                    if let Err(error) = vault::VaultManager::lock_vault() {
                        tracing::error!("Lock on blur failed: {error}");
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
