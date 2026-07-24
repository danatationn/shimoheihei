use std::{fs::File, io::{self, Read, Seek}, path::Path};

use thiserror::Error;

///
/// STUB
/// Undocumented file. Related to gfx related stuff I guess??
/// 

pub struct Kx2 {}

#[derive(Error, Debug)]
pub enum Kx2Error {
	#[error("Io error: {0}")]
	IoError(#[from] io::Error),
	#[error("Invalid signature! (expected `KX2\\0`, found `{0:?}`)")]
	InvalidSignature([u8; 4]),
}

impl Kx2 {
	pub fn from_path<P: AsRef<Path>>(path: &P) -> Result<Self, Kx2Error> {
		let mut file = File::open(path)?;

		Self::from_reader(&mut file)
	}

	pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, Kx2Error> {
		let mut magic_buf = [0u8; 4];
		reader.read_exact(&mut magic_buf)?;
		if magic_buf != *b"KX2\0" {
			return Err(Kx2Error::InvalidSignature(magic_buf))
		}

		Ok(Self {})
	}
}
