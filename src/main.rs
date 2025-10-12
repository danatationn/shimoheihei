#[warn(unused_imports)]
#[warn(unused_variables)]

mod cli;
mod kif;
mod pe;

use std::path::{Path, PathBuf};
use clap::{Args, Parser, Subcommand};
use crate::cli::display_info;
use crate::Commands::Info;
use crate::kif::KifFile;
use crate::pe::{find_cs2_pe, Cs2PeFile};
use anyhow::Result;

#[derive(Parser)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Info(InfoArgs),
	FindPe(FindPeArgs),
}

#[derive(Args)]
struct InfoArgs {
	path: PathBuf,
}

#[derive(Args)]
struct FindPeArgs {
	path: PathBuf,
}

fn main() -> Result<()> {
	let cli = Cli::parse();
	//
	match &cli.command {
		Commands::Info(args) => cli::display_info(args),
		Commands::FindPe(args) => cli::find_pe(args),
	}

	Ok(())
}

