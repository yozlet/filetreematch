use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

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
        params![
            parent_id,
            name,
            full_path,
            file_count,
            total_size,
            fingerprint
        ],
    )?;
    Ok(conn.query_row(
        "SELECT id FROM directories WHERE full_path = ?1",
        [full_path],
        |row| row.get(0),
    )?)
}

pub fn get_directory_id(conn: &Connection, full_path: &str) -> Result<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM directories WHERE full_path = ?1 AND deleted = 0",
            [full_path],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn get_fingerprint(conn: &Connection, dir_id: i64) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT scan_fingerprint FROM directories WHERE id = ?1",
            [dir_id],
            |row| row.get(0),
        )
        .optional()?)
}
