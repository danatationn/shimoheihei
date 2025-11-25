mod extract;
mod find_pe;
mod info;

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Info(InfoArgs),
    Extract(ExtractArgs),
    FindPe(FindPeArgs),
}

#[derive(Args)]
struct InfoArgs {
    file_path: PathBuf,
    #[arg(long)]
    pe_path: Option<PathBuf>,
    #[arg(long)]
    toc_seed: Option<u32>,
}

#[derive(Args)]
struct ExtractArgs {
    kif_path: PathBuf,
    #[arg(long)]
    pe_path: Option<PathBuf>,
    #[arg(long)]
    toc_seed: Option<u32>,
    #[arg(long)]
    dest_path: Option<PathBuf>,
}

#[derive(Args)]
struct FindPeArgs {
    folder_path: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Info(args) => info::display_info(args),
        Commands::Extract(args) => extract::extract_kif(args).unwrap(),
        Commands::FindPe(args) => find_pe::find_pe(args),
    }
}
