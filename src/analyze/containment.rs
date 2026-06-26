use anyhow::Result;
use rusqlite::{params, Connection};

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
