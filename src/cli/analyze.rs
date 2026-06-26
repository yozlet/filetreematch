use super::AnalyzeArgs;
use crate::analyze::run_analyze;
use crate::db::open_db;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(args: AnalyzeArgs, db: Option<PathBuf>) -> Result<()> {
    let database = open_db(db.as_deref())?;
    run_analyze(&database, args.full_detail)
}
