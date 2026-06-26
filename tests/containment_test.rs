mod common;

use filetreematch::analyze::containment::is_subset;
use filetreematch::db::manifests::insert_manifest_entry;

#[test]
fn strict_subset_detected() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    conn.execute(
        "INSERT INTO directories (id, name, full_path, file_count, total_size, scan_fingerprint) VALUES
         (1, 'a', '/a', 1, 10, ''), (2, 'b', '/b', 2, 30, '')",
        [],
    )
    .unwrap();
    insert_manifest_entry(conn, 1, "x.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "x.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "y.txt", 20).unwrap();
    assert!(is_subset(conn, 1, 2).unwrap());
}

#[test]
fn non_subset_when_size_differs() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    conn.execute(
        "INSERT INTO directories (id, name, full_path, file_count, total_size, scan_fingerprint) VALUES
         (1, 'a', '/a', 1, 10, ''), (2, 'b', '/b', 1, 10, '')",
        [],
    )
    .unwrap();
    insert_manifest_entry(conn, 1, "x.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "x.txt", 99).unwrap();
    assert!(!is_subset(conn, 1, 2).unwrap());
}

#[test]
fn empty_manifest_is_not_analyzed() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    conn.execute(
        "INSERT INTO directories (id, name, full_path, file_count, total_size, scan_fingerprint) VALUES
         (1, 'empty', '/empty', 0, 0, '')",
        [],
    )
    .unwrap();
    assert!(!is_subset(conn, 1, 1).unwrap());
}
