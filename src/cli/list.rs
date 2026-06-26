use super::ListArgs;
use crate::db::{list_pairs, open_db};
use anyhow::Result;
use std::path::PathBuf;

pub fn run(args: ListArgs, db: Option<PathBuf>) -> Result<()> {
    let database = open_db(db.as_deref())?;
    let pairs = list_pairs(
        database.conn(),
        args.full_detail,
        args.status.as_deref(),
    )?;

    for pair in &pairs {
        println!(
            "{} ⊂ {} ({} files, {})",
            pair.subset_path,
            pair.superset_path,
            pair.file_count,
            format_size(pair.total_size),
        );
    }

    Ok(())
}

fn format_size(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
