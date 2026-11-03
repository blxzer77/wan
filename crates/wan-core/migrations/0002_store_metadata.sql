CREATE TABLE store_metadata (
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

UPDATE schema_version SET version = 2 WHERE id = 1;
