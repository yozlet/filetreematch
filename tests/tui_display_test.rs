use filetreematch::db::annotations::Annotation;
use filetreematch::db::query::SubsetPairRow;
use filetreematch::tui::display::build_rows;
use std::collections::HashMap;

fn sample_pair(subset: &str, superset: &str, exact: bool) -> SubsetPairRow {
    SubsetPairRow {
        subset_path: subset.to_string(),
        superset_path: superset.to_string(),
        file_count: 10,
        total_size: 1000,
        is_maximal: true,
        is_exact_duplicate: exact,
    }
}

#[test]
fn build_rows_uses_cached_annotations_for_markers() {
    let pairs = vec![
        sample_pair("/vol/a", "/vol/b", false),
        sample_pair("/vol/c", "/vol/d", false),
    ];
    let mut paths = HashMap::new();
    paths.insert("/vol/a".to_string(), 1);
    paths.insert("/vol/c".to_string(), 3);
    let mut annotations = HashMap::new();
    annotations.insert(
        1,
        Annotation {
            status: "delete_candidate".to_string(),
            notes: String::new(),
        },
    );
    annotations.insert(
        3,
        Annotation {
            status: "keep".to_string(),
            notes: String::new(),
        },
    );

    let rows = build_rows(&pairs, &paths, &annotations);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].annotation_marker, " [D]");
    assert_eq!(rows[1].annotation_marker, " [K]");
}

#[test]
fn build_rows_carries_exact_duplicate_flag() {
    let pairs = vec![sample_pair("/vol/a", "/vol/b", true)];
    let rows = build_rows(&pairs, &HashMap::new(), &HashMap::new());
    assert!(rows[0].is_exact_duplicate);
}
