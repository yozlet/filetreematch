//! Profile TUI hot paths against a real database.
//!
//! Usage: cargo run --release --example tui_profile -- --db /path/to/db

use filetreematch::db::annotations::load_all_annotations;
use filetreematch::db::open_db;
use filetreematch::db::query::{count_pairs, list_pairs, list_pairs_query, load_path_index, ListPairsQuery};
use filetreematch::tui::display::build_rows;
use std::path::PathBuf;
use std::time::Instant;

fn timed<F, T>(label: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    eprintln!("{label}: {elapsed:.3?}");
    result
}

fn main() -> anyhow::Result<()> {
    let db_path: Option<PathBuf> = std::env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .windows(2)
        .find(|w| w[0] == "--db")
        .map(|w| PathBuf::from(&w[1]));

    let db = open_db(db_path.as_deref())?;
    let conn = db.conn();

    eprintln!(
        "Database: {}",
        db_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(default)".into())
    );
    eprintln!();

    let path_to_id = timed("load_path_index", || load_path_index(conn))?;
    eprintln!("  -> {} directories", path_to_id.len());

    let annotations = timed("load_all_annotations", || load_all_annotations(conn))?;
    eprintln!("  -> {} annotations", annotations.len());

    let pairs = timed("list_pairs (maximal only)", || list_pairs(conn, false, None))?;
    eprintln!("  -> {} pairs", pairs.len());

    // Break down list_pairs internals
    timed("list_pairs SQL only (no dedupe)", || {
        let mut stmt = conn
            .prepare(
                "SELECT ds.full_path, dp.full_path, sp.file_count, sp.total_size, sp.is_maximal
                 FROM subset_pairs sp
                 JOIN directories ds ON ds.id = sp.subset_dir_id
                 JOIN directories dp ON dp.id = sp.superset_dir_id
                 WHERE sp.is_maximal = 1
                 ORDER BY sp.total_size DESC",
            )
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .unwrap();
        rows.collect::<Result<Vec<_>, _>>().unwrap()
    });

    let rows = timed("build_rows", || build_rows(&pairs, &path_to_id, &annotations));
    eprintln!("  -> {} rows", rows.len());

    timed("render_list simulation (all rows)", || {
        let _items: Vec<String> = rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let prefix = if idx == 0 { "▶ " } else { "  " };
                format!("{prefix}{}{}  {} B", row.subset_path, row.annotation_marker, row.total_size)
            })
            .collect();
    });

    timed("select_next x100 (selection_detail only)", || {
        for i in 0..100 {
            let idx = i % rows.len().max(1);
            let row = &rows[idx];
            let _ = path_to_id.get(&row.subset_path).and_then(|id| annotations.get(id));
        }
    });

    timed("refresh_pairs equivalent (full reload)", || {
        let path_to_id = load_path_index(conn).unwrap();
        let annotations = load_all_annotations(conn).unwrap();
        let pairs = list_pairs(conn, false, None).unwrap();
        let _rows = build_rows(&pairs, &path_to_id, &annotations);
    });

    let query = ListPairsQuery {
        full_detail: false,
        ..Default::default()
    };
    timed("count_pairs (maximal only)", || count_pairs(conn, &query).unwrap());
    timed("list_pairs_query page of 500", || {
        list_pairs_query(
            conn,
            &ListPairsQuery {
                full_detail: false,
                limit: Some(500),
                offset: Some(0),
                ..Default::default()
            },
        )
        .unwrap()
    });
    timed("tui refresh_pairs equivalent (paged)", || {
        let annotations = load_all_annotations(conn).unwrap();
        let path_to_id = load_path_index(conn).unwrap();
        let total = count_pairs(conn, &query).unwrap();
        let pairs = list_pairs_query(
            conn,
            &ListPairsQuery {
                full_detail: false,
                limit: Some(500),
                offset: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        let _rows = build_rows(&pairs, &path_to_id, &annotations);
        let _ = total;
    });

    Ok(())
}
