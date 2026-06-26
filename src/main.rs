use anyhow::Result;
use clap::Parser;
use filetreematch::cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Scan(args) => filetreematch::cli::scan::run(args),
        Commands::Analyze(args) => filetreematch::cli::analyze::run(args),
        Commands::List(args) => filetreematch::cli::list::run(args),
        Commands::Export(args) => filetreematch::cli::export::run(args),
        Commands::Tui(args) => filetreematch::tui::run(args),
    }
}
