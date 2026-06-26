pub mod analyze;
pub mod export;
pub mod list;
pub mod scan;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "filetreematch", about = "Find subset-duplicate folder trees")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to SQLite cache (overrides default)
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    Scan(ScanArgs),
    Analyze(AnalyzeArgs),
    List(ListArgs),
    Export(ExportArgs),
    Tui(TuiArgs),
}

#[derive(clap::Args)]
pub struct ScanArgs {
    pub root: PathBuf,
    #[arg(long)]
    pub analyze: bool,
    #[arg(long = "ignore-add")]
    pub ignore_add: Vec<String>,
    #[arg(long = "ignore-file")]
    pub ignore_file: Option<PathBuf>,
    #[arg(long, default_value_t = 0)]
    pub threads: usize,
}

#[derive(clap::Args)]
pub struct AnalyzeArgs {
    #[arg(long)]
    pub full_detail: bool,
}

#[derive(clap::Args)]
pub struct ListArgs {
    #[arg(long)]
    pub full_detail: bool,
    #[arg(long)]
    pub status: Option<String>,
}

#[derive(clap::Args)]
pub struct ExportArgs {
    #[arg(long, default_value = "trash")]
    pub format: String,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub force: bool,
}

#[derive(clap::Args)]
pub struct TuiArgs {
    #[arg(long)]
    pub full_detail: bool,
}
