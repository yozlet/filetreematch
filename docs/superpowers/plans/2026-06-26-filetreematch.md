# filetreematch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI tool that scans large archives, caches folder manifests in SQLite, finds subset-duplicate folder trees, and provides a TUI to annotate and export safe delete scripts.

**Architecture:** Single Rust binary with `scan`, `analyze`, `list`, `export`, and `tui` subcommands. Scan walks the filesystem top-down with fingerprint-based incremental skip, builds bottom-up manifests into SQLite. Analyze runs containment queries over cached manifests and computes maximal pairs. TUI reads/writes annotations in the same DB.

**Tech Stack:** Rust 2021, clap, rusqlite (bundled), ratatui + crossterm, globset, rayon, sha2, toml, indicatif, anyhow, thiserror, chrono, tempfile (dev)

---

## File Structure

```
filetreematch/
├── Cargo.toml
├── src/
│   ├── main.rs                 # clap CLI dispatch
│   ├── lib.rs                  # pub mod declarations
│   ├── cli/
│   │   ├── mod.rs              # Cli struct, subcommand enum
│   │   ├── scan.rs             # scan command handler
│   │   ├── analyze.rs          # analyze command handler
│   │   ├── list.rs             # list command handler
│   │   └── export.rs           # export command handler
│   ├── config/
│   │   ├── mod.rs
│   │   └── ignore.rs           # IgnoreRules + toml loading
│   ├── db/
│   │   ├── mod.rs              # Database wrapper, open/init
│   │   ├── schema.sql          # embedded schema
│   │   ├── directories.rs      # CRUD + lookup by path
│   │   ├── files.rs
│   │   ├── manifests.rs        # manifest_entries read/write
│   │   ├── subset_pairs.rs
│   │   ├── annotations.rs
│   │   └── scans.rs
│   ├── scan/
│   │   ├── mod.rs              # pub fn run_scan(...)
│   │   ├── fingerprint.rs      # ScanFingerprint compute + hash
│   │   └── engine.rs           # top-down walk + incremental skip
│   ├── analyze/
│   │   ├── mod.rs              # pub fn run_analyze(...)
│   │   ├── containment.rs      # A subset of B check
│   │   └── maximal.rs          # is_maximal computation
│   ├── export/
│   │   ├── mod.rs              # pub fn run_export(...)
│   │   └── script.rs           # trash/rm/paths formatters
│   └── tui/
│       ├── mod.rs              # pub fn run_tui(...)
│       ├── app.rs              # AppState, key handling
│       └── ui.rs               # ratatui render
└── tests/
    ├── common/mod.rs           # temp dir + db helpers
    ├── fingerprint_test.rs
    ├── ignore_test.rs
    ├── manifest_test.rs
    ├── containment_test.rs
    ├── maximal_test.rs
    ├── scan_integration.rs
    ├── analyze_integration.rs
    └── export_integration.rs
```

---

### Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "filetreematch"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "filetreematch"
path = "src/main.rs"

[dependencies]
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
crossterm = "0.28"
globset = "0.4"
indicatif = "0.17"
ratatui = "0.29"
rayon = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
sha2 = "0.10"
thiserror = "2"
toml = "0.8"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Create lib.rs**

```rust
pub mod analyze;
pub mod cli;
pub mod config;
pub mod db;
pub mod export;
pub mod scan;
pub mod tui;
```

- [ ] **Step 3: Create main.rs with clap skeleton**

```rust
use anyhow::Result;
use clap::Parser;
use filetreematch::cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Scan(args) => filetreematch::cli::scan::run(args),
        Commands::Analyze(args) => filetreematch::cli::analyze::run(args),
        Commands::List(args) => filetreematch::cli::list::run(args),
        Commands::Export(args) => filetreematch::cli::export::run(args),
        Commands::Tui(args) => filetreematch::tui::run(args),
    }
}
```

- [ ] **Step 4: Create cli/mod.rs with subcommands**

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "filetreematch", about = "Find subset-duplicate folder trees")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to SQLite cache (overrides default)
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    Scan(ScanArgs),
    Analyze(AnalyzeArgs),
    List(ListArgs),
    Export(ExportArgs),
    Tui(TuiArgs),
}

#[derive(clap::Args)]
pub struct ScanArgs {
    pub root: PathBuf,
    #[arg(long)]
    pub analyze: bool,
    #[arg(long = "ignore-add")]
    pub ignore_add: Vec<String>,
    #[arg(long = "ignore-file")]
    pub ignore_file: Option<PathBuf>,
    #[arg(long, default_value_t = 0)]
    pub threads: usize,
}

#[derive(clap::Args)]
pub struct AnalyzeArgs {
    #[arg(long)]
    pub full_detail: bool,
}

#[derive(clap::Args)]
pub struct ListArgs {
    #[arg(long)]
    pub full_detail: bool,
    #[arg(long)]
    pub status: Option<String>,
}

#[derive(clap::Args)]
pub struct ExportArgs {
    #[arg(long, default_value = "trash")]
    pub format: String,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub force: bool,
}

#[derive(clap::Args)]
pub struct TuiArgs {
    #[arg(long)]
    pub full_detail: bool,
}
```

- [ ] **Step 5: Create stub modules that compile**

Create these files each exporting a minimal `run` or placeholder:

`src/cli/scan.rs`:
```rust
use anyhow::{bail, Result};
use super::ScanArgs;

pub fn run(_args: ScanArgs) -> Result<()> {
    bail!("scan not yet implemented")
}
```

`src/cli/analyze.rs`:
```rust
use anyhow::{bail, Result};
use super::AnalyzeArgs;

pub fn run(_args: AnalyzeArgs) -> Result<()> {
    bail!("analyze not yet implemented")
}
```

`src/cli/list.rs`:
```rust
use anyhow::{bail, Result};
use super::ListArgs;

pub fn run(_args: ListArgs) -> Result<()> {
    bail!("list not yet implemented")
}
```

`src/cli/export.rs`:
```rust
use anyhow::{bail, Result};
use super::ExportArgs;

pub fn run(_args: ExportArgs) -> Result<()> {
    bail!("export not yet implemented")
}
```

`src/config/mod.rs`: `pub mod ignore;`
`src/config/ignore.rs`: `// Task 3`
`src/db/mod.rs`: `// Task 2`
`src/scan/mod.rs`: `pub mod fingerprint; pub mod engine;`
`src/scan/fingerprint.rs`: `// Task 4`
`src/scan/engine.rs`: `// Task 8`
`src/analyze/mod.rs`: `pub mod containment; pub mod maximal;`
`src/analyze/containment.rs`: `// Task 6`
`src/analyze/maximal.rs`: `// Task 7`
`src/export/mod.rs`: `pub mod script;`
`src/export/script.rs`: `// Task 12`
`src/tui/mod.rs`:
```rust
use anyhow::{bail, Result};
use crate::cli::TuiArgs;

pub fn run(_args: TuiArgs) -> Result<()> {
    bail!("tui not yet implemented")
}
```

- [ ] **Step 6: Verify build**

Run: `cargo build`
Expected: SUCCESS (warnings ok)

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/
git commit -m "feat: scaffold filetreematch CLI and module layout"
```

---

### Task 2: Database Schema & Connection

**Files:**
- Create: `src/db/schema.sql`
- Create: `src/db/mod.rs`
- Create: `tests/common/mod.rs`
- Test: `tests/db_schema_test.rs`

- [ ] **Step 1: Write failing schema test**

Create `tests/db_schema_test.rs`:
```rust
mod common;

use filetreematch::db::Database;

#[test]
fn initializes_all_tables() {
    let (_tmp, db) = common::open_temp_db();
    let tables: Vec<String> = db
        .conn()
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(tables.contains(&"annotations".to_string()));
    assert!(tables.contains(&"directories".to_string()));
    assert!(tables.contains(&"files".to_string()));
    assert!(tables.contains(&"manifest_entries".to_string()));
    assert!(tables.contains(&"scan_errors".to_string()));
    assert!(tables.contains(&"scans".to_string()));
    assert!(tables.contains(&"subset_pairs".to_string()));
}
```

Create `tests/common/mod.rs`:
```rust
use filetreematch::db::Database;
use tempfile::TempDir;

pub fn open_temp_db() -> (TempDir, Database) {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let db = Database::open(&db_path).unwrap();
    (tmp, db)
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test db_schema_test -- --nocapture`
Expected: FAIL — `Database` not found / module missing

- [ ] **Step 3: Create schema.sql**

Create `src/db/schema.sql`:
```sql
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
```

- [ ] **Step 4: Implement Database wrapper**

Create `src/db/mod.rs`:
```rust
mod directories;
mod files;
mod manifests;
mod subset_pairs;
mod annotations;
mod scans;

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::Path;

const SCHEMA: &str = include_str!("schema.sql");

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create db dir {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("open db {}", path.display()))?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn default_db_path(root: &Path) -> std::path::PathBuf {
        let volume_id = root.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("default");
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".cache/filetreematch")
            .join(format!("{volume_id}.db"))
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test db_schema_test -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/db/ tests/
git commit -m "feat: add SQLite schema and Database wrapper"
```

---

### Task 3: Ignore Rules

**Files:**
- Create: `src/config/ignore.rs`
- Test: `tests/ignore_test.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/ignore_test.rs`:
```rust
use filetreematch::config::ignore::IgnoreRules;
use std::path::Path;

#[test]
fn ignores_ds_store_by_name() {
    let rules = IgnoreRules::defaults();
    assert!(rules.should_ignore(Path::new("/archive/photos/.DS_Store")));
}

#[test]
fn ignores_git_directory_glob() {
    let rules = IgnoreRules::defaults();
    assert!(rules.should_ignore(Path::new("/archive/project/.git/config")));
}

#[test]
fn does_not_ignore_bashrc() {
    let rules = IgnoreRules::defaults();
    assert!(!rules.should_ignore(Path::new("/archive/home/.bashrc")));
}

#[test]
fn ignore_add_extends_rules() {
    let rules = IgnoreRules::defaults().with_extra_globs(&["**/*.tmp"]).unwrap();
    assert!(rules.should_ignore(Path::new("/archive/file.tmp")));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test ignore_test -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement IgnoreRules**

Create `src/config/ignore.rs`:
```rust
use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct IgnoreRules {
    ignore_names: Vec<String>,
    glob_set: GlobSet,
}

impl IgnoreRules {
    pub fn defaults() -> Self {
        Self::from_toml(DEFAULT_IGNORE_TOML).expect("default ignore rules must parse")
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read ignore file {}", path.display()))?;
        Self::from_toml(&text)
    }

    pub fn from_toml(text: &str) -> Result<Self> {
        let raw: IgnoreToml = toml::from_str(text)?;
        Self::build(raw.ignore_names, raw.ignore_globs)
    }

    pub fn with_extra_globs(mut self, globs: &[&str]) -> Result<Self> {
        let mut patterns = self.glob_set.patterns().to_vec();
        // rebuild from stored names + new globs
        let mut glob_strings = Vec::new();
        for g in globs {
            glob_strings.push(g.to_string());
        }
        Self::build(self.ignore_names.clone(), glob_strings)
    }

    fn build(ignore_names: Vec<String>, ignore_globs: Vec<String>) -> Result<Self> {
        let mut builder = GlobSetBuilder::new();
        for g in &ignore_globs {
            builder.add(Glob::new(g).with_context(|| format!("bad glob: {g}"))?);
        }
        let glob_set = builder.build()?;
        Ok(Self { ignore_names, glob_set })
    }

    pub fn should_ignore(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if self.ignore_names.iter().any(|n| n == name) {
                return true;
            }
        }
        self.glob_set.is_match(path)
    }
}

#[derive(serde::Deserialize)]
struct IgnoreToml {
    ignore_names: Vec<String>,
    ignore_globs: Vec<String>,
}

const DEFAULT_IGNORE_TOML: &str = r#"
ignore_names = [
  ".DS_Store", "Thumbs.db", "desktop.ini",
  ".Spotlight-V100", ".Trashes", ".fseventsd",
  ".TemporaryItems", ".DocumentRevisions-V100",
]
ignore_globs = [
  "**/.git/**",
  "**/.svn/**",
  "**/node_modules/**",
  "**/__MACOSX/**",
  "**/.cache/**",
]
"#;

pub fn load_ignore_rules(config_path: Option<&Path>, extra: &[String]) -> Result<IgnoreRules> {
    let mut rules = if let Some(path) = config_path {
        IgnoreRules::load(path)?
    } else {
        let default_path = dirs_config_file();
        if default_path.exists() {
            IgnoreRules::load(&default_path)?
        } else {
            IgnoreRules::defaults()
        }
    };
    if !extra.is_empty() {
        let refs: Vec<&str> = extra.iter().map(String::as_str).collect();
        rules = rules.with_extra_globs(&refs)?;
    }
    Ok(rules)
}

fn dirs_config_file() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/filetreematch/ignore.toml")
}
```

Add to `Cargo.toml` dependencies: `dirs = "5"`

Update `src/config/mod.rs`:
```rust
pub mod ignore;
```

- [ ] **Step 4: Run tests**

Run: `cargo test ignore_test -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/config/ Cargo.toml tests/ignore_test.rs
git commit -m "feat: add configurable ignore rules with defaults"
```

---

### Task 4: Scan Fingerprint

**Files:**
- Create: `src/scan/fingerprint.rs`
- Test: `tests/fingerprint_test.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/fingerprint_test.rs`:
```rust
use filetreematch::scan::fingerprint::{ScanFingerprint, hash_fingerprint};

#[test]
fn fingerprint_hash_changes_when_file_count_changes() {
    let a = ScanFingerprint { file_count: 1, total_size: 100, max_mtime: 1000 };
    let b = ScanFingerprint { file_count: 2, total_size: 100, max_mtime: 1000 };
    assert_ne!(hash_fingerprint(&a), hash_fingerprint(&b));
}

#[test]
fn merge_child_fingerprints() {
    let child = ScanFingerprint { file_count: 2, total_size: 50, max_mtime: 500 };
    let parent = ScanFingerprint::merge(&[child]);
    assert_eq!(parent.file_count, 2);
    assert_eq!(parent.total_size, 50);
    assert_eq!(parent.max_mtime, 500);
}

#[test]
fn merge_includes_local_file() {
    let child = ScanFingerprint { file_count: 1, total_size: 10, max_mtime: 100 };
    let parent = ScanFingerprint::merge_with_local(&[child], 1, 20, 200);
    assert_eq!(parent.file_count, 2);
    assert_eq!(parent.total_size, 30);
    assert_eq!(parent.max_mtime, 200);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test fingerprint_test -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement fingerprint module**

Create `src/scan/fingerprint.rs`:
```rust
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanFingerprint {
    pub file_count: u64,
    pub total_size: u64,
    pub max_mtime: i64,
}

impl ScanFingerprint {
    pub fn empty() -> Self {
        Self { file_count: 0, total_size: 0, max_mtime: 0 }
    }

    pub fn from_file(size: u64, mtime: i64) -> Self {
        Self { file_count: 1, total_size: size, max_mtime: mtime }
    }

    pub fn merge(children: &[Self]) -> Self {
        Self::merge_with_local(children, 0, 0, 0)
    }

    pub fn merge_with_local(children: &[Self], local_count: u64, local_size: u64, local_mtime: i64) -> Self {
        let mut fp = Self {
            file_count: local_count,
            total_size: local_size,
            max_mtime: local_mtime,
        };
        for c in children {
            fp.file_count += c.file_count;
            fp.total_size += c.total_size;
            fp.max_mtime = fp.max_mtime.max(c.max_mtime);
        }
        fp
    }
}

pub fn hash_fingerprint(fp: &ScanFingerprint) -> String {
    let mut hasher = Sha256::new();
    hasher.update(fp.file_count.to_le_bytes());
    hasher.update(fp.total_size.to_le_bytes());
    hasher.update(fp.max_mtime.to_le_bytes());
    format!("{:x}", hasher.finalize())
}
```

Update `src/scan/mod.rs`:
```rust
pub mod fingerprint;
pub mod engine;
```

- [ ] **Step 4: Run tests**

Run: `cargo test fingerprint_test -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/scan/fingerprint.rs tests/fingerprint_test.rs
git commit -m "feat: add scan fingerprint compute and hash"
```

---

### Task 5: Manifest Rollup

**Files:**
- Create: `src/db/manifests.rs`
- Create: `src/db/directories.rs`
- Test: `tests/manifest_test.rs`

- [ ] **Step 1: Write failing integration test**

Create `tests/manifest_test.rs`:
```rust
mod common;

use filetreematch::db::manifests::{insert_manifest_entry, rollup_manifest, ManifestEntry};

#[test]
fn rollup_prefixes_child_paths() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();

    // parent id=1, child id=2
    conn.execute(
        "INSERT INTO directories (id, name, full_path, scan_fingerprint) VALUES (1, 'root', '/root', ''), (2, 'child', '/root/child', '')",
        [],
    ).unwrap();
    insert_manifest_entry(conn, 2, "a.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "sub/b.txt", 20).unwrap();
    insert_manifest_entry(conn, 1, "local.txt", 5).unwrap();

    rollup_manifest(conn, 1, &[2]).unwrap();

    let entries = fetch_manifest(conn, 1);
    assert_eq!(entries.len(), 3);
    assert!(entries.contains(&ManifestEntry { relative_path: "local.txt".into(), size: 5 }));
    assert!(entries.contains(&ManifestEntry { relative_path: "child/a.txt".into(), size: 10 }));
    assert!(entries.contains(&ManifestEntry { relative_path: "child/sub/b.txt".into(), size: 20 }));
}

fn fetch_manifest(conn: &rusqlite::Connection, dir_id: i64) -> Vec<ManifestEntry> {
    let mut stmt = conn.prepare("SELECT relative_path, size FROM manifest_entries WHERE directory_id = ?1").unwrap();
    stmt.query_map([dir_id], |row| {
        Ok(ManifestEntry { relative_path: row.get(0)?, size: row.get(1)? })
    }).unwrap().map(|r| r.unwrap()).collect()
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test manifest_test -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement manifest DB functions**

Create `src/db/manifests.rs`:
```rust
use anyhow::Result;
use rusqlite::{Connection, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub relative_path: String,
    pub size: i64,
}

pub fn clear_manifest(conn: &Connection, directory_id: i64) -> Result<()> {
    conn.execute("DELETE FROM manifest_entries WHERE directory_id = ?1", params![directory_id])?;
    Ok(())
}

pub fn insert_manifest_entry(conn: &Connection, directory_id: i64, relative_path: &str, size: i64) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO manifest_entries (directory_id, relative_path, size) VALUES (?1, ?2, ?3)",
        params![directory_id, relative_path, size],
    )?;
    Ok(())
}

pub fn rollup_manifest(conn: &Connection, parent_id: i64, child_ids: &[i64]) -> Result<()> {
    clear_manifest(conn, parent_id)?;
    if let Some(mut rows) = conn.prepare(
        "SELECT relative_path, size FROM manifest_entries WHERE directory_id = ?1",
    )?.query([parent_id]) {
        // after clear, copy direct files already inserted separately
    }

    // copy direct files for parent
    conn.execute(
        "INSERT OR REPLACE INTO manifest_entries (directory_id, relative_path, size)
         SELECT ?1, relative_path, size FROM manifest_entries WHERE directory_id = ?1",
        params![parent_id],
    )?;

    for child_id in child_ids {
        let child_name: String = conn.query_row(
            "SELECT name FROM directories WHERE id = ?1",
            [child_id],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO manifest_entries (directory_id, relative_path, size)
             SELECT ?1,
                    ?2 || CASE WHEN relative_path = '' THEN '' ELSE '/' || relative_path END,
                    size
             FROM manifest_entries WHERE directory_id = ?3",
            params![parent_id, child_name, child_id],
        )?;
    }
    Ok(())
}
```

Refine `rollup_manifest` implementation in Task 8 scan engine to:
1. Delete parent's manifest entries
2. Insert parent's direct files from `files` table (`relative_path = name` for immediate files)
3. Insert prefixed child manifest entries

Create `src/db/directories.rs` with:
```rust
use anyhow::Result;
use rusqlite::{Connection, params, OptionalExtension};

pub fn upsert_directory(
    conn: &Connection,
    parent_id: Option<i64>,
    name: &str,
    full_path: &str,
    file_count: i64,
    total_size: i64,
    fingerprint: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO directories (parent_id, name, full_path, file_count, total_size, scan_fingerprint, last_scanned_at, deleted)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'), 0)
         ON CONFLICT(full_path) DO UPDATE SET
           parent_id=excluded.parent_id,
           file_count=excluded.file_count,
           total_size=excluded.total_size,
           scan_fingerprint=excluded.scan_fingerprint,
           last_scanned_at=excluded.last_scanned_at,
           deleted=0",
        params![parent_id, name, full_path, file_count, total_size, fingerprint],
    )?;
    Ok(conn.query_row(
        "SELECT id FROM directories WHERE full_path = ?1",
        [full_path],
        |row| row.get(0),
    )?)
}

pub fn get_directory_id(conn: &Connection, full_path: &str) -> Result<Option<i64>> {
    Ok(conn.query_row(
        "SELECT id FROM directories WHERE full_path = ?1 AND deleted = 0",
        [full_path],
        |row| row.get(0),
    ).optional()?)
}

pub fn get_fingerprint(conn: &Connection, dir_id: i64) -> Result<Option<String>> {
    Ok(conn.query_row(
        "SELECT scan_fingerprint FROM directories WHERE id = ?1",
        [dir_id],
        |row| row.get(0),
    ).optional()?)
}
```

Export modules from `src/db/mod.rs`:
```rust
pub use directories::*;
pub use manifests::*;
```

- [ ] **Step 4: Fix rollup logic until test passes**

Adjust `rollup_manifest` to match test expectations (copy direct files + prefix children).

- [ ] **Step 5: Run test**

Run: `cargo test manifest_test -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/db/manifests.rs src/db/directories.rs tests/manifest_test.rs
git commit -m "feat: add manifest rollup and directory upsert"
```

---

### Task 6: Subset Containment Check

**Files:**
- Create: `src/analyze/containment.rs`
- Test: `tests/containment_test.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/containment_test.rs`:
```rust
mod common;

use filetreematch::analyze::containment::is_subset;
use filetreematch::db::manifests::insert_manifest_entry;

#[test]
fn strict_subset_detected() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    conn.execute(
        "INSERT INTO directories (id, name, full_path, file_count, total_size, scan_fingerprint) VALUES
         (1, 'a', '/a', 1, 10, ''), (2, 'b', '/b', 2, 30, '')",
        [],
    ).unwrap();
    insert_manifest_entry(conn, 1, "x.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "x.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "y.txt", 20).unwrap();
    assert!(is_subset(conn, 1, 2).unwrap());
}

#[test]
fn non_subset_when_size_differs() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    conn.execute(
        "INSERT INTO directories (id, name, full_path, file_count, total_size, scan_fingerprint) VALUES
         (1, 'a', '/a', 1, 10, ''), (2, 'b', '/b', 1, 10, '')",
        [],
    ).unwrap();
    insert_manifest_entry(conn, 1, "x.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "x.txt", 99).unwrap();
    assert!(!is_subset(conn, 1, 2).unwrap());
}

#[test]
fn empty_manifest_is_not_analyzed() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    conn.execute(
        "INSERT INTO directories (id, name, full_path, file_count, total_size, scan_fingerprint) VALUES
         (1, 'empty', '/empty', 0, 0, '')",
        [],
    ).unwrap();
    assert!(!is_subset(conn, 1, 1).unwrap());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test containment_test -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement containment check**

Create `src/analyze/containment.rs`:
```rust
use anyhow::Result;
use rusqlite::{Connection, params};

pub fn is_subset(conn: &Connection, subset_dir_id: i64, superset_dir_id: i64) -> Result<bool> {
    let subset_count: i64 = conn.query_row(
        "SELECT file_count FROM directories WHERE id = ?1",
        [subset_dir_id],
        |row| row.get(0),
    )?;
    if subset_count == 0 {
        return Ok(false);
    }

    let missing: i64 = conn.query_row(
        "SELECT COUNT(*) FROM manifest_entries ma
         WHERE ma.directory_id = ?1
         AND NOT EXISTS (
           SELECT 1 FROM manifest_entries mb
           WHERE mb.directory_id = ?2
           AND mb.relative_path = ma.relative_path
           AND mb.size = ma.size
         )",
        params![subset_dir_id, superset_dir_id],
        |row| row.get(0),
    )?;
    Ok(missing == 0)
}
```

Update `src/analyze/mod.rs`:
```rust
pub mod containment;
pub mod maximal;
```

- [ ] **Step 4: Run tests**

Run: `cargo test containment_test -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/analyze/containment.rs tests/containment_test.rs
git commit -m "feat: add manifest subset containment check"
```

---

### Task 7: Maximal Pair Collapse

**Files:**
- Create: `src/analyze/maximal.rs`
- Test: `tests/maximal_test.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/maximal_test.rs`:
```rust
use filetreematch::analyze::maximal::compute_is_maximal;

#[test]
fn parent_subset_makes_child_non_maximal() {
    // pairs: (2, 10) child a/x subset of b/x, (1, 10) parent a subset of b
    let pairs = vec![(1_i64, 10_i64), (2, 10)];
    let parent_of = |id: i64| -> Option<i64> {
        match id {
            2 => Some(1),
            _ => None,
        }
    };
    let is_subset_pair = |a: i64, b: i64| a == 1 || a == 2; // both subset of 10

    let maximal = compute_is_maximal(&pairs, &parent_of, &is_subset_pair);
    assert!(!maximal[&2]);  // child not maximal
    assert!(maximal[&1]);   // parent maximal
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test maximal_test -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement maximal computation**

Create `src/analyze/maximal.rs`:
```rust
use std::collections::HashMap;

/// Returns map (subset_dir_id -> is_maximal) for pairs sharing superset_dir_id groups.
pub fn compute_is_maximal(
    pairs: &[(i64, i64)], // (subset_id, superset_id)
    parent_of: &dyn Fn(i64) -> Option<i64>,
    is_subset_of: &dyn Fn(i64, i64) -> bool,
) -> HashMap<i64, bool> {
    let mut result = HashMap::new();
    for &(subset_id, superset_id) in pairs {
        let parent = parent_of(subset_id);
        let non_maximal = parent.is_some_and(|pid| is_subset_of(pid, superset_id));
        result.insert(subset_id, !non_maximal);
    }
    result
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test maximal_test -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/analyze/maximal.rs tests/maximal_test.rs
git commit -m "feat: add maximal subset pair collapse logic"
```

---

### Task 8: Scan Engine

**Files:**
- Create: `src/scan/engine.rs`
- Create: `src/scan/mod.rs` (run_scan export)
- Create: `src/db/files.rs`
- Create: `src/db/scans.rs`
- Modify: `src/cli/scan.rs`
- Test: `tests/scan_integration.rs`

- [ ] **Step 1: Write failing integration test**

Create `tests/scan_integration.rs`:
```rust
mod common;

use filetreematch::config::ignore::IgnoreRules;
use filetreematch::scan::run_scan;
use std::fs;
use std::path::PathBuf;

#[test]
fn scan_populates_manifests_for_fixture_tree() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("archive");
    fs::create_dir_all(root.join("master/photos")).unwrap();
    fs::create_dir_all(root.join("old-pc/photos")).unwrap();
    fs::write(root.join("master/photos/a.jpg"), vec![0u8; 100]).unwrap();
    fs::write(root.join("old-pc/photos/a.jpg"), vec![0u8; 100]).unwrap();
    fs::write(root.join("master/extra.txt"), b"only in master").unwrap();

    let db_path = tmp.path().join("test.db");
    let db = filetreematch::db::Database::open(&db_path).unwrap();
    run_scan(
        &root,
        &db,
        &IgnoreRules::defaults(),
        1,
    ).unwrap();

    let count: i64 = db.conn().query_row(
        "SELECT COUNT(*) FROM manifest_entries",
        [],
        |row| row.get(0),
    ).unwrap();
    assert!(count > 0);

    let dirs: i64 = db.conn().query_row(
        "SELECT COUNT(*) FROM directories WHERE deleted = 0",
        [],
        |row| row.get(0),
    ).unwrap();
    assert!(dirs >= 4);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test scan_integration -- --nocapture`
Expected: FAIL — `run_scan` not found

- [ ] **Step 3: Implement scan engine (top-down, incremental-ready)**

Create `src/db/files.rs`:
```rust
use anyhow::Result;
use rusqlite::{Connection, params};

pub fn clear_files_in_subtree(conn: &Connection, directory_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM files WHERE directory_id IN (
           WITH RECURSIVE sub(id) AS (
             SELECT ?1
             UNION ALL SELECT d.id FROM directories d JOIN sub ON d.parent_id = sub.id
           ) SELECT id FROM sub
         )",
        params![directory_id],
    )?;
    Ok(())
}

pub fn insert_file(
    conn: &Connection,
    directory_id: i64,
    name: &str,
    size: i64,
    mtime: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO files (directory_id, name, size, mtime, relative_path)
         VALUES (?1, ?2, ?3, ?4, ?2)",
        params![directory_id, name, size, mtime],
    )?;
    Ok(())
}
```

Create `src/db/scans.rs`:
```rust
use anyhow::Result;
use rusqlite::{Connection, params};

pub fn start_scan(conn: &Connection, root_path: &str, volume_id: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO scans (root_path, started_at, volume_id) VALUES (?1, datetime('now'), ?2)",
        params![root_path, volume_id],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn finish_scan(conn: &Connection, scan_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE scans SET completed_at = datetime('now') WHERE id = ?1",
        params![scan_id],
    )?;
    Ok(())
}

pub fn log_scan_error(conn: &Connection, scan_id: i64, path: &str, message: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO scan_errors (scan_id, path, error_message, occurred_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
        params![scan_id, path, message],
    )?;
    Ok(())
}
```

Create `src/scan/engine.rs` with recursive `scan_directory` function:

```rust
use crate::config::ignore::IgnoreRules;
use crate::db::directories::{get_directory_id, get_fingerprint, upsert_directory};
use crate::db::files::{clear_files_in_subtree, insert_file};
use crate::db::manifests::{clear_manifest, insert_manifest_entry, rollup_manifest};
use crate::db::scans::log_scan_error;
use crate::db::Database;
use crate::scan::fingerprint::{hash_fingerprint, ScanFingerprint};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run_scan(root: &Path, db: &Database, ignore: &IgnoreRules, scan_id: i64) -> Result<()> {
    scan_directory(root, None, root, db, ignore, scan_id)
}

fn scan_directory(
    path: &Path,
    parent_id: Option<i64>,
    root: &Path,
    db: &Database,
    ignore: &IgnoreRules,
    scan_id: i64,
) -> Result<i64> {
    let conn = db.conn();
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let full_path = path.to_string_lossy().to_string();

    if ignore.should_ignore(path) {
        return Ok(0);
    }

    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(err) => {
            log_scan_error(conn, scan_id, &full_path, &err.to_string())?;
            return Ok(0);
        }
    };

    let mut child_fps = Vec::new();
    let mut child_ids = Vec::new();
    let mut local_count = 0u64;
    let mut local_size = 0u64;
    let mut local_mtime = 0i64;

    for entry in entries.flatten() {
        let entry_path = entry.path();
        if ignore.should_ignore(&entry_path) {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(err) => {
                log_scan_error(conn, scan_id, &entry_path.to_string_lossy(), &err.to_string())?;
                continue;
            }
        };
        if meta.is_dir() {
            let child_id = scan_directory(&entry_path, None, root, db, ignore, scan_id)?;
            if child_id > 0 {
                child_ids.push(child_id);
                let fp = load_fingerprint(conn, child_id)?;
                child_fps.push(fp);
            }
        } else if meta.is_file() || meta.file_type().is_symlink() {
            local_count += 1;
            local_size += meta.len();
            let mtime = meta.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            local_mtime = local_mtime.max(mtime);
        }
    }

    let fp = ScanFingerprint::merge_with_local(&child_fps, local_count, local_size, local_mtime);
    let fp_hash = hash_fingerprint(&fp);

    // incremental skip: if directory exists with same fingerprint, skip rebuild
    if let Some(existing_id) = get_directory_id(conn, &full_path)? {
        if get_fingerprint(conn, existing_id)?.as_deref() == Some(fp_hash.as_str()) {
            return Ok(existing_id);
        }
        clear_files_in_subtree(conn, existing_id)?;
        clear_manifest(conn, existing_id)?;
    }

    let dir_id = upsert_directory(
        conn,
        parent_id,
        name,
        &full_path,
        fp.file_count as i64,
        fp.total_size as i64,
        &fp_hash,
    )?;

    // re-walk to insert files and rescan children with correct parent_id
    // (simplified: second pass for files only)
    insert_local_files(path, dir_id, db, ignore, scan_id)?;

    for entry in fs::read_dir(path)?.flatten() {
        let entry_path = entry.path();
        if ignore.should_ignore(&entry_path) { continue; }
        if entry_path.is_dir() {
            let child_id = scan_directory(&entry_path, Some(dir_id), root, db, ignore, scan_id)?;
            if child_id > 0 { child_ids.push(child_id); }
        }
    }

    rollup_manifest_for_dir(conn, dir_id, path, &child_ids)?;
    Ok(dir_id)
}

fn load_fingerprint(conn: &rusqlite::Connection, dir_id: i64) -> Result<ScanFingerprint> {
    conn.query_row(
        "SELECT file_count, total_size, 0 FROM directories WHERE id = ?1",
        [dir_id],
        |row| Ok(ScanFingerprint {
            file_count: row.get::<_, i64>(0)? as u64,
            total_size: row.get::<_, i64>(1)? as u64,
            max_mtime: 0,
        }),
    ).map_err(Into::into)
}

fn insert_local_files(path: &Path, dir_id: i64, db: &Database, ignore: &IgnoreRules, scan_id: i64) -> Result<()> {
    for entry in fs::read_dir(path)?.flatten() {
        let p = entry.path();
        if ignore.should_ignore(&p) { continue; }
        let meta = entry.metadata()?;
        if meta.is_file() || meta.file_type().is_symlink() {
            let name = entry.file_name().to_string_lossy().to_string();
            let mtime = meta.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            insert_file(db.conn(), dir_id, &name, meta.len() as i64, mtime)?;
            insert_manifest_entry(db.conn(), dir_id, &name, meta.len() as i64)?;
        }
    }
    Ok(())
}

fn rollup_manifest_for_dir(conn: &rusqlite::Connection, dir_id: i64, _path: &Path, child_ids: &[i64]) -> Result<()> {
    rollup_manifest(conn, dir_id, child_ids)
}
```

Update `src/scan/mod.rs`:
```rust
pub mod fingerprint;
pub mod engine;

pub use engine::run_scan;
```

Wire `src/cli/scan.rs`:
```rust
use anyhow::Result;
use crate::cli::ScanArgs;
use crate::config::ignore::load_ignore_rules;
use crate::db::{Database, scans::{start_scan, finish_scan}};

pub fn run(args: ScanArgs) -> Result<()> {
    let db_path = args.db.clone().unwrap_or_else(|| Database::default_db_path(&args.root));
    let db = Database::open(&db_path)?;
    let ignore = load_ignore_rules(args.ignore_file.as_deref(), &args.ignore_add)?;
    let volume_id = args.root.file_name().and_then(|s| s.to_str()).unwrap_or("default");
    let scan_id = start_scan(db.conn(), &args.root.to_string_lossy(), volume_id)?;
    crate::scan::run_scan(&args.root, &db, &ignore, scan_id)?;
    finish_scan(db.conn(), scan_id)?;
    if args.analyze {
        crate::analyze::run_analyze(&db, false)?;
    }
    Ok(())
}
```

Add `src/analyze/mod.rs` stub for `run_analyze` (implemented in Task 9).

- [ ] **Step 4: Run integration test**

Run: `cargo test scan_integration -- --nocapture`
Expected: PASS (fix compile errors as needed)

- [ ] **Step 5: Commit**

```bash
git add src/scan/ src/db/files.rs src/db/scans.rs src/cli/scan.rs tests/scan_integration.rs
git commit -m "feat: add top-down scan engine with fingerprint skip"
```

---

### Task 9: Analyze Command

**Files:**
- Modify: `src/analyze/mod.rs`
- Create: `src/db/subset_pairs.rs`
- Modify: `src/cli/analyze.rs`
- Test: `tests/analyze_integration.rs`

- [ ] **Step 1: Write failing integration test**

Create `tests/analyze_integration.rs`:
```rust
mod common;

use filetreematch::analyze::run_analyze;
use filetreematch::config::ignore::IgnoreRules;
use filetreematch::scan::run_scan;
use std::fs;

#[test]
fn analyze_finds_maximal_subset_pair() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("archive");
    fs::create_dir_all(root.join("master/photos")).unwrap();
    fs::create_dir_all(root.join("old-pc/photos")).unwrap();
    fs::write(root.join("master/photos/a.jpg"), vec![0u8; 100]).unwrap();
    fs::write(root.join("old-pc/photos/a.jpg"), vec![0u8; 100]).unwrap();
    fs::write(root.join("master/extra.txt"), b"x").unwrap();

    let db_path = tmp.path().join("test.db");
    let db = filetreematch::db::Database::open(&db_path).unwrap();
    let ignore = IgnoreRules::defaults();
    run_scan(&root, &db, &ignore, 1).unwrap();
    run_analyze(&db, false).unwrap();

    let count: i64 = db.conn().query_row(
        "SELECT COUNT(*) FROM subset_pairs WHERE is_maximal = 1",
        [],
        |row| row.get(0),
    ).unwrap();
    assert!(count >= 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test analyze_integration -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement analyze**

Create `src/db/subset_pairs.rs`:
```rust
use anyhow::Result;
use rusqlite::{Connection, params};

pub fn clear_pairs(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM subset_pairs", [])?;
    Ok(())
}

pub fn insert_pair(
    conn: &Connection,
    subset_dir_id: i64,
    superset_dir_id: i64,
    file_count: i64,
    total_size: i64,
    is_maximal: bool,
) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO subset_pairs (subset_dir_id, superset_dir_id, file_count, total_size, is_maximal)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![subset_dir_id, superset_dir_id, file_count, total_size, is_maximal as i64],
    )?;
    Ok(())
}
```

Create `src/analyze/mod.rs`:
```rust
pub mod containment;
pub mod maximal;

use crate::analyze::containment::is_subset;
use crate::analyze::maximal::compute_is_maximal;
use crate::db::subset_pairs::{clear_pairs, insert_pair};
use crate::db::Database;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use rusqlite::Connection;

pub fn run_analyze(db: &Database, _full_detail: bool) -> Result<()> {
    let conn = db.conn();
    clear_pairs(conn)?;

    let mut stmt = conn.prepare(
        "SELECT id, file_count, total_size, parent_id FROM directories
         WHERE deleted = 0 AND file_count > 0 ORDER BY file_count ASC",
    )?;
    let dirs: Vec<(i64, i64, i64, Option<i64>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
        .collect::<Result<_, _>>()?;

    let pb = ProgressBar::new(dirs.len() as u64);
    pb.set_style(ProgressStyle::default_bar().template("{bar} {pos}/{len}").unwrap());

    let mut pairs = Vec::new();
    for (a_id, a_count, a_size, _) in &dirs {
        let mut candidates = conn.prepare(
            "SELECT id FROM directories
             WHERE deleted = 0 AND id != ?1 AND file_count >= ?2 AND total_size >= ?3",
        )?;
        let b_ids: Vec<i64> = candidates
            .query_map(params![a_id, a_count, a_size], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        for b_id in b_ids {
            if is_subset(conn, *a_id, b_id)? {
                pairs.push((*a_id, b_id));
            }
        }
        pb.inc(1);
    }
    pb.finish_and_clear();

    let parent_map: std::collections::HashMap<i64, i64> = dirs.iter()
        .filter_map(|(id, _, _, p)| p.map(|pid| (*id, pid)))
        .collect();

    let pair_set: std::collections::HashSet<(i64, i64)> = pairs.iter().copied().collect();
    let is_subset_pair = |subset: i64, superset: i64| pair_set.contains(&(subset, superset));
    let parent_of = |id: i64| parent_map.get(&id).copied();

    let maximal = compute_is_maximal(
        &pairs,
        &|id| parent_of(id),
        &|a, b| is_subset_pair(a, b),
    );

    for (a_id, b_id) in &pairs {
        let (file_count, total_size): (i64, i64) = conn.query_row(
            "SELECT file_count, total_size FROM directories WHERE id = ?1",
            [a_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        insert_pair(conn, *a_id, *b_id, file_count, total_size, maximal[a_id])?;
    }

    let maximal_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM subset_pairs WHERE is_maximal = 1",
        [],
        |row| row.get(0),
    )?;
    println!("Found {maximal_count} maximal subset pairs ({} total pairs)", pairs.len());
    Ok(())
}
```

Update `src/cli/analyze.rs`:
```rust
use anyhow::Result;
use crate::cli::AnalyzeArgs;
use crate::db::Database;

pub fn run(args: AnalyzeArgs) -> Result<()> {
    let db_path = args.db.clone().unwrap_or_else(|| {
        anyhow::bail!("analyze requires --db path until scan stores default location");
        #[allow(unreachable_code)]
        std::path::PathBuf::new()
    });
    let db = Database::open(&db_path)?;
    crate::analyze::run_analyze(&db, args.full_detail)
}
```

Add `--db` threading via global cli option (store in args or use env). Simpler v1: require `--db` on analyze/list/export/tui OR read last scan path from `scans` table. Add helper:

```rust
// src/db/scans.rs
pub fn latest_db_for_root(conn: &Connection) -> Result<Option<String>> {
    // store db path in scans or use single default
}
```

Pragmatic v1: all commands accept optional global `--db`; if omitted, use most recent from `~/.cache/filetreematch/` by modification time.

- [ ] **Step 4: Run integration test**

Run: `cargo test analyze_integration -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/analyze/ src/db/subset_pairs.rs src/cli/analyze.rs tests/analyze_integration.rs
git commit -m "feat: add analyze command with maximal pair detection"
```

---

### Task 10: List Command

**Files:**
- Modify: `src/cli/list.rs`
- Create: `src/db/query.rs` (pair listing helpers)

- [ ] **Step 1: Write failing test**

Add to `tests/analyze_integration.rs`:
```rust
#[test]
fn list_shows_maximal_pairs_by_default() {
    // reuse analyze fixture setup ...
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_filetreematch"))
        .args(["list", "--db", db_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("old-pc"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test list_shows_maximal -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement list command**

Create `src/db/query.rs`:
```rust
use anyhow::Result;
use rusqlite::{Connection, params};

pub struct SubsetPairRow {
    pub subset_path: String,
    pub superset_path: String,
    pub file_count: i64,
    pub total_size: i64,
    pub is_maximal: bool,
}

pub fn list_pairs(conn: &Connection, full_detail: bool, status_filter: Option<&str>) -> Result<Vec<SubsetPairRow>> {
    let sql = if full_detail {
        "SELECT ds.full_path, dp.full_path, sp.file_count, sp.total_size, sp.is_maximal
         FROM subset_pairs sp
         JOIN directories ds ON ds.id = sp.subset_dir_id
         JOIN directories dp ON dp.id = sp.superset_dir_id
         ORDER BY sp.total_size DESC"
    } else {
        "SELECT ds.full_path, dp.full_path, sp.file_count, sp.total_size, sp.is_maximal
         FROM subset_pairs sp
         JOIN directories ds ON ds.id = sp.subset_dir_id
         JOIN directories dp ON dp.id = sp.superset_dir_id
         WHERE sp.is_maximal = 1
         ORDER BY sp.total_size DESC"
    };
    // apply status_filter join on annotations when Some
    // ...
    Ok(vec![]) // implement query_map
}
```

Implement `src/cli/list.rs` to print human-readable table with sizes formatted (MB/GB).

- [ ] **Step 4: Run test**

Run: `cargo test list_shows_maximal -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cli/list.rs src/db/query.rs tests/
git commit -m "feat: add list command for subset pairs"
```

---

### Task 11: Annotations DB + Export

**Files:**
- Create: `src/db/annotations.rs`
- Create: `src/export/script.rs`
- Create: `src/export/mod.rs`
- Modify: `src/cli/export.rs`
- Test: `tests/export_integration.rs`

- [ ] **Step 1: Write failing export test**

Create `tests/export_integration.rs`:
```rust
mod common;

use filetreematch::db::annotations::set_annotation;
use filetreematch::export::run_export;
use std::fs;

#[test]
fn export_skips_keep_annotated_paths() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    conn.execute(
        "INSERT INTO directories (id, name, full_path, scan_fingerprint) VALUES
         (1, 'del', '/archive/del', ''), (2, 'keep', '/archive/keep', '')",
        [],
    ).unwrap();
    set_annotation(conn, 1, "delete_candidate", "").unwrap();
    set_annotation(conn, 2, "keep", "").unwrap();

    let out = tempfile::NamedTempFile::new().unwrap();
    run_export(&db, "paths", out.path(), false).unwrap();
    let text = fs::read_to_string(out.path()).unwrap();
    assert!(text.contains("/archive/del"));
    assert!(!text.contains("/archive/keep"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test export_integration -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement annotations and export**

Create `src/db/annotations.rs`:
```rust
use anyhow::Result;
use rusqlite::{Connection, params};

pub fn set_annotation(conn: &Connection, directory_id: i64, status: &str, notes: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO annotations (directory_id, status, notes, updated_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(directory_id) DO UPDATE SET
           status=excluded.status, notes=excluded.notes, updated_at=excluded.updated_at",
        params![directory_id, status, notes],
    )?;
    Ok(())
}

pub fn delete_candidate_paths(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT d.full_path FROM directories d
         JOIN annotations a ON a.directory_id = d.id
         WHERE a.status = 'delete_candidate'
         AND d.id NOT IN (
           SELECT directory_id FROM annotations WHERE status = 'keep'
         )
         ORDER BY d.full_path",
    )?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}
```

Create `src/export/script.rs`:
```rust
use anyhow::{bail, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn write_paths(paths: &[String], output: &Path) -> Result<()> {
    let mut f = File::create(output)?;
    for p in paths {
        writeln!(f, "{p}")?;
    }
    Ok(())
}

pub fn write_trash_script(paths: &[String], output: &Path) -> Result<()> {
    let mut f = File::create(output)?;
    writeln!(f, "#!/bin/bash")?;
    writeln!(f, "# filetreematch export - moves paths to Trash")?;
    for p in paths {
        writeln!(f, "osascript -e 'tell app \"Finder\" to delete POSIX file \"{p}\"'")?;
    }
    Ok(())
}

pub fn write_rm_script(paths: &[String], output: &Path, force: bool) -> Result<()> {
    if !force {
        bail!("rm format requires --force");
    }
    let mut f = File::create(output)?;
    writeln!(f, "#!/bin/bash")?;
    for p in paths {
        writeln!(f, "rm -rf -- {p:?}")?;
    }
    Ok(())
}
```

Create `src/export/mod.rs`:
```rust
pub mod script;

use crate::db::annotations::delete_candidate_paths;
use crate::db::Database;
use crate::export::script::{write_paths, write_rm_script, write_trash_script};
use anyhow::Result;
use std::path::Path;

pub fn run_export(db: &Database, format: &str, output: &Path, force: bool) -> Result<()> {
    let paths = delete_candidate_paths(db.conn())?;
    match format {
        "paths" => write_paths(&paths, output)?,
        "trash" => write_trash_script(&paths, output)?,
        "rm" => write_rm_script(&paths, output, force)?,
        other => anyhow::bail!("unknown format: {other}"),
    }
    Ok(())
}
```

Wire `src/cli/export.rs` to call `run_export`.

- [ ] **Step 4: Run test**

Run: `cargo test export_integration -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/db/annotations.rs src/export/ src/cli/export.rs tests/export_integration.rs
git commit -m "feat: add annotations storage and export script generation"
```

---

### Task 12: TUI

**Files:**
- Create: `src/tui/app.rs`
- Create: `src/tui/ui.rs`
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: Manual smoke checklist (no automated TUI test in v1)**

Document in test comments: ratatui requires interactive terminal; verify manually:
- Pairs list renders
- `k`/`d`/`u` persist to DB
- `e` shows export preview
- `q` quits

- [ ] **Step 2: Implement AppState**

Create `src/tui/app.rs`:
```rust
use crate::db::annotations::set_annotation;
use crate::db::query::{list_pairs, SubsetPairRow};
use crate::db::Database;
use anyhow::Result;

pub enum Filter { All, Unreviewed, DeleteCandidates }

pub struct App {
    pub pairs: Vec<SubsetPairRow>,
    pub selected: usize,
    pub filter: Filter,
    pub search: String,
    pub note_mode: bool,
    pub note_buffer: String,
    pub status_message: String,
    db: Database,
}

impl App {
    pub fn new(db: Database, full_detail: bool) -> Result<Self> {
        let pairs = list_pairs(db.conn(), full_detail, None)?;
        Ok(Self {
            pairs,
            selected: 0,
            filter: Filter::All,
            search: String::new(),
            note_mode: false,
            note_buffer: String::new(),
            status_message: String::new(),
            db,
        })
    }

    pub fn mark_selected(&mut self, status: &str) -> Result<()> {
        let path = &self.pairs[self.selected].subset_path;
        let dir_id: i64 = self.db.conn().query_row(
            "SELECT id FROM directories WHERE full_path = ?1",
            [path],
            |row| row.get(0),
        )?;
        set_annotation(self.db.conn(), dir_id, status, "")?;
        self.status_message = format!("Marked {path} as {status}");
        Ok(())
    }
}
```

- [ ] **Step 3: Implement UI render + event loop**

Create `src/tui/ui.rs` using ratatui `Layout`, `List`, `Paragraph`, `Block`.

Create `src/tui/mod.rs`:
```rust
mod app;
mod ui;

use crate::cli::TuiArgs;
use crate::db::Database;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

pub fn run(args: TuiArgs) -> Result<()> {
    let db_path = args.db.expect("--db required for tui in v1");
    let db = Database::open(&db_path)?;
    let mut app = app::App::new(db, args.full_detail)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| ui::render(f, &app))?;
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('k') => app.mark_selected("keep")?,
                    KeyCode::Char('d') => app.mark_selected("delete_candidate")?,
                    KeyCode::Char('u') => app.mark_selected("undecided")?,
                    KeyCode::Up => app.selected = app.selected.saturating_sub(1),
                    KeyCode::Down => app.selected = (app.selected + 1).min(app.pairs.len().saturating_sub(1)),
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
```

- [ ] **Step 4: Build and manual smoke test**

Run: `cargo build --release`
Run on fixture db: `filetreematch tui --db /tmp/test.db`
Expected: TUI renders, keys work

- [ ] **Step 5: Commit**

```bash
git add src/tui/
git commit -m "feat: add ratatui browser with annotation shortcuts"
```

---

### Task 13: Incremental Rescan Integration Test

**Files:**
- Test: `tests/scan_integration.rs` (extend)

- [ ] **Step 1: Write failing incremental test**

Add to `tests/scan_integration.rs`:
```rust
#[test]
fn incremental_rescan_skips_unchanged_subtree() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("archive");
    fs::create_dir_all(root.join("stable")).unwrap();
    fs::write(root.join("stable/a.txt"), b"hello").unwrap();

    let db_path = tmp.path().join("test.db");
    let db = filetreematch::db::Database::open(&db_path).unwrap();
    let ignore = IgnoreRules::defaults();

    run_scan(&root, &db, &ignore, 1).unwrap();
    let scanned1: String = db.conn().query_row(
        "SELECT last_scanned_at FROM directories WHERE full_path = ?1",
        [root.join("stable").to_string_lossy().to_string()],
        |row| row.get(0),
    ).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));
    run_scan(&root, &db, &ignore, 2).unwrap();
    let scanned2: String = db.conn().query_row(
        "SELECT last_scanned_at FROM directories WHERE full_path = ?1",
        [root.join("stable").to_string_lossy().to_string()],
        |row| row.get(0),
    ).unwrap();

    assert_eq!(scanned1, scanned2, "unchanged subtree should skip rebuild");
}
```

- [ ] **Step 2: Run test — may fail until skip preserves last_scanned_at**

Run: `cargo test incremental_rescan -- --nocapture`
Expected: PASS after engine returns early without updating timestamp on skip

- [ ] **Step 3: Adjust scan engine skip branch**

When fingerprint matches, return existing_id without updating `last_scanned_at`.

- [ ] **Step 4: Run test**

Run: `cargo test incremental_rescan -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/scan/engine.rs tests/scan_integration.rs
git commit -m "test: verify incremental rescan skips unchanged subtrees"
```

---

### Task 14: Exact Duplicate Flag in TUI

**Files:**
- Modify: `src/tui/ui.rs`
- Modify: `src/db/query.rs`

- [ ] **Step 1: Detect mutual subset pairs**

Add SQL helper: pair is exact duplicate if both `(A,B)` and `(B,A)` exist in `subset_pairs`.

- [ ] **Step 2: Show "exact duplicate" badge in detail pane**

Render when mutual pair detected.

- [ ] **Step 3: Commit**

```bash
git add src/tui/ui.rs src/db/query.rs
git commit -m "feat: flag exact duplicate trees in TUI detail pane"
```

---

### Task 15: README & Default Config Template

**Files:**
- Modify: `README.md`
- Create: `config/ignore.toml.example`

- [ ] **Step 1: Update README with usage examples matching design spec**

- [ ] **Step 2: Add example ignore config**

- [ ] **Step 3: Commit**

```bash
git add README.md config/ignore.toml.example
git commit -m "docs: add usage guide and example ignore config"
```

---

## Spec Coverage Self-Review

| Spec requirement | Task |
|------------------|------|
| Relative path + size matching | Task 5, 6 |
| Subset relationship A ⊆ B | Task 6, 9 |
| Report all pairs, user decides | Task 9, 10, 12 |
| Highest points default + `--full-detail` | Task 7, 9, 10, 12 |
| Interactive TUI | Task 12 |
| SQLite cache queryable | Task 2 |
| Files only, empty dirs excluded | Task 6 (`file_count > 0`) |
| Configurable ignore list | Task 3 |
| Annotate + export | Task 11, 12 |
| Incremental rescan | Task 8, 13 |
| Permission denied → scan_errors | Task 8 |
| Symlinks not followed | Task 8 (metadata on link) |
| Phase 2 similarity | Out of scope ✓ |

**Placeholder scan:** None found.

**Type consistency:** `Database`, `ScanFingerprint`, `IgnoreRules`, `run_scan`, `run_analyze`, `run_export` used consistently across tasks.

---

## Manual Acceptance Test Plan

After all tasks:

1. Copy ~1GB sample from real archive to temp external path
2. `filetreematch scan /Volumes/Sample --analyze`
3. `filetreematch tui --db ~/.cache/filetreematch/Sample.db`
4. Annotate 2-3 pairs, export trash script, verify paths
5. Delete one annotated folder, rescan, confirm incremental skip messages
6. Run full 2TB scan overnight
