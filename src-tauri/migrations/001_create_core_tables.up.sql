-- Migration 001: Create core tables
-- This migration creates the foundational tables for the vault

-- Metadata table
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Insert version
INSERT OR IGNORE INTO meta (key, value) VALUES ('version', '1');

-- Folders table
CREATE TABLE IF NOT EXISTS folders (
    id BLOB PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id BLOB REFERENCES folders(id),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Tags table
CREATE TABLE IF NOT EXISTS tags (
    id BLOB PRIMARY KEY,
    name TEXT UNIQUE NOT NULL
);

-- Entries table
CREATE TABLE IF NOT EXISTS entries (
    id BLOB PRIMARY KEY,
    folder_id BLOB REFERENCES folders(id),
    title TEXT NOT NULL,
    url TEXT,
    username TEXT,
    password TEXT NOT NULL,
    notes TEXT,
    totp_secret TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    is_favorite INTEGER DEFAULT 0 CHECK (is_favorite IN (0,1))
);

-- Entry-Tag junction table
CREATE TABLE IF NOT EXISTS entry_tags (
    entry_id BLOB REFERENCES entries(id) ON DELETE CASCADE,
    tag_id BLOB REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (entry_id, tag_id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_entries_title ON entries(title);
CREATE INDEX IF NOT EXISTS idx_entries_url ON entries(url);
CREATE INDEX IF NOT EXISTS idx_entries_folder ON entries(folder_id);

-- Full-text search virtual table
CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(
    title,
    url,
    username,
    notes,
    content=entries
);
