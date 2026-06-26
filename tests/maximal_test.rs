use filetreematch::analyze::maximal::compute_is_maximal;

#[test]
fn parent_subset_makes_child_non_maximal() {
    let pairs = vec![(1_i64, 10_i64), (2, 10)];
    let parent_of = |id: i64| -> Option<i64> {
        match id {
            2 => Some(1),
            _ => None,
        }
    };
    let is_subset_pair = |a: i64, _b: i64| a == 1 || a == 2;

    let maximal = compute_is_maximal(&pairs, &parent_of, &is_subset_pair);
    assert!(!maximal[&2]);
    assert!(maximal[&1]);
}
