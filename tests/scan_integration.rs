mod common;

use filetreematch::config::ignore::IgnoreRules;
use filetreematch::scan::run_scan;
use std::fs;

#[test]
fn scan_populates_manifests_for_fixture_tree() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("archive");
    fs::create_dir_all(root.join("master/photos")).unwrap();
    fs::create_dir_all(root.join("old-pc/photos")).unwrap();
    fs::write(root.join("master/photos/a.jpg"), vec![0u8; 100]).unwrap();
    fs::write(root.join("old-pc/photos/a.jpg"), vec![0u8; 100]).unwrap();
    fs::write(root.join("master/extra.txt"), b"only in master").unwrap();

    let db_path = tmp.path().join("test.db");
    let db = filetreematch::db::Database::open(&db_path).unwrap();
    run_scan(&root, &db, &IgnoreRules::defaults(), 1).unwrap();

    let count: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM manifest_entries", [], |row| row.get(0))
        .unwrap();
    assert!(count > 0);

    let dirs: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM directories WHERE deleted = 0",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(dirs >= 4);
}

#[test]
fn incremental_rescan_skips_unchanged_subtree() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("archive");
    fs::create_dir_all(root.join("stable")).unwrap();
    fs::write(root.join("stable/a.txt"), b"hello").unwrap();

    let db_path = tmp.path().join("test.db");
    let db = filetreematch::db::Database::open(&db_path).unwrap();
    let ignore = IgnoreRules::defaults();

    run_scan(&root, &db, &ignore, 1).unwrap();
    let scanned1: String = db.conn().query_row(
        "SELECT last_scanned_at FROM directories WHERE full_path = ?1",
        [root.join("stable").to_string_lossy().to_string()],
        |row| row.get(0),
    ).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));
    run_scan(&root, &db, &ignore, 2).unwrap();
    let scanned2: String = db.conn().query_row(
        "SELECT last_scanned_at FROM directories WHERE full_path = ?1",
        [root.join("stable").to_string_lossy().to_string()],
        |row| row.get(0),
    ).unwrap();

    assert_eq!(scanned1, scanned2, "unchanged subtree should skip rebuild");
}
