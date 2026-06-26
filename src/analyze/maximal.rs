use std::collections::HashMap;

pub fn compute_is_maximal(
    pairs: &[(i64, i64)],
    parent_of: &dyn Fn(i64) -> Option<i64>,
    is_subset_of: &dyn Fn(i64, i64) -> bool,
) -> HashMap<i64, bool> {
    let mut result = HashMap::new();
    for &(subset_id, superset_id) in pairs {
        let parent = parent_of(subset_id);
        let non_maximal = parent.is_some_and(|pid| is_subset_of(pid, superset_id));
        result.insert(subset_id, !non_maximal);
    }
    result
}
