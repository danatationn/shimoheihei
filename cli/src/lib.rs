use std::{path::PathBuf, sync::LazyLock};
use clap::{Args, Parser, Subcommand};
use indicatif::ProgressStyle;
use smhh::{anm::Anm, fes::Fes, hgx::Hgx, kif::Kif, pe::Cs2Pe};

pub mod info;
pub mod pack;
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
    #[arg(short, long, default_value = "png")]
    convert_hgx_to: String,
    #[arg(short, long)]
    raw: bool,
    #[arg(short, long)]
    skip_existing_files: bool,
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
	ProgressStyle::with_template("{msg} {wide_bar} {pos}/{len}")
		.expect("Invalid template")
		.progress_chars("#=-")
});

///
/// helper functions
///

pub enum Cs2FileType {
	Pe(Cs2Pe),
	Kif(Kif),
	Hgx(Hgx),
	// Cst(Cst),
	// Cstl(Cstl),
	Anm(Anm),
	Fes(Fes),
	// Zt(Zt),
}

pub enum ArgsType {
	Info(InfoArgs),
	Unpack(UnpackArgs),
}

pub fn determine_path(src_path: &PathBuf, dest_path: &Option<PathBuf>) -> PathBuf {
	dest_path.as_ref().cloned().unwrap_or_else(|| src_path.to_owned())
}
