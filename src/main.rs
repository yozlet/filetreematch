use anyhow::Result;
use clap::Parser;
use filetreematch::cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Scan(args) => filetreematch::cli::scan::run(args, cli.db),
        Commands::Analyze(args) => filetreematch::cli::analyze::run(args, cli.db),
        Commands::List(args) => filetreematch::cli::list::run(args, cli.db),
        Commands::Export(args) => filetreematch::cli::export::run(args, cli.db),
        Commands::Tui(args) => filetreematch::tui::run(args),
    }
}
