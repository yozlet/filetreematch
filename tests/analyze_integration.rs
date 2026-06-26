mod common;

use filetreematch::analyze::run_analyze;
use filetreematch::config::ignore::IgnoreRules;
use filetreematch::db::list_pairs;
use filetreematch::scan::run_scan;
use std::fs;

#[test]
fn analyze_finds_maximal_subset_pair() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("archive");
    fs::create_dir_all(root.join("master/photos")).unwrap();
    fs::create_dir_all(root.join("old-pc/photos")).unwrap();
    fs::write(root.join("master/photos/a.jpg"), vec![0u8; 100]).unwrap();
    fs::write(root.join("old-pc/photos/a.jpg"), vec![0u8; 100]).unwrap();
    fs::write(root.join("master/extra.txt"), b"x").unwrap();

    let db_path = tmp.path().join("test.db");
    let db = filetreematch::db::Database::open(&db_path).unwrap();
    let ignore = IgnoreRules::defaults();
    run_scan(&root, &db, &ignore, 1).unwrap();
    run_analyze(&db, false).unwrap();

    let count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM subset_pairs WHERE is_maximal = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(count >= 1);
}

#[test]
fn list_shows_maximal_pairs_by_default() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("archive");
    fs::create_dir_all(root.join("master/photos")).unwrap();
    fs::create_dir_all(root.join("old-pc/photos")).unwrap();
    fs::write(root.join("master/photos/a.jpg"), vec![0u8; 100]).unwrap();
    fs::write(root.join("old-pc/photos/a.jpg"), vec![0u8; 100]).unwrap();
    fs::write(root.join("master/extra.txt"), b"x").unwrap();

    let db_path = tmp.path().join("test.db");
    let db = filetreematch::db::Database::open(&db_path).unwrap();
    let ignore = IgnoreRules::defaults();
    run_scan(&root, &db, &ignore, 1).unwrap();
    run_analyze(&db, false).unwrap();

    let pairs = list_pairs(db.conn(), false, None).unwrap();
    assert!(!pairs.is_empty());
    assert!(pairs.iter().any(|p| p.subset_path.contains("old-pc")));
}
