pub mod script;

use crate::db::annotations::delete_candidate_paths;
use crate::db::Database;
use crate::export::script::{write_paths, write_rm_script, write_trash_script};
use anyhow::Result;
use std::path::Path;

pub fn run_export(db: &Database, format: &str, output: &Path, force: bool) -> Result<()> {
    let paths = delete_candidate_paths(db.conn())?;
    match format {
        "paths" => write_paths(&paths, output)?,
        "trash" => write_trash_script(&paths, output)?,
        "rm" => write_rm_script(&paths, output, force)?,
        other => anyhow::bail!("unknown format: {other}"),
    }
    Ok(())
}
