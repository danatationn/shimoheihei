use std::{fmt, fs::File, io::{Read, Seek, SeekFrom}, path::Path};
use thiserror::Error;

use crate::ReadExt;

pub struct Hgx {
	pub signature: [u8; 4],
	pub version: u8,
	pub frames: Vec<HgxFrame>
}

#[derive(Error, Debug)]
pub enum HgxError {
	#[error("Io error: {0}")]
	IoError(#[from] std::io::Error),

	#[error("Invalid file signature!")]
	InvalidSignature(),
	#[error("Frame error: {0}")]
	FrameError(#[from] HgxFrameError),
	#[error("Tag error: {0}")]
	TagError(#[from] HgxTagError)
}

impl Hgx {
	pub fn from<P: AsRef<Path>>(path: &P) -> Result<Self, HgxError> {
		let mut file = File::open(path)?;

		let mut signature = [0u8; 4];
		file.read_exact(&mut signature)?;
		if !signature.starts_with(b"HG-") {
			return Err(HgxError::InvalidSignature())
		}
		if signature[3] != b'2' && signature[3] != b'3' {
			return Err(HgxError::InvalidSignature())
		}

		// ignoring header size (it's always 12)
		file.seek(SeekFrom::Start(4*2))?;

		let mut version_pair = [0u8; 2];
		file.read_exact(&mut version_pair)?;
		let version = version_pair[0] * 10 + version_pair[1];

		let mut pos = 4*3;
		let mut frames = Vec::new();
		loop {
			let frame = HgxFrame::from(&mut file, pos)?;
			pos = frame.next_pos as u64;
			frames.push(frame);
			if pos == 0 {
				break
			}
		}

		Ok(Hgx { signature, version, frames })
	}
}

impl fmt::Display for Hgx {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f, "{}, ver. {}",
			String::from_utf8_lossy(&self.signature),
			&self.version,
		)?;

		if *&self.frames.len() == 1 {
			write!(f, " (1 frame)")?;
		} else {
			write!(f, " ({} frames)", &self.frames.len())?;
		}

		Ok(())
	}
}

pub struct HgxFrame {
	pub next_pos: u32,
	pub id: u32,
	pub tags: Vec<HgxTag>
}

#[derive(Error, Debug)]
pub enum HgxFrameError {
	#[error("Io error: {0}")]
	IoError(#[from] std::io::Error),
	#[error("Tag error: {0}")]
	TagError(#[from] HgxTagError)
}

impl HgxFrame {
	pub fn from<F: Read + Seek>(file: &mut F, pos: u64) -> Result<Self, HgxFrameError> {
		file.seek(SeekFrom::Start(pos))?;
		let next_pos = file.read_le::<u32>()?;
		let id = file.read_le::<u32>()?;
		let mut tags: Vec<HgxTag> = Vec::new();

		loop {
			let mut name_bytes = [0u8; 8];
			file.read_exact(&mut name_bytes)?;
			let next_offset = file.read_le::<u32>()?;
			let length = file.read_le::<u32>()?;
			let tag = HgxTag::from(file, name_bytes, length)?;
			tags.push(tag);
			if next_offset == 0 {
				break
			}
		}

		Ok(HgxFrame { next_pos, id, tags })
	}
}

impl fmt::Display for HgxFrame {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Frame ID: {}, {} tags", &self.id, &self.tags.len())?;

		Ok(())
	}
}

impl fmt::Debug for HgxFrame {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("HgxFrame")
    		.field("next_pos", &self.next_pos)
    		.field("id", &self.id)
      		.field("tag_count", &self.tags.len())
        	.finish()
	}
}

#[derive(Error, Debug)]
pub enum HgxTagError {
	#[error("Io error: {0}")]
	IoError(#[from] std::io::Error),
	#[error("Unknown tag type: {0}")]
	UnknownType(String)
}

pub enum HgxTag {
	StdInfo(StdInfoTag),
	Img(ImgTag),
	ImgJpg(ImgJpgTag),
	ImgAl(ImgAlTag),
	ImgJpa(),
	ImgWbp(ImgWbpTag),
	Ats(AtsTag),
	CpType(CpTypeTag),
	ImgMode(ImgModeTag)
}

impl HgxTag {
	pub fn from<F: Read + Seek>(file: &mut F, name_bytes: [u8; 8], length: u32) -> Result<Self, HgxTagError> {
		if name_bytes.starts_with(b"img") && !name_bytes.starts_with(b"img_") {
			return Ok(Self::Img(ImgTag::parse(file, length)?))
		}
		if name_bytes.starts_with(b"ats") {
			return Ok(Self::Ats(AtsTag::parse(file)?))
		}

		match &name_bytes {
			b"stdinfo\0" => Ok(Self::StdInfo(StdInfoTag::parse(file)?)),
			b"img_jpg\0" => Ok(Self::ImgJpg(ImgJpgTag::parse(file, length)?)),
			b"img_al\0\0"=> Ok(Self::ImgAl(ImgAlTag::parse(file)?)),
			b"img_jpa\0" => Ok(Self::ImgJpa()),
			b"img_wbp\0" => Ok(Self::ImgWbp(ImgWbpTag::parse(file, length)?)),
			b"cptype\0\0"=> Ok(Self::CpType(CpTypeTag::parse(file)?)),
			b"imgmode\0" => Ok(Self::ImgMode(ImgModeTag::parse(file)?)),
			_ => return Err(HgxTagError::UnknownType(String::from_utf8_lossy(&name_bytes).to_string()))
		}
	}
}

impl fmt::Display for HgxTag {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			HgxTag::StdInfo(std_info_tag) => write!(
				f, "Info: {}x{}, {}bit",
				std_info_tag.width,
				std_info_tag.height,
				std_info_tag.bit_depth
			),
			HgxTag::Img(img_tag) => write!(
				f, "Image: {} bytes compressed, {} bytes uncompressed",
				img_tag.compressed_data_length + img_tag.compressed_cmd_length,
				img_tag.decompressed_data_length + img_tag.decompressed_cmd_length
			),
			HgxTag::ImgJpg(img_jpg_tag) => write!(
				f, "JPEG image: {} bytes",
				img_jpg_tag.length
			),
			HgxTag::ImgAl(img_al_tag) => write!(
				f, "Alpha mask: {} bytes compressed, {} bytes uncompressed",
				img_al_tag.compressed_length,
				img_al_tag.decompressed_length
			),
			HgxTag::ImgWbp(img_wbp_tag) => write!(
				f, "WebP image: {} bytes",
				img_wbp_tag.length
			),
			HgxTag::Ats(ats_tag) => write!(
				f, "Attribute info: {}x{}, {}w {}h",
				ats_tag.x,
				ats_tag.y,
				ats_tag.width,
				ats_tag.height
			),
			HgxTag::CpType(cp_type_tag) => write!(
				f, "CG page type: {}",
				cp_type_tag.cp_type
			),
			HgxTag::ImgMode(img_mode_tag) => write!(
				f, "Image mode: {}",
				img_mode_tag.mode
			),
			_ => write!(f, "")
		}
	}
}

// impl fmt::Debug for HgxTag  todo: I CAN'T BE BOTHERED

pub struct StdInfoTag {
	pub width: u32,
	pub height: u32,
	pub bit_depth: u32,
	pub offset_x: i32,
	pub offset_y: i32,
	pub total_width: u32,
	pub total_height: u32,
	pub is_transparent: bool,
	pub base_x: i32,
	pub base_y: i32
}

impl StdInfoTag {
	fn parse<F: Read>(file: &mut F) -> Result<Self, HgxTagError> {
		Ok(Self {
			width: file.read_le::<u32>()?,
			height: file.read_le::<u32>()?,
			bit_depth: file.read_le::<u32>()?,
			offset_x: file.read_le::<i32>()?,
			offset_y: file.read_le::<i32>()?,
			total_width: file.read_le::<u32>()?,
			total_height: file.read_le::<u32>()?,
			is_transparent : file.read_le::<u32>()? != 0,
			base_x: file.read_le::<i32>()?,
			base_y: file.read_le::<i32>()?
		})
	}
}

pub struct ImgTag {
	pub slice_start: u32,
	pub slice_length: u32,
	pub compressed_data_length: u32,
	pub decompressed_data_length: u32,
	pub compressed_cmd_length: u32,
	pub decompressed_cmd_length: u32,
	pub data_pos: u64,
}

impl ImgTag {
	fn parse<F: Read + Seek>(file: &mut F, length: u32) -> Result<Self, HgxTagError> {
		let pos = file.stream_position()?;
		let slice_start = file.read_le::<u32>()?;
		let slice_length = file.read_le::<u32>()?;
		let compressed_data_length = file.read_le::<u32>()?;
		let decompressed_data_length = file.read_le::<u32>()?;
		let compressed_cmd_length = file.read_le::<u32>()?;
		let decompressed_cmd_length = file.read_le::<u32>()?;
		file.seek(SeekFrom::Start(pos + length as u64))?;

		Ok(Self { slice_start, slice_length, compressed_data_length, decompressed_data_length, compressed_cmd_length, decompressed_cmd_length, data_pos: pos + 4*6 })
	}
}

pub struct ImgJpgTag {
	pub length: u32,
	pub data_pos: u64
}

impl ImgJpgTag {
	fn parse<F: Read + Seek>(file: &mut F, length: u32) -> Result<Self, HgxTagError> {
		let data_pos = file.stream_position()?;
		file.seek_relative(length as i64)?;
		Ok(Self { length, data_pos })
	}
}

pub struct ImgAlTag {
	pub compressed_length: u32,
	pub decompressed_length: u32,
	pub data_pos: u64,
}

impl ImgAlTag {
	fn parse<F: Read + Seek>(file: &mut F) -> Result<Self, HgxTagError> {
		let pos = file.stream_position()?;
		let compressed_length = file.read_le::<u32>()?;
		let decompressed_length = file.read_le::<u32>()?;
		file.seek(SeekFrom::Start(pos + compressed_length as u64))?;

		Ok(Self { compressed_length, decompressed_length, data_pos: pos + 4*2 })
	}
}

pub struct ImgWbpTag {
	pub length: u32,
	pub data_pos: u64
}

impl ImgWbpTag {
	fn parse<F: Read + Seek>(file: &mut F, length: u32) -> Result<Self, HgxTagError> {
		let data_pos = file.stream_position()?;
		file.seek_relative(length as i64)?;
		Ok(Self { length, data_pos })
	}
}

pub struct AtsTag {
	pub x: i32,
	pub y: i32,
	pub width: u32,
	pub height: u32
	// pub color
}

impl AtsTag {
	fn parse<F: Read>(file: &mut F) -> Result<Self, HgxTagError> {
		Ok(Self {
			x: file.read_le::<i32>()?,
			y: file.read_le::<i32>()?,
			width: file.read_le::<u32>()?,
			height: file.read_le::<u32>()?
			// color:
		})
	}
}

pub struct CpTypeTag {
	pub cp_type: u32
}

impl CpTypeTag {
	fn parse<F: Read>(file: &mut F) -> Result<Self, HgxTagError> {
		Ok(Self { cp_type: file.read_le::<u32>()? })
	}
}

pub struct ImgModeTag {
	pub mode: bool
}

impl ImgModeTag {
	fn parse<F: Read>(file: &mut F) -> Result<Self, HgxTagError> {
		Ok(Self { mode: file.read_le::<u32>()? != 0 })
	}
}
