-- Rebuild the external-content FTS index and keep it in sync with triggers.
DROP TRIGGER IF EXISTS entries_fts_insert;
DROP TRIGGER IF EXISTS entries_fts_delete;
DROP TRIGGER IF EXISTS entries_fts_update;
DROP TRIGGER IF EXISTS entries_fts_update_after;
DROP TABLE IF EXISTS entries_fts;

CREATE VIRTUAL TABLE entries_fts USING fts5(
    title,
    url,
    username,
    notes,
    content=entries,
    content_rowid=rowid
);

CREATE TRIGGER entries_fts_insert AFTER INSERT ON entries BEGIN
    INSERT INTO entries_fts(rowid, title, url, username, notes)
    VALUES (
        new.rowid,
        new.title,
        coalesce(new.url, ''),
        trim(coalesce(new.username, '') || ' ' || coalesce(new.username2, '') || ' ' || coalesce(new.username3, '')),
        coalesce(new.notes, '')
    );
END;

CREATE TRIGGER entries_fts_delete BEFORE DELETE ON entries BEGIN
    INSERT INTO entries_fts(entries_fts, rowid, title, url, username, notes)
    VALUES (
        'delete',
        old.rowid,
        old.title,
        coalesce(old.url, ''),
        trim(coalesce(old.username, '') || ' ' || coalesce(old.username2, '') || ' ' || coalesce(old.username3, '')),
        coalesce(old.notes, '')
    );
END;

CREATE TRIGGER entries_fts_update BEFORE UPDATE ON entries BEGIN
    INSERT INTO entries_fts(entries_fts, rowid, title, url, username, notes)
    VALUES (
        'delete',
        old.rowid,
        old.title,
        coalesce(old.url, ''),
        trim(coalesce(old.username, '') || ' ' || coalesce(old.username2, '') || ' ' || coalesce(old.username3, '')),
        coalesce(old.notes, '')
    );
END;

CREATE TRIGGER entries_fts_update_after AFTER UPDATE ON entries BEGIN
    INSERT INTO entries_fts(rowid, title, url, username, notes)
    VALUES (
        new.rowid,
        new.title,
        coalesce(new.url, ''),
        trim(coalesce(new.username, '') || ' ' || coalesce(new.username2, '') || ' ' || coalesce(new.username3, '')),
        coalesce(new.notes, '')
    );
END;

INSERT INTO entries_fts(entries_fts) VALUES('rebuild');
