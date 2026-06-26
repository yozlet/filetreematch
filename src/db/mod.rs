pub mod directories;
pub mod files;
pub mod manifests;
pub mod scans;
pub mod subset_pairs;

use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

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

    pub fn default_db_path(root: &Path) -> PathBuf {
        let volume_id = root.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("default");
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cache/filetreematch")
            .join(format!("{volume_id}.db"))
    }
}

pub fn open_db(db_arg: Option<&Path>) -> Result<Database> {
    if let Some(p) = db_arg {
        return Database::open(p);
    }

    let cache_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache/filetreematch");

    if !cache_dir.is_dir() {
        bail!(
            "no database found: specify --db or run scan first (cache dir {} does not exist)",
            cache_dir.display()
        );
    }

    let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(&cache_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("db") {
            let mtime = entry.metadata()?.modified()?;
            if latest.as_ref().is_none_or(|(t, _)| mtime > *t) {
                latest = Some((mtime, path));
            }
        }
    }

    match latest {
        Some((_, path)) => Database::open(&path),
        None => bail!(
            "no database found in {}: specify --db or run scan first",
            cache_dir.display()
        ),
    }
}

pub use directories::*;
pub use files::*;
pub use manifests::*;
pub use scans::*;
pub use subset_pairs::*;
