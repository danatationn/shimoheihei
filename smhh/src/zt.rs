use std::{collections::HashMap, ffi::OsString, fs::File, io::{Read, Seek, SeekFrom}, path::Path};
use encoding_rs::SHIFT_JIS;
use flate2::read::ZlibDecoder;
use thiserror::Error;

use crate::ReadExt;

/// ZT Package

pub struct Zt {
	pub entries: HashMap<u64, ZtEntry>,
	pub root_nodes: Vec<ZtNode>,
}

#[derive(Error, Debug)]
pub enum ZtError {
	#[error("Io error: {0}")]
	IoError(#[from] std::io::Error),
	#[error("The file uses the wrong extension! `{0:?}` (supposed to be .zt)")]
	InvalidExtension(OsString),
	#[error("Invalid attribute: {0} (supposed to be 0/1)")]
	InvalidAttribute(u32),
}

impl Zt {
	pub fn from_path(path: &Path) -> Result<Self, ZtError> {
		let ext = path.extension().unwrap_or_default();
		if ext != "zt" {
			return Err(ZtError::InvalidExtension(ext.to_owned()))
		}

		let mut file = File::open(path)?;
		Self::from_reader(&mut file)
	}

	pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, ZtError> {
		let mut entries: HashMap<u64, ZtEntry> = HashMap::new();

		loop {
			let entry = match ZtEntry::from_reader(reader) {
				Ok(entry) => entry,
				Err(ZtError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
					break;
				},
				Err(e) => return Err(e),
			};
			entries.insert(entry.stream_pos, entry);
		}

		let root_nodes = Self::build_chain(0, &entries)?;

		Ok(Self { entries, root_nodes })
	}

	fn build_chain(offset: u64, entries: &HashMap<u64, ZtEntry>) -> Result<Vec<ZtNode>, ZtError> {
		let mut nodes: Vec<ZtNode> = Vec::new();
		let mut current = offset;

		loop {
			let Some(entry) = entries.get(&current) else {
				break;
			};

			let node = if entry.attributes == 1 {
				let children_offset = entry.stream_pos + entry.offset_children as u64;
				let children = Self::build_chain(children_offset, entries)?;
				ZtNode::Directory { name: entry.file_name.clone(), children }
			} else {
				ZtNode::File { name: entry.file_name.clone(), entry: entry.clone() }
			};
			nodes.push(node);

			if entry.offset_next == 0 {
				break;
			}
			current = entry.stream_pos + entry.offset_next as u64;
		}

		Ok(nodes)
	}
}

#[derive(Clone)]
pub struct ZtEntry {
	pub stream_pos: u64,
	pub offset_next: u32,
	pub offset_children: u32,
	pub length: u32,
	pub attributes: u32,
	pub file_name: String,
	pub compressed_size: u32,
	pub decompressed_size: u32,
}

impl ZtEntry {
	pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, ZtError> {
		// file entry header
		let stream_pos = reader.stream_position()?;
		let offset_next = reader.read_le::<u32>()?;
		let offset_children = reader.read_le::<u32>()?;
		let length = reader.read_le::<u32>()?;

		// file entry
		let attributes = reader.read_le::<u32>()?;
		if attributes > 1 {
			return Err(ZtError::InvalidAttribute(attributes))
		}
		let mut file_name_buf = vec![0u8; 260];
		reader.read_exact(&mut file_name_buf)?;
		if let Some(pos) = file_name_buf.iter().position(|&byte| byte == 0) {
			file_name_buf.truncate(pos);
		}
		let file_name = SHIFT_JIS.decode(&file_name_buf).0.into_owned();

		let compressed_size = reader.read_le::<u32>()?;
		let decompressed_size = reader.read_le::<u32>()?;

		reader.seek(std::io::SeekFrom::Current(compressed_size as i64))?;

		Ok(Self { stream_pos, offset_next, offset_children, length, attributes, file_name, compressed_size, decompressed_size })
	}

	pub fn extract<R: Read + Seek>(&self, reader: &mut R) -> Result<Vec<u8>, ZtError> {
		reader.seek(SeekFrom::Start(self.stream_pos + 4 * 6 + 260))?;   // header size
		if &self.decompressed_size == &0 {
			return Ok(Vec::new())
		}

		let mut decoder = ZlibDecoder::new(reader);
		let mut file_data = vec![0u8; self.decompressed_size as usize];
		decoder.read_exact(&mut file_data)?;

		Ok(file_data)
	}
}

pub enum ZtNode {
	File {
		name: String,
		entry: ZtEntry,
	},
	Directory {
		name: String,
		children: Vec<ZtNode>,
	},
}
