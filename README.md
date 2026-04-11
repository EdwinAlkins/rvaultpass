# RVaultPass

> A secure, local, zero-knowledge password manager for your desktop.

**RVaultPass** stores all your passwords, notes, and TOTP tokens in a single encrypted `.vault` file — no server, no cloud, no telemetry. Your data never leaves your machine.

---

## Features

### 🔐 Security

- **Argon2id** key derivation (64 MiB memory, 3 iterations)
- **SQLCipher** (AES-256-CBC) encrypted SQLite database
- **Keyed HMAC-SHA256** header integrity verification
- **Zero-knowledge**: no sensitive data ever exposed to the frontend
- **Secure memory**: `secrecy::Secret<T>` + `zeroize` for all key material

### 📂 Vault Management

- Create and open password vaults with a master password
- **Backup rotation**: automatic `.bak.1` → `.bak.2` → `.bak.3` on save
- Atomic save (write to `.tmp`, then rename)
- Lock vault to clear encryption key from memory

### 🔑 Password Management

- **Full CRUD** for entries (title, URL, username, password, notes, TOTP secret)
- **Folders** for hierarchical organization
- **Tags** for cross-cutting categorization
- **Favorites** for quick access
- **Full-text search** (FTS5) across all fields

### ⚡ Tools

- **Password Generator** — configurable length, charset, and Diceware passphrase
- **TOTP Generator** — live 6-digit codes with countdown timer
- **Copy to clipboard** with 30-second warning

### 🎨 UI

- **Dark / Light theme** toggle
- **English / French** i18n
- **Toast notifications** (success, error, info)
- Responsive sidebar navigation

---

## Architecture

```
┌──────────────────────┐     IPC (Tauri Commands)     ┌──────────────────────┐
│   Preact Frontend    │ ◄──────────────────────────► │   Rust Backend       │
│ • Zustand (state)    │    24 typed commands         │ • SQLCipher DB       │
│ • i18n (EN/FR)       │                              │ • Argon2id + HMAC    │
│ • Tailwind CSS       │                              │ • zeroize + secrecy  │
└──────────────────────┘                              └──────────┬───────────┘
                                                                 │
                                                        ┌────────▼────────┐
                                                        │  .vault file     │
                                                        │ • 96B header     │
                                                        │ • SQLCipher DB   │
                                                        │ • AES-256-CBC    │
                                                        └─────────────────┘
```

### Vault File Format

```
[ 96-byte Header ] [ SQLCipher Encrypted Database ]
 ├─ Magic ("KVLT")    ├─ AES-256-CBC encrypted SQLite
 ├─ Version (0x01)    ├─ FTS5 full-text search index
 ├─ Salt (16B)        ├─ entries, folders, tags
 ├─ Argon2 params     └─ entry_tags junction table
 └─ HMAC-SHA256 (32B, keyed)
```

---

## Getting Started

### Prerequisites

- **Node.js** ≥ 18
- **Rust** ≥ 1.70 (edition 2021)
- System dependencies for [Tauri](https://tauri.app/start/prerequisites/)

### Development

```bash
# Install dependencies
npm install

# Run the app (Vite dev server + Tauri)
npm run tauri dev
```

### Production Build

```bash
# Build desktop app (creates platform-specific installers)
npm run tauri build
```

### Other Commands

```bash
# Preview production build
npm run preview

# Run Rust tests
cd src-tauri && cargo test -- --test-threads=1
```

---

## Tech Stack


| Layer        | Technology                                  |
| ------------ | ------------------------------------------- |
| **Frontend** | Preact 10 · TypeScript · Vite 6 · Zustand 5 |
| **Styling**  | Tailwind CSS 4 · CSS Custom Properties      |
| **i18n**     | i18next 26 · react-i18next 17               |
| **Backend**  | Tauri 2 · Rust (2021)                       |
| **Database** | rusqlite 0.32 + SQLCipher (OpenSSL)         |
| **Crypto**   | Argon2 0.5 · HMAC-SHA256 · SHA-1 (TOTP)     |
| **Security** | zeroize 1 · secrecy 0.8 · ring 0.17         |


---

## Running Tests

```bash
cd src-tauri
cargo test -- --test-threads=1
```

**9 tests pass** (5 unit + 4 integration):


| Test                                       | Coverage                                        |
| ------------------------------------------ | ----------------------------------------------- |
| `test_header_serialization`                | 96-byte header roundtrip                        |
| `test_hmac_with_key`                       | Keyed HMAC uniqueness                           |
| `test_key_derivation`                      | Argon2id 256-bit output                         |
| `test_zeroize_on_drop`                     | `Secret<T>` memory safety                       |
| `test_create_and_open_vault`               | SQLCipher lifecycle                             |
| `test_full_vault_lifecycle`                | Create → Save → Lock → Reopen → Reject wrong pw |
| `test_vault_backup_rotation`               | `.bak.1` → `.bak.2` → `.bak.3`                  |
| `test_create_duplicate_vault_fails`        | Duplicate rejection                             |
| `test_vault_data_persists_across_sessions` | Data persistence across lock/reopen             |


---

## Tauri Commands (IPC API)

### Vault Lifecycle


| Command                        | Description                          |
| ------------------------------ | ------------------------------------ |
| `create_vault(path, password)` | Create a new `.vault` file           |
| `open_vault(path, password)`   | Open an existing vault               |
| `lock_vault()`                 | Lock vault and clear key from memory |
| `save_vault()`                 | Atomic save with backup rotation     |
| `is_vault_open()`              | Check if vault is currently open     |


### Entry CRUD


| Command                 | Description                 |
| ----------------------- | --------------------------- |
| `create_entry(...)`     | Create a new password entry |
| `get_all_entries()`     | List all entries            |
| `get_entry(id)`         | Get a single entry          |
| `update_entry(id, ...)` | Update an entry             |
| `delete_entry(id)`      | Delete an entry             |


### Folders & Tags


| Command                              | Description                         |
| ------------------------------------ | ----------------------------------- |
| `create_folder(name, parent_id)`     | Create a folder                     |
| `get_all_folders()`                  | List all folders                    |
| `update_folder(id, name, parent_id)` | Update a folder                     |
| `delete_folder(id)`                  | Delete a folder (unassigns entries) |
| `create_tag(name)`                   | Create a tag                        |
| `get_all_tags()`                     | List all tags                       |
| `delete_tag(id)`                     | Delete a tag                        |


### Search & Tools


| Command                      | Description                  |
| ---------------------------- | ---------------------------- |
| `search_entries(query)`      | FTS5 full-text search        |
| `generate_password(params)`  | Generate random password     |
| `generate_passphrase(count)` | Generate Diceware passphrase |
| `generate_totp(secret)`      | Generate TOTP 6-digit code   |


---

## Out of Scope (V1)

- ❌ Cloud sync / multi-device
- ❌ Key file or hardware token (YubiKey)
- ❌ Change history per entry
- ❌ Browser autofill / OS integration
- ❌ KeePass `.kdbx` compatibility
- ❌ Encrypted sharing between users

---

## License

MIT