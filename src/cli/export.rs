use super::ExportArgs;
use crate::db::open_db;
use crate::export::run_export;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(args: ExportArgs, db: Option<PathBuf>) -> Result<()> {
    let database = open_db(db.as_deref())?;
    let output = args
        .output
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--output is required"))?;
    run_export(&database, &args.format, output, args.force)
}
