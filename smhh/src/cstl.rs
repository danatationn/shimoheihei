use std::{fs::File, io::{self, Read, Seek}, path::Path};

use thiserror::Error;

///
/// STUB
/// CatScene Localization
///

pub struct Cstl {}

#[derive(Error, Debug)]
pub enum CstlError {
	#[error("Io error: {0}")]
	IoError(#[from] io::Error),
}

impl Cstl {
	pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, CstlError> {
		let mut file = File::open(&path)?;

		Self::from_reader(&mut file)
	}

	pub fn from_reader<R: Read + Seek>(_reader: &mut R) -> Result<Self, CstlError> {
		todo!()
	}
}
