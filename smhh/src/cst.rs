use std::{fs::File, io::Read, path::Path};

use thiserror::Error;
use flate2::read::ZlibDecoder;

use crate::ReadExt;

/// CatScene

pub struct Cst {
	pub compressed_size: u32,
	pub decompressed_size: u32,
	pub raw_script_data: Vec<u8>,  // capacity(compressed_size)
	pub script_data: Vec<u8>,  // capacity(decompressed_size)
}

#[derive(Error, Debug)]
pub enum CstError {
	#[error("Io error: {0}")]
	IoError(#[from] std::io::Error),
	#[error("Wrong file signature: {0} != `CatScene`")]
	WrongSignature(String),
	#[error("Unexpected decompressed size: {0}")]
	WrongSize(usize),
}

impl Cst {
	/// needs to be the compiled .cst and not the .txt
	pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, CstError> {
		let mut file = File::open(path)?;
		Self::from_file(&mut file)
	}

	/// same as above
	pub fn from_file<F: Read>(file: &mut F) -> Result<Self, CstError> {
		let mut magic = [0u8; 8];
		file.read_exact(&mut magic)?;
		if magic != *b"CatScene" {
			return Err(CstError::WrongSignature(String::from_utf8_lossy(&magic).into_owned()))
		}

		let compressed_size = file.read_le::<u32>()?;
		let decompressed_size = file.read_le::<u32>()?;

		let raw_script_data = Vec::new();
		let mut script_data = Vec::new();

		if compressed_size == 0 {
			// not compressed
			script_data = vec![0u8; decompressed_size as usize];
			file.read_exact(&mut script_data)?;
		} else {
			let mut raw_script_data = vec![0u8; compressed_size as usize];
			file.read_exact(&mut raw_script_data)?;

			let mut decoder = ZlibDecoder::new(&raw_script_data[..]);
			decoder.read_to_end(&mut script_data)?;
		}

		if script_data.len() != decompressed_size as usize {
			return Err(CstError::WrongSize(script_data.len()))
		}

		Ok(Self { compressed_size, decompressed_size, raw_script_data, script_data })
	}
}
