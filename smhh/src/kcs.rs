use std::{fs::File, io::{self, Read, Seek}, path::Path};

use flate2::read::ZlibDecoder;
use thiserror::Error;

use crate::ReadExt;

///
/// STUB
/// Undocumented file. (kclib script ?)
///

pub struct Kcs {
	pub header_bytes: Vec::<u8>,
	pub data: Vec::<u8>,
}

#[derive(Error, Debug)]
pub enum KcsError {
	#[error("Io error: {0}")]
	IoError(#[from] io::Error),
	#[error("Wrong file signature! (expected `KCS\\0`, got `{0:?}`)")]
	InvalidSignature([u8; 4]),
}

impl Kcs {
	pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, KcsError> {
		let mut file = File::open(path)?;

		Self::from_reader(&mut file)
	}

	pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, KcsError> {
		let mut magic_buf = [0u8; 4];
		reader.read_exact(&mut magic_buf)?;
		if magic_buf != *b"KCS\0" {
			return Err(KcsError::InvalidSignature(magic_buf))
		}

		dbg!("UNKNOWN NUMBERS INCOMING!!!");
		for i in 0..12 {
			let number = reader.read_le::<u32>()?;
			dbg!(i, number);
		}

		reader.seek(io::SeekFrom::Start(13*4))?;

		let mut zlib_data: Vec::<u8> = Vec::new();
		reader.read_to_end(&mut zlib_data)?;

		let mut decoder = ZlibDecoder::new(&zlib_data[..]);
		let mut data: Vec::<u8> = Vec::new();
		decoder.read_to_end(&mut data)?;

		Ok(Self { header_bytes: data.clone(), data })
	}
}
