use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

pub struct SubsetPairRow {
    pub subset_path: String,
    pub superset_path: String,
    pub file_count: i64,
    pub total_size: i64,
    pub is_maximal: bool,
    pub is_exact_duplicate: bool,
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
            is_exact_duplicate: false,
        })
    };

    let rows = if let Some(status) = status_filter {
        stmt.query_map([status], map_row)?
    } else {
        stmt.query_map([], map_row)?
    };

    let mut pairs: Vec<SubsetPairRow> = rows.collect::<Result<_, _>>()?;
    dedupe_exact_duplicate_pairs(&mut pairs);
    Ok(pairs)
}

fn dedupe_exact_duplicate_pairs(pairs: &mut Vec<SubsetPairRow>) {
    use std::collections::HashSet;

    let reverse_exists: HashSet<(String, String)> = pairs
        .iter()
        .map(|p| (p.superset_path.clone(), p.subset_path.clone()))
        .collect();

    for pair in pairs.iter_mut() {
        pair.is_exact_duplicate = reverse_exists.contains(&(pair.subset_path.clone(), pair.superset_path.clone()));
    }

    pairs.retain(|pair| {
        if pair.is_exact_duplicate && pair.subset_path > pair.superset_path {
            return false;
        }
        true
    });
}

pub fn is_exact_duplicate(
    conn: &Connection,
    subset_dir_id: i64,
    superset_dir_id: i64,
) -> Result<bool> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM subset_pairs
            WHERE subset_dir_id = ?1 AND superset_dir_id = ?2
        )",
        [superset_dir_id, subset_dir_id],
        |row| row.get(0),
    )?;
    Ok(exists)
}

pub fn load_path_index(conn: &Connection) -> Result<HashMap<String, i64>> {
    let mut stmt = conn.prepare("SELECT full_path, id FROM directories WHERE deleted = 0")?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get(1)?)))?;
    rows.collect::<Result<HashMap<_, _>, _>>()
        .map_err(Into::into)
}
