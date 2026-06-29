//! Correctness tests for analyze: subset pair discovery and maximal marking.
//! These guard the optimized in-memory analyze path against regressions.

mod common;

use filetreematch::analyze::run_analyze;
use filetreematch::config::ignore::IgnoreRules;
use filetreematch::db::manifests::insert_manifest_entry;
use filetreematch::db::{list_pairs, Database};
use filetreematch::scan::run_scan;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

fn pair_paths(db: &Database, full_detail: bool) -> HashSet<(String, String)> {
    list_pairs(db.conn(), full_detail, None)
        .unwrap()
        .into_iter()
        .map(|p| (p.subset_path, p.superset_path))
        .collect()
}

fn maximal_pair_paths(db: &Database) -> HashSet<(String, String)> {
    list_pairs(db.conn(), false, None)
        .unwrap()
        .into_iter()
        .map(|p| (p.subset_path, p.superset_path))
        .collect()
}

fn insert_dir(
    conn: &rusqlite::Connection,
    id: i64,
    name: &str,
    full_path: &str,
    parent_id: Option<i64>,
    file_count: i64,
    total_size: i64,
) {
    conn.execute(
        "INSERT INTO directories (id, parent_id, name, full_path, file_count, total_size, scan_fingerprint)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, '')",
        rusqlite::params![id, parent_id, name, full_path, file_count, total_size],
    )
    .unwrap();
}

fn scan_fixture(root: &Path) -> (tempfile::TempDir, Database) {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let db = Database::open(&db_path).unwrap();
    run_scan(root, &db, &IgnoreRules::defaults(), 1).unwrap();
    (tmp, db)
}

#[test]
fn scanned_duplicate_tree_is_subset_of_larger_copy() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("archive");
    fs::create_dir_all(root.join("copy-a/photos")).unwrap();
    fs::create_dir_all(root.join("copy-b/photos")).unwrap();
    fs::write(root.join("copy-a/photos/x.jpg"), vec![0u8; 50]).unwrap();
    fs::write(root.join("copy-b/photos/x.jpg"), vec![0u8; 50]).unwrap();
    fs::write(root.join("copy-b/photos/y.jpg"), vec![0u8; 60]).unwrap();
    fs::write(root.join("copy-b/readme.txt"), b"extra").unwrap();

    let (_guard, db) = scan_fixture(&root);
    run_analyze(&db, false).unwrap();

    let pairs = pair_paths(&db, true);
    assert!(pairs.contains(&(
        root.join("copy-a").to_string_lossy().into_owned(),
        root.join("copy-b").to_string_lossy().into_owned(),
    )));
}

#[test]
fn child_folder_is_not_automatic_subset_of_parent() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("archive");
    fs::create_dir_all(root.join("backup/photos")).unwrap();
    fs::write(root.join("backup/photos/a.jpg"), vec![0u8; 50]).unwrap();
    fs::write(root.join("backup/readme.txt"), b"root file").unwrap();

    let (_guard, db) = scan_fixture(&root);
    run_analyze(&db, false).unwrap();

    let photos = root.join("backup/photos").to_string_lossy().into_owned();
    let backup = root.join("backup").to_string_lossy().into_owned();
    let pairs = pair_paths(&db, true);
    assert!(
        !pairs.contains(&(photos, backup)),
        "child files roll up with prefixed paths, so leaf != parent manifest paths"
    );
}

#[test]
fn scanned_top_level_folder_subset_of_sibling_with_extra_files() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("archive");
    fs::create_dir_all(root.join("master/photos")).unwrap();
    fs::create_dir_all(root.join("old-pc/photos")).unwrap();
    fs::write(root.join("master/photos/a.jpg"), vec![0u8; 100]).unwrap();
    fs::write(root.join("old-pc/photos/a.jpg"), vec![0u8; 100]).unwrap();
    fs::write(root.join("master/extra.txt"), b"only in master").unwrap();

    let (_guard, db) = scan_fixture(&root);
    run_analyze(&db, false).unwrap();

    let pairs = pair_paths(&db, true);
    assert!(pairs.contains(&(
        root.join("old-pc").to_string_lossy().into_owned(),
        root.join("master").to_string_lossy().into_owned(),
    )));
    let photo_pairs: Vec<_> = pairs
        .iter()
        .filter(|(subset, _)| subset.contains("photos"))
        .collect();
    assert_eq!(
        photo_pairs.len(),
        1,
        "identical photo folders should list one canonical pair"
    );
}

#[test]
fn partial_overlap_does_not_create_subset_pair() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    insert_dir(conn, 1, "a", "/vol/a", None, 2, 30);
    insert_dir(conn, 2, "b", "/vol/b", None, 2, 30);
    insert_manifest_entry(conn, 1, "shared.txt", 10).unwrap();
    insert_manifest_entry(conn, 1, "only-a.txt", 20).unwrap();
    insert_manifest_entry(conn, 2, "shared.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "only-b.txt", 20).unwrap();

    run_analyze(&db, false).unwrap();
    assert!(pair_paths(&db, true).is_empty());
}

#[test]
fn same_paths_different_sizes_are_not_subsets() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    insert_dir(conn, 1, "a", "/vol/a", None, 1, 10);
    insert_dir(conn, 2, "b", "/vol/b", None, 1, 99);
    insert_manifest_entry(conn, 1, "x.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "x.txt", 99).unwrap();

    run_analyze(&db, false).unwrap();
    assert!(pair_paths(&db, true).is_empty());
}

#[test]
fn identical_manifests_show_single_canonical_pair() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    insert_dir(conn, 1, "copy-a", "/vol/copy-a", None, 2, 30);
    insert_dir(conn, 2, "copy-b", "/vol/copy-b", None, 2, 30);
    for id in [1, 2] {
        insert_manifest_entry(conn, id, "a.txt", 10).unwrap();
        insert_manifest_entry(conn, id, "b.txt", 20).unwrap();
    }

    run_analyze(&db, false).unwrap();
    let pairs = pair_paths(&db, true);
    assert_eq!(pairs.len(), 1);
    assert!(pairs.contains(&("/vol/copy-a".to_string(), "/vol/copy-b".to_string()))
        || pairs.contains(&("/vol/copy-b".to_string(), "/vol/copy-a".to_string())));
}

#[test]
fn transitive_subsets_report_all_pairs_in_full_detail() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    insert_dir(conn, 1, "small", "/vol/small", None, 1, 10);
    insert_dir(conn, 2, "medium", "/vol/medium", None, 2, 30);
    insert_dir(conn, 3, "large", "/vol/large", None, 3, 60);
    insert_manifest_entry(conn, 1, "a.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "a.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "b.txt", 20).unwrap();
    insert_manifest_entry(conn, 3, "a.txt", 10).unwrap();
    insert_manifest_entry(conn, 3, "b.txt", 20).unwrap();
    insert_manifest_entry(conn, 3, "c.txt", 30).unwrap();

    run_analyze(&db, false).unwrap();
    let pairs = pair_paths(&db, true);
    assert!(pairs.contains(&("/vol/small".into(), "/vol/medium".into())));
    assert!(pairs.contains(&("/vol/small".into(), "/vol/large".into())));
    assert!(pairs.contains(&("/vol/medium".into(), "/vol/large".into())));
}

#[test]
fn nested_filesystem_subset_marks_inner_pair_non_maximal() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    insert_dir(conn, 3, "large", "/vol/large", None, 3, 60);
    insert_dir(conn, 2, "medium", "/vol/large/medium", Some(3), 2, 30);
    insert_dir(conn, 1, "small", "/vol/large/medium/small", Some(2), 1, 10);
    insert_manifest_entry(conn, 1, "a.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "a.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "b.txt", 20).unwrap();
    insert_manifest_entry(conn, 3, "a.txt", 10).unwrap();
    insert_manifest_entry(conn, 3, "b.txt", 20).unwrap();
    insert_manifest_entry(conn, 3, "c.txt", 30).unwrap();

    run_analyze(&db, false).unwrap();

    let all = pair_paths(&db, true);
    assert!(all.contains(&(
        "/vol/large/medium/small".into(),
        "/vol/large/medium".into()
    )));
    assert!(all.contains(&(
        "/vol/large/medium/small".into(),
        "/vol/large".into()
    )));
    assert!(all.contains(&(
        "/vol/large/medium".into(),
        "/vol/large".into()
    )));

    let maximal = maximal_pair_paths(&db);
    assert_eq!(maximal.len(), 0, "nested chain collapses when parent is also a subset");
}

#[test]
fn reanalyze_replaces_stale_pairs() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    insert_dir(conn, 1, "a", "/vol/a", None, 1, 10);
    insert_dir(conn, 2, "b", "/vol/b", None, 2, 30);
    insert_manifest_entry(conn, 1, "x.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "x.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "y.txt", 20).unwrap();

    run_analyze(&db, false).unwrap();
    assert_eq!(pair_paths(&db, true).len(), 1);

    conn.execute("DELETE FROM manifest_entries WHERE directory_id = 2 AND relative_path = 'y.txt'", [])
        .unwrap();
    conn.execute(
        "UPDATE directories SET file_count = 1, total_size = 10 WHERE id = 2",
        [],
    )
    .unwrap();

    run_analyze(&db, false).unwrap();
    let pairs = list_pairs(db.conn(), true, None).unwrap();
    assert_eq!(pairs.len(), 1);
    assert!(pairs[0].is_exact_duplicate);
    assert!(
        pairs[0].subset_path == "/vol/a" || pairs[0].subset_path == "/vol/b"
    );
}

#[test]
fn deleted_and_empty_directories_are_ignored() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    insert_dir(conn, 1, "live", "/vol/live", None, 1, 10);
    insert_dir(conn, 2, "empty", "/vol/empty", None, 0, 0);
    conn.execute(
        "INSERT INTO directories (id, name, full_path, file_count, total_size, scan_fingerprint, deleted)
         VALUES (3, 'gone', '/vol/gone', 1, 10, '', 1)",
        [],
    )
    .unwrap();
    insert_manifest_entry(conn, 1, "x.txt", 10).unwrap();
    insert_manifest_entry(conn, 3, "y.txt", 10).unwrap();

    run_analyze(&db, false).unwrap();
    assert!(pair_paths(&db, true).is_empty());
}

#[test]
fn same_file_names_different_relative_paths_are_not_subsets() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    insert_dir(conn, 1, "a", "/vol/a", None, 1, 10);
    insert_dir(conn, 2, "b", "/vol/b", None, 1, 10);
    insert_manifest_entry(conn, 1, "photos/a.jpg", 10).unwrap();
    insert_manifest_entry(conn, 2, "backup/a.jpg", 10).unwrap();

    run_analyze(&db, false).unwrap();
    assert!(pair_paths(&db, true).is_empty());
}

#[test]
fn small_subset_found_among_many_unrelated_superset_candidates() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    insert_dir(conn, 1, "needle", "/vol/needle", None, 1, 5);
    insert_manifest_entry(conn, 1, "unique.txt", 5).unwrap();

    for i in 2..52 {
        let path = format!("/vol/big-{i}");
        insert_dir(conn, i, &format!("big-{i}"), &path, None, 100, 10_000);
        for j in 0..100 {
            insert_manifest_entry(conn, i, &format!("file-{j}.bin"), 100).unwrap();
        }
    }

    insert_dir(conn, 52, "haystack", "/vol/haystack", None, 100, 10_005);
    for j in 0..100 {
        insert_manifest_entry(conn, 52, &format!("file-{j}.bin"), 100).unwrap();
    }
    insert_manifest_entry(conn, 52, "unique.txt", 5).unwrap();

    run_analyze(&db, false).unwrap();
    let pairs = pair_paths(&db, true);
    assert!(pairs.contains(&("/vol/needle".into(), "/vol/haystack".into())));
    assert_eq!(
        pairs
            .iter()
            .filter(|(subset, _)| subset == "/vol/needle")
            .count(),
        1
    );
}

#[test]
fn large_manifest_subset_of_ancestor_still_detected() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    const N: i64 = 5_100;

    insert_dir(conn, 2, "parent", "/vol/parent", None, N + 1, N * 10 + 7);
    insert_dir(conn, 1, "child", "/vol/parent/child", Some(2), N, N * 10);

    for i in 0..N {
        insert_manifest_entry(conn, 1, &format!("child-{i}.dat"), 10).unwrap();
        insert_manifest_entry(conn, 2, &format!("child-{i}.dat"), 10).unwrap();
    }
    insert_manifest_entry(conn, 2, "parent-only.dat", 7).unwrap();

    run_analyze(&db, false).unwrap();
    let pairs = pair_paths(&db, true);
    assert!(pairs.contains(&("/vol/parent/child".into(), "/vol/parent".into())));
}

#[test]
fn scanned_matching_leaf_folders_are_mutual_subsets() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("archive");
    fs::create_dir_all(root.join("tree-a/b/c")).unwrap();
    fs::create_dir_all(root.join("tree-b/b/c")).unwrap();
    fs::write(root.join("tree-a/b/c/leaf.txt"), vec![0u8; 42]).unwrap();
    fs::write(root.join("tree-b/b/c/leaf.txt"), vec![0u8; 42]).unwrap();
    fs::write(root.join("tree-b/b/c/extra.txt"), vec![0u8; 7]).unwrap();

    let (_guard, db) = scan_fixture(&root);
    run_analyze(&db, false).unwrap();

    let leaf_a = root.join("tree-a/b/c").to_string_lossy().into_owned();
    let leaf_b = root.join("tree-b/b/c").to_string_lossy().into_owned();
    let pairs = pair_paths(&db, true);
    assert!(pairs.contains(&(leaf_a.clone(), leaf_b.clone())));
    assert!(!pairs.contains(&(leaf_a, root.join("tree-a").to_string_lossy().into_owned())));
}
