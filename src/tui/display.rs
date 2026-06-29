use crate::db::annotations::Annotation;
use crate::db::query::SubsetPairRow;
use std::collections::HashMap;

pub struct TuiRow {
    pub subset_path: String,
    pub superset_path: String,
    pub file_count: i64,
    pub total_size: i64,
    pub annotation_marker: String,
    pub is_exact_duplicate: bool,
}

pub struct SelectionDetail {
    pub status: String,
    pub notes: String,
    pub is_exact_duplicate: bool,
}

pub fn build_rows(
    pairs: &[SubsetPairRow],
    path_to_id: &HashMap<String, i64>,
    annotations: &HashMap<i64, Annotation>,
) -> Vec<TuiRow> {
    pairs
        .iter()
        .map(|pair| TuiRow {
            subset_path: pair.subset_path.clone(),
            superset_path: pair.superset_path.clone(),
            file_count: pair.file_count,
            total_size: pair.total_size,
            annotation_marker: annotation_marker(path_to_id, annotations, &pair.subset_path),
            is_exact_duplicate: pair.is_exact_duplicate,
        })
        .collect()
}

pub fn selection_detail(
    row: &TuiRow,
    path_to_id: &HashMap<String, i64>,
    annotations: &HashMap<i64, Annotation>,
) -> SelectionDetail {
    let (status, notes) = path_to_id
        .get(&row.subset_path)
        .and_then(|id| annotations.get(id))
        .map(|a| (a.status.clone(), a.notes.clone()))
        .unwrap_or_else(|| ("unreviewed".to_string(), String::new()));

    SelectionDetail {
        status,
        notes,
        is_exact_duplicate: row.is_exact_duplicate,
    }
}

pub fn window_offset_for_selection(selected: usize, total: usize, page_size: usize) -> usize {
    if total <= page_size {
        return 0;
    }
    let half = page_size / 2;
    selected.saturating_sub(half).min(total.saturating_sub(page_size))
}

pub fn render_slice_bounds(
    selected_global: usize,
    window_offset: usize,
    loaded_count: usize,
    viewport_height: usize,
) -> (usize, usize) {
    if loaded_count == 0 || viewport_height == 0 {
        return (0, 0);
    }
    let local_selected = selected_global.saturating_sub(window_offset);
    let visible = viewport_height.min(loaded_count);
    let start = local_selected
        .saturating_sub(visible / 2)
        .min(loaded_count.saturating_sub(visible));
    (start, visible)
}

fn annotation_marker(
    path_to_id: &HashMap<String, i64>,
    annotations: &HashMap<i64, Annotation>,
    subset_path: &str,
) -> String {
    let Some(dir_id) = path_to_id.get(subset_path) else {
        return String::new();
    };
    let Some(annotation) = annotations.get(dir_id) else {
        return String::new();
    };
    match annotation.status.as_str() {
        "keep" => " [K]".to_string(),
        "delete_candidate" => " [D]".to_string(),
        _ => " [U]".to_string(),
    }
}
