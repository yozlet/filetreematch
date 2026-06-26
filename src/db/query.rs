use anyhow::Result;
use rusqlite::Connection;

pub struct SubsetPairRow {
    pub subset_path: String,
    pub superset_path: String,
    pub file_count: i64,
    pub total_size: i64,
    pub is_maximal: bool,
}

pub fn list_pairs(
    conn: &Connection,
    full_detail: bool,
    status_filter: Option<&str>,
) -> Result<Vec<SubsetPairRow>> {
    let mut sql = String::from(
        "SELECT ds.full_path, dp.full_path, sp.file_count, sp.total_size, sp.is_maximal
         FROM subset_pairs sp
         JOIN directories ds ON ds.id = sp.subset_dir_id
         JOIN directories dp ON dp.id = sp.superset_dir_id",
    );

    if status_filter.is_some() {
        sql.push_str(" JOIN annotations a ON a.directory_id = sp.subset_dir_id");
    }

    let mut conditions = Vec::new();
    if !full_detail {
        conditions.push("sp.is_maximal = 1".to_string());
    }
    if status_filter.is_some() {
        conditions.push("a.status = ?1".to_string());
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY sp.total_size DESC");

    let mut stmt = conn.prepare(&sql)?;
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(SubsetPairRow {
            subset_path: row.get(0)?,
            superset_path: row.get(1)?,
            file_count: row.get(2)?,
            total_size: row.get(3)?,
            is_maximal: row.get::<_, i64>(4)? != 0,
        })
    };

    let rows = if let Some(status) = status_filter {
        stmt.query_map([status], map_row)?
    } else {
        stmt.query_map([], map_row)?
    };

    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}
