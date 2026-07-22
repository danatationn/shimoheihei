use std::{fmt, fs::File, io::{Read, Seek, SeekFrom}, path::Path};
use thiserror::Error;
use flate2::read::ZlibDecoder;

use crate::ReadExt;

/// Front End Script

pub struct Fes {
	pub compressed_size: u32,
	pub decompressed_size: u32,
	pub raw_script_text: Vec<u8>,
}

#[derive(Error, Debug)]
pub enum FesError {
	#[error("Io error: {0}")]
	IoError(#[from] std::io::Error),
	#[error("Invalid signature: {0:?}")]
	InvalidSignature([u8; 4]),
	#[error("Invalid size")]
	InvalidSize,
}

impl Fes {
	pub fn from_path<P: AsRef<Path>>(path: &P) -> Result<Self, FesError> {
		let mut file = File::open(path)?;

		Self::from_reader(&mut file)
	}

	pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, FesError> {
		let mut magic = [0u8; 4];
		reader.read_exact(&mut magic)?;
		if magic != *b"FES\0" {
			return Err(FesError::InvalidSignature(magic));
		}

		let compressed_size = reader.read_le::<u32>()?;
		let decompressed_size = reader.read_le::<u32>()?;
		reader.seek(SeekFrom::Current(4))?;  // reserved

		let mut raw_script_text = vec![0u8; compressed_size as usize];
		reader.read_exact(&mut raw_script_text)?;


		Ok(Self { compressed_size, decompressed_size, raw_script_text})
	}

	pub fn extract(&self) -> Result<Vec<u8>, FesError> {
		let mut decoder = ZlibDecoder::new(&self.raw_script_text[..]);
		let mut script_text = Vec::new();
		decoder.read_to_end(&mut script_text)?;

		if script_text.len() != self.decompressed_size as usize {
			return Err(FesError::InvalidSize)
		}

		Ok(script_text)
	}
}

impl fmt::Debug for Fes {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("Fes")
			.field("compressed_size", &self.compressed_size)
			.field("decompressed_size", &self.decompressed_size)
			.finish()
	}
}
