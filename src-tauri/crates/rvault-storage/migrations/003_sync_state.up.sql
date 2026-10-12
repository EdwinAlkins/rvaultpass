CREATE TABLE IF NOT EXISTS sync_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_objects (
    object_id TEXT PRIMARY KEY,
    object_type TEXT NOT NULL,
    server_version INTEGER NOT NULL,
    content_hash TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_conflicts (
    conflict_id TEXT PRIMARY KEY,
    object_id TEXT NOT NULL,
    object_type TEXT NOT NULL,
    remote_version INTEGER NOT NULL,
    remote_device_id TEXT NOT NULL,
    remote_nonce TEXT NOT NULL,
    remote_ciphertext TEXT NOT NULL,
    remote_deleted INTEGER NOT NULL,
    local_canonical TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
