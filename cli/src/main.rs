mod cli;

use std::path::PathBuf;
use clap::{Args, Parser, Subcommand};

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
	path: PathBuf,
}

#[derive(Args)]
struct ExtractArgs {
	path: PathBuf,
	destination: Option<PathBuf>,
}

#[derive(Args)]
struct FindPeArgs {
	path: PathBuf,
}

fn main() {
	let cli = Cli::parse();

	match &cli.command {
		Commands::Info(args) => cli::display_info(args),
		Commands::Extract(args) => cli::extract(args).unwrap(),
		Commands::FindPe(args) => cli::find_pe(args),
	}
}

