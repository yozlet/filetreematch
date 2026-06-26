mod common;

use filetreematch::db::Database;

#[test]
fn initializes_all_tables() {
    let (_tmp, db) = common::open_temp_db();
    let tables: Vec<String> = db
        .conn()
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(tables.contains(&"annotations".to_string()));
    assert!(tables.contains(&"directories".to_string()));
    assert!(tables.contains(&"files".to_string()));
    assert!(tables.contains(&"manifest_entries".to_string()));
    assert!(tables.contains(&"scan_errors".to_string()));
    assert!(tables.contains(&"scans".to_string()));
    assert!(tables.contains(&"subset_pairs".to_string()));
}
