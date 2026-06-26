use anyhow::Result;
use rusqlite::{params, Connection};

pub fn set_annotation(
    conn: &Connection,
    directory_id: i64,
    status: &str,
    notes: &str,
) -> Result<()> {
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
