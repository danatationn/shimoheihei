use std::{fmt, fs::File, io::{BufRead, Cursor, Read, Seek, SeekFrom}, path::Path};

use thiserror::Error;
use flate2::read::ZlibDecoder;

use crate::ReadExt;

/// CatScene

pub struct Cst {
	pub script_name: String,
	pub compressed_size: u32,
	pub decompressed_size: u32,
	pub raw_script_data: Vec<u8>,
	pub script_data: Vec<u8>,  // capacity(decompressed_size)

	pub script_len: u32,
	pub input_count: u32,
	pub offset_table_offset: u32,
	pub string_table_offset: u32,
	pub entry_count: u32,
}

#[derive(Error, Debug)]
pub enum CstError {
	#[error("Io error: {0}")]
	IoError(#[from] std::io::Error),
	#[error("Wrong file signature: {0} != `CatScene`")]
	WrongSignature(String),
	#[error("Unexpected decompressed size: {0}")]
	WrongSize(usize),
	#[error("Unknown line type: {0}")]
	UnknownLineType(u8),
}

impl Cst {
	pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, CstError> {
		let mut file = File::open(&path)?;
		let file_stem = path.as_ref()
			.file_stem()
			.map(|s| s.to_string_lossy().into_owned())
			.unwrap();
		Self::from_reader(&mut file, file_stem)
	}

	pub fn from_reader<R: Read>(reader: &mut R, script_name: String) -> Result<Self, CstError> {
		let mut magic = [0u8; 8];
		reader.read_exact(&mut magic)?;
		if magic != *b"CatScene" {
			return Err(CstError::WrongSignature(String::from_utf8_lossy(&magic).into_owned()))
		}

		let compressed_size = reader.read_le::<u32>()?;
		let decompressed_size = reader.read_le::<u32>()?;

		let raw_script_data = Vec::new();
		let mut script_data = Vec::new();

		if compressed_size == 0 {
			// not compressed
			script_data = vec![0u8; decompressed_size as usize];
			reader.read_exact(&mut script_data)?;
		} else {
			let mut raw_script_data = vec![0u8; compressed_size as usize];
			reader.read_exact(&mut raw_script_data)?;

			let mut decoder = ZlibDecoder::new(&raw_script_data[..]);
			decoder.read_to_end(&mut script_data)?;
		}

		if script_data.len() != decompressed_size as usize {
			return Err(CstError::WrongSize(script_data.len()))
		}

		let mut cursor = Cursor::new(&script_data);
		// script_len excludes the script header (4 u32s)
		let script_len = cursor.read_le::<u32>()?;
		if script_len + 4*4 != decompressed_size {
			return Err(CstError::WrongSize(script_len as usize + 4*4))
		}

		let input_count = cursor.read_le::<u32>()?;
		let offset_table_offset = cursor.read_le::<u32>()?;
		let string_table_offset = cursor.read_le::<u32>()?;
		let entry_count = (string_table_offset - offset_table_offset) / 4;

		Ok(Self {
			script_name, compressed_size, decompressed_size, raw_script_data, script_data,
			script_len, input_count, offset_table_offset, string_table_offset, entry_count
		})
	}

	pub fn decomp_script(&self) -> Result<Vec<u8>, CstError> {
		let mut decompiled_script: Vec::<u8> = Vec::new();

		let mut cursor = Cursor::new(&self.script_data);
		cursor.seek(SeekFrom::Start(self.string_table_offset as u64 + 4*4))?;

		for _ in 0..self.entry_count {
			cursor.seek(SeekFrom::Current(1))?; // reserved byte
			let raw_line_type = cursor.read_byte()?;
			let line_type = CstLineType::try_from(raw_line_type)?;

			let mut content: Vec::<u8> = Vec::new();
			cursor.read_until(0x00, &mut content)?;

			if content.last() == Some(&b'\0') {
				content.pop();
			}

			match line_type {
				CstLineType::Input => {
					if decompiled_script.last() == Some(&b'\n') {
						decompiled_script.pop();
					}
					decompiled_script.extend_from_slice(b"\\@\n");
					continue;
				}
				CstLineType::Page => {
					if decompiled_script.last() == Some(&b'\n') {
						decompiled_script.pop();
					}
					decompiled_script.extend_from_slice(b"\\n\n");
					continue;
				}
				_ => {}
			}

			if content.trim_ascii().first().map_or(false, |b| b.is_ascii_alphanumeric() && line_type == CstLineType::Message) {
				content.insert(0, b'\\');
			}

			content.insert(0, b'\t');

			decompiled_script.extend(content);

			if line_type != CstLineType::Name {
				// i think it's safe to assume the next line will always be a message ?
				decompiled_script.push(b'\n');
			}
		}

		decompiled_script.splice(0..0, format!("#{}\n", self.script_name).into_bytes());
		Ok(decompiled_script)
	}

	/// to be implemented
	pub fn comp_script() -> () {
		()
	}
}

impl fmt::Debug for Cst {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("Cst")
			.field("compressed_size", &self.compressed_size)
			.field("decompressed_size", &self.decompressed_size)
			.finish()
	}
}

pub struct CstLine {
	pub line_type: CstLineType,
	pub content: String,
}

#[derive(PartialEq, Eq)]
pub enum CstLineType {
	Input		= 0x02,
	Page		= 0x03,
	Message		= 0x20,
	Name		= 0x21,
	Command		= 0x30,
	ScriptName	= 0xF0,  // debug
	LineNumber	= 0xF1,  // debug
}

impl CstLineType {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Input => "Input",
			Self::Page => "Page",
			Self::Message => "Message",
			Self::Name => "Name",
			Self::Command => "Command",
			Self::ScriptName => "ScriptName",
			Self::LineNumber => "LineNumber",
		}
	}
}

impl TryFrom<u8> for CstLineType {
	type Error = CstError;

	fn try_from(value: u8) -> Result<CstLineType, Self::Error> {
		match value {
			0x02 => Ok(Self::Input),
			0x03 => Ok(Self::Page),
			0x20 => Ok(Self::Message),
			0x21 => Ok(Self::Name),
			0x30 => Ok(Self::Command),
			0xF0 => Ok(Self::ScriptName),
			0xF1 => Ok(Self::LineNumber),
			other => Err(CstError::UnknownLineType(other))
		}
	}
}
