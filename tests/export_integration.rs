mod common;

use filetreematch::db::annotations::set_annotation;
use filetreematch::export::run_export;
use std::fs;

#[test]
fn export_skips_keep_annotated_paths() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();
    conn.execute(
        "INSERT INTO directories (id, name, full_path, scan_fingerprint) VALUES
         (1, 'del', '/archive/del', ''), (2, 'keep', '/archive/keep', '')",
        [],
    )
    .unwrap();
    set_annotation(conn, 1, "delete_candidate", "").unwrap();
    set_annotation(conn, 2, "keep", "").unwrap();

    let out = tempfile::NamedTempFile::new().unwrap();
    run_export(&db, "paths", out.path(), false).unwrap();
    let text = fs::read_to_string(out.path()).unwrap();
    assert!(text.contains("/archive/del"));
    assert!(!text.contains("/archive/keep"));
}
