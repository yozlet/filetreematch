use anyhow::{bail, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn write_paths(paths: &[String], output: &Path) -> Result<()> {
    let mut f = File::create(output)?;
    for p in paths {
        writeln!(f, "{p}")?;
    }
    Ok(())
}

pub fn write_trash_script(paths: &[String], output: &Path) -> Result<()> {
    let mut f = File::create(output)?;
    writeln!(f, "#!/bin/bash")?;
    writeln!(f, "# filetreematch export - moves paths to Trash")?;
    for p in paths {
        writeln!(
            f,
            "osascript -e 'tell app \"Finder\" to delete POSIX file \"{p}\"'"
        )?;
    }
    Ok(())
}

pub fn write_rm_script(paths: &[String], output: &Path, force: bool) -> Result<()> {
    if !force {
        bail!("rm format requires --force");
    }
    let mut f = File::create(output)?;
    writeln!(f, "#!/bin/bash")?;
    for p in paths {
        writeln!(f, "rm -rf -- {p:?}")?;
    }
    Ok(())
}
