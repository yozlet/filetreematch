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

#[derive(Clone, Copy, Default)]
pub struct ListPairsQuery<'a> {
    pub full_detail: bool,
    pub status_filter: Option<&'a str>,
    pub unreviewed_only: bool,
    pub search: Option<&'a str>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

struct PairSql {
    sql: String,
    params: Vec<String>,
}

fn build_pair_sql(query: &ListPairsQuery<'_>) -> PairSql {
    let mut sql = String::from(
        "SELECT ds.full_path, dp.full_path, sp.file_count, sp.total_size, sp.is_maximal,
                EXISTS(
                    SELECT 1 FROM subset_pairs sp_rev
                    WHERE sp_rev.subset_dir_id = sp.superset_dir_id
                      AND sp_rev.superset_dir_id = sp.subset_dir_id
                ) AS is_exact_duplicate
         FROM subset_pairs sp
         JOIN directories ds ON ds.id = sp.subset_dir_id
         JOIN directories dp ON dp.id = sp.superset_dir_id",
    );

    let mut params = Vec::new();

    if query.status_filter.is_some() || query.unreviewed_only {
        sql.push_str(" LEFT JOIN annotations a ON a.directory_id = sp.subset_dir_id");
    }

    let mut conditions = vec![
        "NOT (
            EXISTS(
                SELECT 1 FROM subset_pairs sp_rev
                WHERE sp_rev.subset_dir_id = sp.superset_dir_id
                  AND sp_rev.superset_dir_id = sp.subset_dir_id
            )
            AND ds.full_path > dp.full_path
        )"
        .to_string(),
    ];

    if !query.full_detail {
        conditions.push("sp.is_maximal = 1".to_string());
    }
    if let Some(status) = query.status_filter {
        conditions.push("a.status = ?".to_string());
        params.push(status.to_string());
    }
    if query.unreviewed_only {
        conditions.push("a.directory_id IS NULL".to_string());
    }
    if let Some(search) = query.search {
        if !search.is_empty() {
            let pattern = format!("%{}%", search.to_lowercase());
            conditions.push(
                "(LOWER(ds.full_path) LIKE ? OR LOWER(dp.full_path) LIKE ?)".to_string(),
            );
            params.push(pattern.clone());
            params.push(pattern);
        }
    }

    sql.push_str(" WHERE ");
    sql.push_str(&conditions.join(" AND "));

    PairSql { sql, params }
}

pub fn count_pairs(conn: &Connection, query: &ListPairsQuery<'_>) -> Result<usize> {
    let base = build_pair_sql(query);
    let sql = format!("SELECT COUNT(*) FROM ({}) sub", base.sql);
    let mut stmt = conn.prepare(&sql)?;
    let count = if base.params.is_empty() {
        stmt.query_row([], |row| row.get::<_, i64>(0))?
    } else {
        let param_refs: Vec<&dyn rusqlite::ToSql> = base
            .params
            .iter()
            .map(|p| p as &dyn rusqlite::ToSql)
            .collect();
        stmt.query_row(param_refs.as_slice(), |row| row.get::<_, i64>(0))?
    };
    Ok(count as usize)
}

pub fn list_pairs_query(
    conn: &Connection,
    query: &ListPairsQuery<'_>,
) -> Result<Vec<SubsetPairRow>> {
    let base = build_pair_sql(query);
    let mut sql = base.sql;
    sql.push_str(" ORDER BY sp.total_size DESC");
    if let Some(limit) = query.limit {
        sql.push_str(&format!(" LIMIT {limit}"));
    }
    if let Some(offset) = query.offset {
        sql.push_str(&format!(" OFFSET {offset}"));
    }

    let mut stmt = conn.prepare(&sql)?;
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(SubsetPairRow {
            subset_path: row.get(0)?,
            superset_path: row.get(1)?,
            file_count: row.get(2)?,
            total_size: row.get(3)?,
            is_maximal: row.get::<_, i64>(4)? != 0,
            is_exact_duplicate: row.get::<_, i64>(5)? != 0,
        })
    };

    let rows = if base.params.is_empty() {
        stmt.query_map([], map_row)?
    } else {
        let param_refs: Vec<&dyn rusqlite::ToSql> = base
            .params
            .iter()
            .map(|p| p as &dyn rusqlite::ToSql)
            .collect();
        stmt.query_map(param_refs.as_slice(), map_row)?
    };

    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn list_pairs(
    conn: &Connection,
    full_detail: bool,
    status_filter: Option<&str>,
) -> Result<Vec<SubsetPairRow>> {
    list_pairs_query(
        conn,
        &ListPairsQuery {
            full_detail,
            status_filter,
            ..Default::default()
        },
    )
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
