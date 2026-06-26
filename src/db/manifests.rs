use anyhow::Result;
use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub relative_path: String,
    pub size: i64,
}

pub fn clear_manifest(conn: &Connection, directory_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM manifest_entries WHERE directory_id = ?1",
        params![directory_id],
    )?;
    Ok(())
}

pub fn insert_manifest_entry(
    conn: &Connection,
    directory_id: i64,
    relative_path: &str,
    size: i64,
) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO manifest_entries (directory_id, relative_path, size) VALUES (?1, ?2, ?3)",
        params![directory_id, relative_path, size],
    )?;
    Ok(())
}

pub fn rollup_manifest(conn: &Connection, parent_id: i64, child_ids: &[i64]) -> Result<()> {
    let mut parent_entries: Vec<(String, i64)> = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT relative_path, size FROM manifest_entries WHERE directory_id = ?1",
        )?;
        let rows = stmt.query_map([parent_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        for row in rows {
            parent_entries.push(row?);
        }
    }

    clear_manifest(conn, parent_id)?;

    for (relative_path, size) in &parent_entries {
        insert_manifest_entry(conn, parent_id, relative_path, *size)?;
    }

    conn.execute(
        "INSERT OR REPLACE INTO manifest_entries (directory_id, relative_path, size)
         SELECT ?1, relative_path, size FROM files WHERE directory_id = ?1",
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
