-- Migration 002: Additional login fields (Dashlane username2/username3)

ALTER TABLE entries ADD COLUMN username2 TEXT;
ALTER TABLE entries ADD COLUMN username3 TEXT;

UPDATE meta SET value = '2' WHERE key = 'version';
