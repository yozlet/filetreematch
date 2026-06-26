use anyhow::Result;
use rusqlite::{params, Connection};

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
