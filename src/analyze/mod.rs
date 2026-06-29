pub mod containment;
pub mod index;
pub mod maximal;

use crate::analyze::index::ManifestIndex;
use crate::db::subset_pairs::{clear_pairs, insert_pair};
use crate::db::Database;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

pub fn run_analyze(db: &Database, _full_detail: bool) -> Result<()> {
    let conn = db.conn();
    clear_pairs(conn)?;

    let index = ManifestIndex::build(conn)?;

    let pb = ProgressBar::new(index.dir_count() as u64);
    pb.set_style(ProgressStyle::default_bar().template("{bar} {pos}/{len}").unwrap());

    let pairs = index.find_pairs(&pb);
    pb.finish_and_clear();

    let parent_of = |id: i64| index.parent_of(id);

    for (a_id, b_id) in &pairs {
        let is_maximal = !parent_of(*a_id).is_some_and(|pid| {
            pid == *b_id || index.is_manifest_subset(pid, *b_id)
        });
        let meta = index.meta_for(*a_id).expect("subset dir metadata");
        insert_pair(
            conn,
            *a_id,
            *b_id,
            meta.file_count,
            meta.total_size,
            is_maximal,
        )?;
    }

    let maximal_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM subset_pairs WHERE is_maximal = 1",
        [],
        |row| row.get(0),
    )?;
    println!(
        "Found {maximal_count} maximal subset pairs ({} total pairs)",
        pairs.len()
    );
    Ok(())
}
