mod cli;
mod kif;
mod pe;

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
}

#[derive(Args)]
struct InfoArgs {
	path: PathBuf,
}

fn main() {
	let cli = Cli::parse();

	match &cli.command {
		Commands::Info(args) => cli::display_info(args),
		_ => ()
	}
}

