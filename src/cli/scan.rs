use super::ScanArgs;
use crate::config::ignore::load_ignore_rules;
use crate::db::{finish_scan, start_scan, Database};
use crate::scan::run_scan;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(args: ScanArgs, db: Option<PathBuf>) -> Result<()> {
    let db_path = db.unwrap_or_else(|| Database::default_db_path(&args.root));
    let database = Database::open(&db_path)?;
    let ignore = load_ignore_rules(args.ignore_file.as_deref(), &args.ignore_add)?;
    let volume_id = args
        .root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("default");
    let scan_id = start_scan(
        database.conn(),
        &args.root.to_string_lossy(),
        volume_id,
    )?;
    run_scan(&args.root, &database, &ignore, scan_id)?;
    finish_scan(database.conn(), scan_id)?;
    if args.analyze {
        // Task 9: wire analyze after scan
    }
    Ok(())
}
