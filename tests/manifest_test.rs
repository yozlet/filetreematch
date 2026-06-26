mod common;

use filetreematch::db::manifests::{insert_manifest_entry, rollup_manifest, ManifestEntry};

#[test]
fn rollup_prefixes_child_paths() {
    let (_tmp, db) = common::open_temp_db();
    let conn = db.conn();

    conn.execute(
        "INSERT INTO directories (id, name, full_path, scan_fingerprint) VALUES (1, 'root', '/root', ''), (2, 'child', '/root/child', '')",
        [],
    )
    .unwrap();
    insert_manifest_entry(conn, 2, "a.txt", 10).unwrap();
    insert_manifest_entry(conn, 2, "sub/b.txt", 20).unwrap();
    insert_manifest_entry(conn, 1, "local.txt", 5).unwrap();

    rollup_manifest(conn, 1, &[2]).unwrap();

    let entries = fetch_manifest(conn, 1);
    assert_eq!(entries.len(), 3);
    assert!(entries.contains(&ManifestEntry {
        relative_path: "local.txt".into(),
        size: 5
    }));
    assert!(entries.contains(&ManifestEntry {
        relative_path: "child/a.txt".into(),
        size: 10
    }));
    assert!(entries.contains(&ManifestEntry {
        relative_path: "child/sub/b.txt".into(),
        size: 20
    }));
}

fn fetch_manifest(conn: &rusqlite::Connection, dir_id: i64) -> Vec<ManifestEntry> {
    let mut stmt = conn
        .prepare("SELECT relative_path, size FROM manifest_entries WHERE directory_id = ?1")
        .unwrap();
    stmt.query_map([dir_id], |row| {
        Ok(ManifestEntry {
            relative_path: row.get(0)?,
            size: row.get(1)?,
        })
    })
    .unwrap()
    .map(|r| r.unwrap())
    .collect()
}
