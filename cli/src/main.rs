mod extract;
mod utils;
mod info;
mod convert;

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
    Convert(ConvertArgs)
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
    #[arg(short, long, default_value = "bmp")]
    convert_images_to: Option<String>,
    #[arg(long)]
    dont_convert_images: bool,
}

#[derive(Args)]
struct FindPeArgs {
    folder_path: PathBuf,
}

#[derive(Args)]
struct ConvertArgs {
	file_path: PathBuf,
	dest_path: PathBuf
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Info(args) => info::display_info(args),
        Commands::Extract(args) => extract::extract(args).unwrap(),
        Commands::Convert(args) => convert::convert(args).unwrap(),
    }
}
