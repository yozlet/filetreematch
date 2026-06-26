pub mod containment;
pub mod maximal;

use crate::analyze::containment::is_subset;
use crate::analyze::maximal::compute_is_maximal;
use crate::db::subset_pairs::{clear_pairs, insert_pair};
use crate::db::Database;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use rusqlite::params;
use std::collections::{HashMap, HashSet};

pub fn run_analyze(db: &Database, _full_detail: bool) -> Result<()> {
    let conn = db.conn();
    clear_pairs(conn)?;

    let mut stmt = conn.prepare(
        "SELECT id, file_count, total_size, parent_id FROM directories
         WHERE deleted = 0 AND file_count > 0 ORDER BY file_count ASC",
    )?;
    let dirs: Vec<(i64, i64, i64, Option<i64>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
        .collect::<Result<_, _>>()?;

    let pb = ProgressBar::new(dirs.len() as u64);
    pb.set_style(ProgressStyle::default_bar().template("{bar} {pos}/{len}").unwrap());

    let mut pairs = Vec::new();
    for (a_id, a_count, a_size, _) in &dirs {
        let mut candidates = conn.prepare(
            "SELECT id FROM directories
             WHERE deleted = 0 AND id != ?1 AND file_count >= ?2 AND total_size >= ?3",
        )?;
        let b_ids: Vec<i64> = candidates
            .query_map(params![a_id, a_count, a_size], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        for b_id in b_ids {
            if is_subset(conn, *a_id, b_id)? {
                pairs.push((*a_id, b_id));
            }
        }
        pb.inc(1);
    }
    pb.finish_and_clear();

    let parent_map: HashMap<i64, i64> = dirs
        .iter()
        .filter_map(|(id, _, _, p)| p.map(|pid| (*id, pid)))
        .collect();

    let pair_set: HashSet<(i64, i64)> = pairs.iter().copied().collect();
    let is_subset_pair = |subset: i64, superset: i64| pair_set.contains(&(subset, superset));
    let parent_of = |id: i64| parent_map.get(&id).copied();

    let maximal = compute_is_maximal(
        &pairs,
        &|id| parent_of(id),
        &|a, b| is_subset_pair(a, b),
    );

    for (a_id, b_id) in &pairs {
        let (file_count, total_size): (i64, i64) = conn.query_row(
            "SELECT file_count, total_size FROM directories WHERE id = ?1",
            [a_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        insert_pair(
            conn,
            *a_id,
            *b_id,
            file_count,
            total_size,
            maximal[a_id],
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
