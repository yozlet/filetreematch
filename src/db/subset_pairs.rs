use anyhow::Result;
use rusqlite::{params, Connection};

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
        params![
            subset_dir_id,
            superset_dir_id,
            file_count,
            total_size,
            is_maximal as i64
        ],
    )?;
    Ok(())
}
