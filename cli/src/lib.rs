use std::{path::PathBuf, sync::LazyLock};
use clap::{Args, Parser, Subcommand};
use indicatif::ProgressStyle;
use smhh::pe::{Cs2Pe, find_cs2_pe};

pub mod list;
pub mod unpack;
pub mod info;

///
/// clap config
///

#[derive(Parser)]
#[command(
	name = "shimoheihei",
	bin_name = "shimoheihei",
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Info(InfoArgs),
    Unpack(UnpackArgs),
    List(ListArgs),
}

#[derive(Args, Clone)]
pub struct InfoArgs {
    file_path: PathBuf,
    #[arg(long)]
    pe_path: Option<PathBuf>,
    #[arg(long)]
    toc_seed: Option<u32>,
}

#[derive(Args, Clone)]
pub struct ListArgs {
	file_path: PathBuf,
	#[arg(long)]
    pe_path: Option<PathBuf>,
    #[arg(long)]
    toc_seed: Option<u32>,
}

#[derive(Args, Clone)]
pub struct UnpackArgs {
    paths: Vec<PathBuf>,
    #[arg(short, long)]
    dest_path: Option<PathBuf>,
    #[arg(long)]
    pe_path: Option<PathBuf>,
    #[arg(long)]
    toc_seed: Option<u32>,
    #[arg(short, long, default_value = "png")]
    convert_hgx_to: String,
    #[arg(short = 'r', long)]
    no_convert: bool,
    #[arg(short = 'o', long)]
    no_overwrite: bool,
}

///
/// indicatif consts
///

pub static SPINNER_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
	ProgressStyle::with_template("{spinner} {prefix} {msg}")
    	.expect("Invalid template")
     	.tick_chars("|/-\\O")
});

pub static BAR_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
	ProgressStyle::with_template(" {msg} | {elapsed} [{wide_bar}] ETA: {eta} | {pos}/{len} ")
		.expect("Invalid template")
		.progress_chars("#--")
});

///
/// helper functions
///

pub enum ArgsType {
	Info(InfoArgs),
	List(ListArgs),
	Unpack(UnpackArgs),
}

pub fn get_toc_seed(args: ArgsType, path2: Option<PathBuf>) -> Option<u32> {
	let (toc_seed, pe_path, mut path) = match args {
		ArgsType::Info(args) => (args.toc_seed, args.pe_path, Some(args.file_path)),
		ArgsType::List(args) => (args.toc_seed, args.pe_path, Some(args.file_path)),
		ArgsType::Unpack(args) => (args.toc_seed, args.pe_path, None),
	};

	if path2.is_some() {
		// Canonicalized path
		// this is really duct taped so i think i'll just make the path an arg and skip reading
		// it from args altogether
		path = path2;
	}

	if let Some(toc_seed) = toc_seed {
		return Some(toc_seed)
	}

	if let Some(pe_path) = pe_path {
		return Cs2Pe::from_path(&pe_path).ok().map(|pe| pe.toc_seed);
	}

	let target_file = path?;
	let parent_dir = target_file.parent()?;
	let pe_file = find_cs2_pe(parent_dir)?;

	Cs2Pe::from_path(&pe_file).ok().map(|pe| pe.toc_seed)
}
