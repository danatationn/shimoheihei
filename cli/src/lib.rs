use std::{path::PathBuf, sync::LazyLock};
use clap::{Args, Parser, Subcommand};
use indicatif::ProgressStyle;

pub mod info;
pub mod unpack;

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
pub struct UnpackArgs {
    paths: Vec<PathBuf>,
    #[arg(short, long)]
    dest_path: Option<PathBuf>,
    #[arg(long)]
    pe_path: Option<PathBuf>,
    #[arg(long)]
    toc_seed: Option<u32>,
    #[arg(short, long, default_value = "jpg")]
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
	Unpack(UnpackArgs),
}
