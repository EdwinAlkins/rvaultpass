📘 CAHIER DES CHARGES v1.0
Gestionnaire de Mots de Passe Local (Modèle Fichier Unique)
Stack : Tauri v2 (Rust + preact) | rusqlite + SQLCipher | Open Source
1. 🎯 Vision & Objectifs

    Application desktop cross-platform (Windows, macOS, Linux)
    Modèle 100% local : un seul fichier .vault porte l'intégralité du coffre-fort
    Architecture zero-knowledge par conception : aucun serveur, aucune télémétrie, aucun échange réseau
    Priorité absolue : sécurité cryptographique, résilience aux pannes, transparence open-source
    Livrable V1 stable, auditable, avec CI/CD et documentation publique prête pour GitHub

2. 🏗️ Architecture Technique

```mermaid
┌─────────────────────┐     IPC Sécurisé      ┌─────────────────────┐
│   preact (Frontend)  │ ◄────────────────────► │   Tauri (Rust)      │
│ • Vite + TS + Tailwind│   (tauri::command)   │ • Validation stricte│
│ • Zustand (état UI) │                        │ • rusqlite + SQLCipher│
│ • i18n + Thème      │                        │ • argon2 + zeroize  │
└─────────────────────┘                        └────────┬────────────┘
                                                       │
                                                ┌──────▼──────┐
                                                │ Fichier .vault│
                                                │ • Header 96B  │
                                                │ • SQLCipher   │
                                                │ (AES-256-CBC) │
                                                └───────────────┘
```
    Séparation stricte : Le contexte JS/preact ne manipule jamais de clés ou mots de passe en clair.
    Flux de données : UI → tauri::command (payload validé) → Rust → déchiffrement mémoire → DB → réponse sérialisée.
    État applicatif : Zustand gère l'UI, TanStack Query ou hooks custom pour les requêtes DB asynchrones via IPC.

3. 📁 Spécification du Fichier Vault
3.1 En-tête (Header non chiffré - 96 octets)
Offset
	
Taille
	
Champ
	
Description
0x00
	
4B
	
Magic
	
"KVLT" (identifiant format)
0x04
	
1B
	
Version
	
0x01 (version du format)
0x05
	
16B
	
Salt
	
Sel aléatoire pour Argon2id
0x15
	
4B
	
MemCost
	
65536 (64 MiB)
0x19
	
2B
	
TimeCost
	
3 (itérations)
0x1B
	
2B
	
Parallel
	
1 (thread)
0x1D
	
32B
	
HeaderHMAC
	
SHA-256 des octets 0x00 à 0x1C
0x3D
	
35B
	
Padding
	
Réservé pour évolutions futures
3.2 Flux d'Ouverture & Sauvegarde

    Lecture header → vérification HeaderHMAC (détection corruption/altération)
    Demande du mot de passe maître (UI → Rust via secrecy::Secret<String>)
    Dérivation clé 256-bit via argon2id (paramètres fixes lus dans le header)
    Injection clé brute dans SQLCipher : PRAGMA key = "x'<CLÉ_HEX>'";
    Exécution des migrations (rusqlite-migration)
    Ouverture DB rusqlite → session sécurisée
    Verrouillage/Sortie : PRAGMA rekey = ""; → zeroize() sur toutes les variables sensibles → fermeture connexion
    Sauvegarde atomique : écriture dans .vault.tmp → rename() → suppression ancien → conservation .bak.1 à .bak.3

4. 🗃️ Base de Données & Migrations
4.1 Moteur & Versionning

    Bibliothèque : rusqlite avec feature bundled-sqlcipher
    Migration : rusqlite-migration (équivalent Alembic pour Rust)
        Fichiers SQL purs embarqués via include_str!
        Exécutés uniquement après PRAGMA key
        Enveloppés dans une transaction (rollback auto sur échec)
        Table de version : _rusqlite_migration_schema_version

4.2 Schéma Initial (001_create_core_tables.up.sql)

```sql
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS folders (
    id BLOB PRIMARY KEY, name TEXT NOT NULL,
    parent_id BLOB REFERENCES folders(id),
    created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS tags (id BLOB PRIMARY KEY, name TEXT UNIQUE NOT NULL);
CREATE TABLE IF NOT EXISTS entries (
    id BLOB PRIMARY KEY, folder_id BLOB REFERENCES folders(id),
    title TEXT NOT NULL, url TEXT, username TEXT, password TEXT NOT NULL,
    notes TEXT, totp_secret TEXT,
    created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
    is_favorite INTEGER DEFAULT 0 CHECK (is_favorite IN (0,1))
);
CREATE TABLE IF NOT EXISTS entry_tags (
    entry_id BLOB REFERENCES entries(id) ON DELETE CASCADE,
    tag_id BLOB REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (entry_id, tag_id)
);
CREATE INDEX IF NOT EXISTS idx_entries_title ON entries(title);
CREATE INDEX IF NOT EXISTS idx_entries_url ON entries(url);
CREATE INDEX IF NOT EXISTS idx_entries_folder ON entries(folder_id);
CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(title, url, username, notes, content=entries);
```

4.3 Règles de Migration

    ❌ Jamais de DROP TABLE/COLUMN en production V1
    ✅ Uniquement ALTER TABLE ADD COLUMN ou création de tables
    ✅ Numérotation stricte 001_..., 002_... (ne jamais renommer/supprimer)
    ✅ Gestion d'erreur UI : "Vault corrompu ou version incompatible. Restaurez une sauvegarde."

5. 🔒 Exigences de Sécurité (Non-Négociables)
Domaine
	
Règle Stricte
Mémoire
	
zeroize::Zeroize sur tout Vec<u8>, String, ou struct contenant MP/clé. Drop garantit l'effacement.
Types sensibles
	
secrecy::Secret<T> pour tout transit temporaire. Jamais de &str ou String brut pour les secrets.
Contexte JS
	
ZÉRO donnée sensible exposée au frontend preact. Les mots de passe ne transitent que via tauri::command typés.
IPC Tauri
	
Validation stricte des payloads (serde + schémas). devtools désactivé en production.
Presse-papier
	
Auto-clear après 30s. Option désactivable. Avertissement si autre app lit le clipboard.
Verrouillage
	
Auto-lock configurable (inactivité, perte de focus, veille). Verrou manuel via raccourci/bouton.
Sauvegarde
	
Écriture atomique : .vault.tmp → rename() → suppression ancien. Backup .bak.1 à .bak.3 conservés.
Logs
	
tracing filtré à INFO minimum. Aucune donnée vault, MP ou clé dans les logs/stdout.
Mises à jour
	
Tauri Updater avec signature Ed25519. Vérification intégrité binaire obligatoire avant installation.
6. ✅ Périmètre Fonctionnel V1
Module
	
Fonctionnalités
Vault
	
Création/ouverture fichier, vérification header, auto-save atomique, backup .bak
Auth
	
MP maître unique, Argon2id fixe, dérivation Rust, zeroize mémoire
DB
	
rusqlite + SQLCipher, CRUD complet, FTS recherche, filtres/tri
Générateur
	
Longueur, charset, passphrase Diceware, copie sécurisée
TOTP
	
Saisie secret/URI, génération 30s, copie + auto-clear 30s
Import/Export
	
CSV, JSON (compatible Bitwarden), JSON interne (backup)
UI/UX
	
Dark/Light mode, responsive desktop, i18n, notifications non-intrusives, gestion dossiers/tags
Open Source
	
SECURITY.md, threat model, CI (test/clippy/audit/build), releases signées, docs architecture
7. 🚫 Limites Explicites (Hors V1)

    ❌ Sync cloud / multi-device automatique
    ❌ Fichier clé (keyfile) ou support hardware token (YubiKey)
    ❌ Historique des modifications par entrée
    ❌ Autofill navigateur / intégration OS native
    ❌ Compatibilité native .kdbx KeePass
    ❌ Double chiffrement applicatif (redondant avec SQLCipher)
    ❌ Partage chiffré entre utilisateurs

8. 🛠️ Stack & Dépendances Techniques
Couche
	
Dépendances / Features
Frontend
	
preact@19, typescript, vite, zustand, tailwindcss, preact-i18next, @tauri-apps/api@2
Backend
	
tauri@2, serde, serde_json, rmp-serde, uuid, chrono, tracing
DB
	
rusqlite = { version = "0.32", features = ["bundled-sqlcipher", "modern_sqlite"] }
Migrations
	
rusqlite-migration = "1"
Crypto
	
argon2 = "0.5", ring = "0.17", zeroize = "1", secrecy = "0.8"
Build/CI
	
GitHub Actions : cargo test, cargo clippy, cargo audit, pnpm build, tauri build
Packaging
	
Tauri v2 bundler (AppImage, DMG, MSI/NSIS) + signature Ed25519
9. 📅 Roadmap de Développement (Estimation Solo)
Phase
	
Durée
	
Livrables
1. Setup & Core Crypto
	
Sem 1-2
	
Arborescence, CI, parser header, module Argon2id, tests zeroize, validation HMAC
2. DB & Migrations
	
Sem 3-4
	
Intégration SQLCipher + rusqlite, rusqlite-migration, commandes Tauri typées, sauvegarde atomique
3. UI & Features
	
Sem 5-6
	
Interface preact complète, recherche FTS, générateur MP, TOTP, import/export
4. Hardening & Docs
	
Sem 7
	
Auto-lock, clipboard manager, SECURITY.md, threat model, pré-release
5. Beta & Release
	
Sem 8
	
Tests cross-OS, bugfix, documentation, release GitHub signée
10. 🚀 Livrables & Prochaines Étapes
Ce document est figé et validé pour le développement V1.
Il servira de référence unique pour l'implémentation, les revues de code et les audits.
🔜 Génération Immédiate (sur confirmation)

    📦 Cargo.toml complet avec versions exactes + features
    🗂️ Arborescence Tauri v2 + preact + migrations/
    🔐 src-tauri/src/crypto.rs (Argon2id, HMAC, zeroize, tests)
    🗃️ src-tauri/src/db.rs (SQLCipher init, rusqlite-migration setup, connexion sécurisée)
    🔌 src-tauri/src/commands.rs (contrats IPC typés, validation, gestion mémoire)
    🛡️ SECURITY.md + Threat Model + .github/workflows/ci.yml
