#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod crypto;
mod db;
mod commands;
mod models;
mod vault;

use commands::{
    create_vault, open_vault, lock_vault, save_vault, is_vault_open,
    create_entry, get_all_entries, get_entry, update_entry, delete_entry,
    create_folder, get_all_folders, update_folder, delete_folder,
    create_tag, get_all_tags, delete_tag,
    search_entries,
    generate_password, generate_passphrase,
    generate_totp,
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
            // TOTP
            generate_totp,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
