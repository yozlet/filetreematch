CREATE TABLE IF NOT EXISTS scans (
    id              INTEGER PRIMARY KEY,
    root_path       TEXT NOT NULL,
    started_at      TEXT NOT NULL,
    completed_at    TEXT,
    volume_id       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS directories (
    id              INTEGER PRIMARY KEY,
    parent_id       INTEGER REFERENCES directories(id),
    name            TEXT NOT NULL,
    full_path       TEXT NOT NULL UNIQUE,
    file_count      INTEGER NOT NULL DEFAULT 0,
    total_size      INTEGER NOT NULL DEFAULT 0,
    scan_fingerprint TEXT NOT NULL DEFAULT '',
    last_scanned_at TEXT,
    deleted         INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_directories_parent ON directories(parent_id);
CREATE INDEX IF NOT EXISTS idx_directories_file_count ON directories(file_count);

CREATE TABLE IF NOT EXISTS files (
    id              INTEGER PRIMARY KEY,
    directory_id    INTEGER NOT NULL REFERENCES directories(id),
    name            TEXT NOT NULL,
    size            INTEGER NOT NULL,
    mtime           INTEGER NOT NULL,
    relative_path   TEXT NOT NULL,
    name_raw        BLOB
);

CREATE INDEX IF NOT EXISTS idx_files_directory ON files(directory_id);

CREATE TABLE IF NOT EXISTS manifest_entries (
    directory_id    INTEGER NOT NULL REFERENCES directories(id),
    relative_path   TEXT NOT NULL,
    size            INTEGER NOT NULL,
    PRIMARY KEY (directory_id, relative_path)
);

CREATE INDEX IF NOT EXISTS idx_manifest_size ON manifest_entries(directory_id, size);

CREATE TABLE IF NOT EXISTS subset_pairs (
    id              INTEGER PRIMARY KEY,
    subset_dir_id   INTEGER NOT NULL REFERENCES directories(id),
    superset_dir_id INTEGER NOT NULL REFERENCES directories(id),
    file_count      INTEGER NOT NULL,
    total_size      INTEGER NOT NULL,
    is_maximal      INTEGER NOT NULL DEFAULT 0,
    UNIQUE(subset_dir_id, superset_dir_id)
);

CREATE TABLE IF NOT EXISTS scan_errors (
    id              INTEGER PRIMARY KEY,
    scan_id         INTEGER NOT NULL REFERENCES scans(id),
    path            TEXT NOT NULL,
    error_message   TEXT NOT NULL,
    occurred_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS annotations (
    directory_id    INTEGER PRIMARY KEY REFERENCES directories(id),
    status          TEXT NOT NULL DEFAULT 'undecided',
    notes           TEXT NOT NULL DEFAULT '',
    updated_at      TEXT NOT NULL
);
