use crate::config::ignore::IgnoreRules;
use crate::db::directories::upsert_directory;
use crate::db::files::insert_file;
use crate::db::manifests::{insert_manifest_entry, rollup_manifest};
use crate::db::scans::log_scan_error;
use crate::db::Database;
use crate::scan::fingerprint::{hash_fingerprint, ScanFingerprint};
use anyhow::Result;
use std::fs;
use std::path::Path;

struct LocalFile {
    name: String,
    size: u64,
    mtime: i64,
}

pub fn run_scan(root: &Path, db: &Database, ignore: &IgnoreRules, scan_id: i64) -> Result<()> {
    scan_directory(root, None, db, ignore, scan_id)?;
    Ok(())
}

fn scan_directory(
    path: &Path,
    parent_id: Option<i64>,
    db: &Database,
    ignore: &IgnoreRules,
    scan_id: i64,
) -> Result<i64> {
    if ignore.should_ignore(path) {
        return Ok(0);
    }

    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let full_path = path.to_string_lossy().to_string();
    let conn = db.conn();

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) => {
            log_scan_error(conn, scan_id, &full_path, &err.to_string())?;
            return Ok(0);
        }
    };

    let mut subdirs = Vec::new();
    let mut local_files = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                log_scan_error(conn, scan_id, &full_path, &err.to_string())?;
                continue;
            }
        };
        let entry_path = entry.path();
        if ignore.should_ignore(&entry_path) {
            continue;
        }

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(err) => {
                log_scan_error(
                    conn,
                    scan_id,
                    &entry_path.to_string_lossy(),
                    &err.to_string(),
                )?;
                continue;
            }
        };

        if meta.is_dir() {
            subdirs.push(entry_path);
        } else if meta.is_file() || meta.file_type().is_symlink() {
            let mtime = file_mtime(&meta);
            local_files.push(LocalFile {
                name: entry.file_name().to_string_lossy().to_string(),
                size: meta.len(),
                mtime,
            });
        }
    }

    let mut child_ids = Vec::new();
    let mut child_fps = Vec::new();
    for subdir in subdirs {
        let child_id = scan_directory(&subdir, None, db, ignore, scan_id)?;
        if child_id > 0 {
            child_ids.push(child_id);
            child_fps.push(load_fingerprint(conn, child_id)?);
        }
    }

    let mut local_count = 0u64;
    let mut local_size = 0u64;
    let mut local_mtime = 0i64;
    for file in &local_files {
        local_count += 1;
        local_size += file.size;
        local_mtime = local_mtime.max(file.mtime);
    }

    let fp = ScanFingerprint::merge_with_local(&child_fps, local_count, local_size, local_mtime);
    let fp_hash = hash_fingerprint(&fp);

    let dir_id = upsert_directory(
        conn,
        parent_id,
        name,
        &full_path,
        fp.file_count as i64,
        fp.total_size as i64,
        &fp_hash,
    )?;

    for &child_id in &child_ids {
        conn.execute(
            "UPDATE directories SET parent_id = ?1 WHERE id = ?2",
            rusqlite::params![dir_id, child_id],
        )?;
    }

    for file in &local_files {
        insert_file(
            conn,
            dir_id,
            &file.name,
            file.size as i64,
            file.mtime,
        )?;
        insert_manifest_entry(conn, dir_id, &file.name, file.size as i64)?;
    }

    rollup_manifest(conn, dir_id, &child_ids)?;

    Ok(dir_id)
}

fn load_fingerprint(conn: &rusqlite::Connection, dir_id: i64) -> Result<ScanFingerprint> {
    conn.query_row(
        "SELECT file_count, total_size FROM directories WHERE id = ?1",
        [dir_id],
        |row| {
            Ok(ScanFingerprint {
                file_count: row.get::<_, i64>(0)? as u64,
                total_size: row.get::<_, i64>(1)? as u64,
                max_mtime: 0,
            })
        },
    )
    .map_err(Into::into)
}

fn file_mtime(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
