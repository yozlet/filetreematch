use filetreematch::scan::fingerprint::{ScanFingerprint, hash_fingerprint};

#[test]
fn fingerprint_hash_changes_when_file_count_changes() {
    let a = ScanFingerprint { file_count: 1, total_size: 100, max_mtime: 1000 };
    let b = ScanFingerprint { file_count: 2, total_size: 100, max_mtime: 1000 };
    assert_ne!(hash_fingerprint(&a), hash_fingerprint(&b));
}

#[test]
fn merge_child_fingerprints() {
    let child = ScanFingerprint { file_count: 2, total_size: 50, max_mtime: 500 };
    let parent = ScanFingerprint::merge(&[child]);
    assert_eq!(parent.file_count, 2);
    assert_eq!(parent.total_size, 50);
    assert_eq!(parent.max_mtime, 500);
}

#[test]
fn merge_includes_local_file() {
    let child = ScanFingerprint { file_count: 1, total_size: 10, max_mtime: 100 };
    let parent = ScanFingerprint::merge_with_local(&[child], 1, 20, 200);
    assert_eq!(parent.file_count, 2);
    assert_eq!(parent.total_size, 30);
    assert_eq!(parent.max_mtime, 200);
}
