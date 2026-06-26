use anyhow::Result;
use rusqlite::{params, Connection};

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
