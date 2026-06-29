mod common;

use filetreematch::db::manifests::insert_manifest_entry;
use filetreematch::db::subset_pairs::insert_pair;
use filetreematch::db::list_pairs;

fn insert_dir(
    conn: &rusqlite::Connection,
    id: i64,
    name: &str,
    full_path: &str,
    file_count: i64,
    total_size: i64,
) {
    conn.execute(
        "INSERT INTO directories (id, name, full_path, file_count, total_size, scan_fingerprint)
         VALUES (?1, ?2, ?3, ?4, ?5, '')",
        rusqlite::params![id, name, full_path, file_count, total_size],
    )
    .unwrap();
}

#[test]
fn list_pairs_shows_one_canonical_entry_for_exact_duplicates() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    insert_dir(conn, 1, "copy-a", "/vol/copy-a", 2, 30);
    insert_dir(conn, 2, "copy-b", "/vol/copy-b", 2, 30);
    for id in [1, 2] {
        insert_manifest_entry(conn, id, "a.txt", 10).unwrap();
        insert_manifest_entry(conn, id, "b.txt", 20).unwrap();
    }
    insert_pair(conn, 1, 2, 2, 30, true).unwrap();
    insert_pair(conn, 2, 1, 2, 30, true).unwrap();

    let pairs = list_pairs(conn, true, None).unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].subset_path, "/vol/copy-a");
    assert_eq!(pairs[0].superset_path, "/vol/copy-b");
    assert!(pairs[0].is_exact_duplicate);
}

#[test]
fn list_pairs_keeps_both_directions_for_proper_subsets() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    insert_dir(conn, 1, "small", "/vol/small", 1, 10);
    insert_dir(conn, 2, "large", "/vol/large", 2, 30);
    insert_manifest_entry(conn, 1, "a.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "a.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "b.txt", 20).unwrap();
    insert_pair(conn, 1, 2, 1, 10, true).unwrap();

    let pairs = list_pairs(conn, true, None).unwrap();
    assert_eq!(pairs.len(), 1);
    assert!(!pairs[0].is_exact_duplicate);
}
