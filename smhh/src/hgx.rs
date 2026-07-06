use std::{fmt, fs::File, io::{Cursor, Read, Seek, SeekFrom}, path::Path};
use flate2::read::ZlibDecoder;
use thiserror::Error;
use image::{self, Rgb, Rgba};

use crate::{ReadExt, crypt::unpack_value};
use crate::crypt::BitReader;

/// Harvard Grapics 2/3

pub struct Hgx {
	pub version: u8,
	pub frames: Vec<HgxFrame>
}

#[derive(Error, Debug)]
pub enum HgxError {
	#[error("Io error: {0}")]
	IoError(#[from] std::io::Error),

	#[error("Invalid file signature!")]
	InvalidSignature(),
	#[error("HG2 is not supported")]
	Hg2NotSupported,
	#[error("Frame error: {0}")]
	FrameError(#[from] HgxFrameError),
	#[error("Tag error: {0}")]
	TagError(#[from] HgxTagError)
}

impl Hgx {
	pub fn from_path<P: AsRef<Path>>(path: &P) -> Result<Self, HgxError> {
		let mut file = File::open(path)?;

		Self::from_reader(&mut file)
	}

	pub fn from_reader<R: Read+ Seek>(reader: &mut R) -> Result<Self, HgxError> {
		let mut magic = [0u8; 4];
		reader.read_exact(&mut magic)?;
		if !magic.starts_with(b"HG-") {
			return Err(HgxError::InvalidSignature())
		}
		if magic[3] != b'2' && magic[3] != b'3' {
			return Err(HgxError::InvalidSignature())
		}
		if magic[3] == b'2' {
			return Err(HgxError::Hg2NotSupported)
		}

		reader.seek(SeekFrom::Start(4*2))?;

		let mut version_pair = [0u8; 2];
		reader.read_exact(&mut version_pair)?;
		let version = version_pair[0] * 10 + version_pair[1];

		let mut pos = 4*3;
		let mut frames = Vec::new();
		loop {
			let frame = HgxFrame::from(reader, pos)?;
			pos += frame.next_pos as u64;
			let next_pos = frame.next_pos;
			frames.push(frame);
			if next_pos == 0 {
				break
			}
		}

		Ok(Hgx { version, frames })
	}
}

impl fmt::Display for Hgx {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f, "HGX, ver. {}",
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

	#[error("The image info was not read first!")]
	UnexpectedTagAtStart(),
	#[error("Nor CG page type or image mode were read last!")]
	UnexpectedTagAtEnd(),
	#[error("Tag error: {0}")]
	TagError(#[from] HgxTagError)
}

impl HgxFrame {
	pub fn from<R: Read + Seek>(reader: &mut R, pos: u64) -> Result<Self, HgxFrameError> {
		reader.seek(SeekFrom::Start(pos))?;
		let next_pos = reader.read_le::<u32>()?;
		let id = reader.read_le::<u32>()?;
		let mut tags: Vec<HgxTag> = Vec::new();

		loop {
			let mut name_bytes = [0u8; 8];
			reader.read_exact(&mut name_bytes)?;
			let next_offset = reader.read_le::<u32>()?;
			let length = reader.read_le::<u32>()?;
			let tag = HgxTag::from(reader, name_bytes, length)?;
			tags.push(tag);
			if next_offset == 0 {
				match tags.last() {
					Some(HgxTag::CpType(_)) => break,
					Some(HgxTag::ImgMode(_)) => break,
					_ => return Err(HgxFrameError::UnexpectedTagAtEnd())
				}
			}
			if tags.len() == 1 {
				match tags.first() {
					Some(HgxTag::StdInfo(_)) => continue,
					_ => return Err(HgxFrameError::UnexpectedTagAtStart())
				}
			}
		};

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
	#[error("Image error: {0}")]
	ImageError(#[from] image::ImageError),

	#[error("Unknown tag type: {0}")]
	UnknownType(String),
	#[error("Invalid Elias Gamma data: {0}")]
	InvalidGamma(String),
	#[error("Invalid RLE data")]
	InvalidRle(),
	#[error("Invalid Delta data: {0}")]
	InvalidDelta(String),
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
	pub fn from<R: Read + Seek>(reader: &mut R, name_bytes: [u8; 8], length: u32) -> Result<Self, HgxTagError> {
		let name: String = name_bytes.iter()
		    .take_while(|&&b| b != 0)
		    .map(|&b| b as char)
			.collect();

		// TODO make this cleaner
		if name.starts_with("img") && !name.starts_with("img_") && !name.starts_with("imgmode") {
			return Ok(Self::Img(ImgTag::parse(reader, length)?))
		}
		if name.starts_with("ats") {
			return Ok(Self::Ats(AtsTag::parse(reader)?))
		}

		match name.as_str() {
			"stdinfo"	=> Ok(Self::StdInfo(StdInfoTag::parse(reader)?)),
			"img_jpg"	=> Ok(Self::ImgJpg(ImgJpgTag::parse(reader, length)?)),
			"img_al"	=> Ok(Self::ImgAl(ImgAlTag::parse(reader)?)),
			"img_jpa"	=> Ok(Self::ImgJpa()),
			"img_wbp"	=> Ok(Self::ImgWbp(ImgWbpTag::parse(reader, length)?)),
			"cptype"	=> Ok(Self::CpType(CpTypeTag::parse(reader)?)),
			"imgmode"	=> Ok(Self::ImgMode(ImgModeTag::parse(reader)?)),
			_ 			=> return Err(HgxTagError::UnknownType(String::from_utf8_lossy(&name_bytes).to_string()))
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

// impl fmt::Debug for HgxTag {
	// fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//
	// }
// }

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
	fn parse<R: Read>(reader: &mut R) -> Result<Self, HgxTagError> {
		Ok(Self {
			width: reader.read_le::<u32>()?,
			height: reader.read_le::<u32>()?,
			bit_depth: reader.read_le::<u32>()?,
			offset_x: reader.read_le::<i32>()?,
			offset_y: reader.read_le::<i32>()?,
			total_width: reader.read_le::<u32>()?,
			total_height: reader.read_le::<u32>()?,
			is_transparent : reader.read_le::<u32>()? != 0,
			base_x: reader.read_le::<i32>()?,
			base_y: reader.read_le::<i32>()?
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
	fn parse<R: Read + Seek>(reader: &mut R, length: u32) -> Result<Self, HgxTagError> {
		let pos = reader.stream_position()?;
		let slice_start = reader.read_le::<u32>()?;
		let slice_length = reader.read_le::<u32>()?;
		let compressed_data_length = reader.read_le::<u32>()?;
		let decompressed_data_length = reader.read_le::<u32>()?;
		let compressed_cmd_length = reader.read_le::<u32>()?;
		let decompressed_cmd_length = reader.read_le::<u32>()?;
		reader.seek(SeekFrom::Start(pos + length as u64))?;

		Ok(Self { slice_start, slice_length, compressed_data_length, decompressed_data_length, compressed_cmd_length, decompressed_cmd_length, data_pos: pos + 4*6 })
	}

	pub fn read_raw<R: Read + Seek>(&self, reader: &mut R, std_info_tag: &StdInfoTag) -> Result<Vec<u8>, HgxTagError> {
		reader.seek(SeekFrom::Start(self.data_pos))?;
		let mut inflated_data_buf = vec![0u8; self.compressed_data_length as usize];
		reader.read_exact(&mut inflated_data_buf)?;
		let mut inflated_cmd_buf = vec![0u8; self.compressed_cmd_length as usize];
		reader.read_exact(&mut inflated_cmd_buf)?;

		let mut data_buf = vec![0u8; self.decompressed_data_length as usize];
		let mut cmd_buf = vec![0u8; self.decompressed_cmd_length as usize];
		let mut decoder = ZlibDecoder::new(&inflated_data_buf[..]);
		decoder.read_exact(&mut data_buf)?;
		decoder.reset(&inflated_cmd_buf[..]);
		decoder.read_exact(&mut cmd_buf)?;

		let unrle_buf = self.unrle_data(&cmd_buf, &data_buf)?;
		let delta_filter_buf = self.delta_filter(&unrle_buf, std_info_tag.width, std_info_tag.height, std_info_tag.bit_depth)?;

		Ok(delta_filter_buf)
	}

	pub fn convert<R: Read + Seek>(&self, reader: &mut R, std_info_tag: &StdInfoTag, format: image::ImageFormat) -> Result<Vec<u8>, HgxTagError> {
		let raw_data = self.read_raw(reader, std_info_tag).unwrap();
		let mut data = Cursor::new(Vec::new());

		if std_info_tag.bit_depth == 24 {
			let img_buffer = image::ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(std_info_tag.width, std_info_tag.height, raw_data).unwrap();
			let img = image::DynamicImage::ImageRgb8(img_buffer).flipv();
			img.write_to(&mut data, format)?;
		} else {
			let img_buffer = image::ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(std_info_tag.width, std_info_tag.height, raw_data).unwrap();
			let img = image::DynamicImage::ImageRgba8(img_buffer).flipv();
			img.write_to(&mut data, format)?;
		}

		Ok(data.into_inner())
	}

	fn unrle_data(&self, cmd_buf: &[u8], data_buf: &[u8]) -> Result<Vec<u8>, HgxTagError> {
		let mut bit_reader = BitReader::new(&cmd_buf);
		let mut copy_flag = bit_reader.read_bit();
		let buf_length = bit_reader.read_elias_gamma();
		if buf_length == 0 {
			return Err(HgxTagError::InvalidGamma("Buffer length is 0".to_string()))
		}

		let mut buf = Vec::with_capacity(buf_length);
		let mut data_pos = 0;
		let mut data_left = data_buf.len();
		let mut buf_left = buf_length;

		while buf_left > 0 {
			let chunk_length = bit_reader.read_elias_gamma();
			if chunk_length == 0 {
				return Err(HgxTagError::InvalidGamma("Chunk length is 0".to_string()))
			}
			if chunk_length > buf_left {
				return Err(HgxTagError::InvalidRle());
			}

			if copy_flag {
				if chunk_length > data_left {
					return Err(HgxTagError::InvalidRle())
				}
				buf.extend_from_slice(&data_buf[data_pos..data_pos + chunk_length]);
				data_pos += chunk_length;
				data_left -= chunk_length;
			} else {
				// let zeros = vec![0u8; chunk_length];
				// buf.extend_from_slice(&zeros);
				buf.extend(std::iter::repeat(0u8).take(chunk_length));
			}
			buf_left -= chunk_length;
			copy_flag = !copy_flag;
		}
		if buf.len() != (buf.len() / 4) * 4 {
			return Err(HgxTagError::InvalidRle())
		}

		Ok(buf)
	}

	fn delta_filter(&self, unrle_buf: &[u8], width: u32, height: u32, bit_depth: u32) -> Result<Vec<u8>, HgxTagError> {
		// split the image in 4 equal sections
		let sect_length = unrle_buf.len() / 4;
		let sect1 = &unrle_buf[0..sect_length];
		let sect2 = &unrle_buf[sect_length..sect_length*2];
		let sect3 = &unrle_buf[sect_length*2..sect_length*3];
		let sect4 = &unrle_buf[sect_length*3..sect_length*4];

		// bit-reconstruction lookup tables (format obfuscation)
		let mut table1 = [0u32; 256];
		let mut table2 = [0u32; 256];
		let mut table3 = [0u32; 256];
		let mut table4 = [0u32; 256];
		for i in 0..256 {
			// by masking and shifting the bits we are scattering the 4 pairs of bits into one byte
			let mut value = (i as u32) & 0b11000000 as u32;
			value <<= 6;
			value |= (i as u32) & 0b00110000;
			value <<= 6;
			value |= (i as u32) & 0b00001100;
			value <<= 6;
			value |= (i as u32) & 0b00000011;

			// the shifting here makes sure that data doesn't get reused from the wrong sect
			table4[i] = value << 0;
			table3[i] = value << 2;
			table2[i] = value << 4;
			table1[i] = value << 6;
		}

		let depth_bytes = bit_depth / 8;
		let stride = depth_bytes * width;
		let buf_length = (stride * height) as usize;
		let mut buf = vec![0u8; buf_length];

		let pixel_length = (width * height) as usize;
		let max_pixels = sect_length.min(pixel_length);

		let mut out_pos = 0;
		for i in 0..max_pixels {
			if out_pos+depth_bytes as usize > buf.len() {
				return Err(HgxTagError::InvalidDelta("Buffer overflow".to_string()))
			}
			// restores the original packed color values
			let value = table1[sect1[i] as usize] |
				table2[sect2[i] as usize] |
				table3[sect3[i] as usize] |
			 	table4[sect4[i] as usize];

			// in case image is 24bit
			let b0 = unpack_value((value >> 0) as u8);
			let b1 = unpack_value((value >> 8) as u8);
			let b2 = unpack_value((value >> 16) as u8);
			let b3 = unpack_value((value >> 24) as u8);

			if out_pos < buf.len() { buf[out_pos] = b0; }
			if out_pos + 1 < buf.len() { buf[out_pos + 1] = b1; }
			if out_pos + 2 < buf.len() { buf[out_pos + 2] = b2; }
			if out_pos + 3 < buf.len() { buf[out_pos + 3] = b3; }

			out_pos += 4;
		}

		// delta decompression
		// horizontal decoding
		for x in depth_bytes..stride {
			let x = x as usize;
			if x >= buf.len() {
				return Err(HgxTagError::InvalidDelta("Buffer overflow caused by image row".to_string()));
			}
			buf[x] = buf[x].wrapping_add(buf[x - depth_bytes as usize]);
		}

		// vertical decoding
		for y in 1..height {
			let line_start = y * stride;
			let prev_start = (y - 1) * stride;

			if (line_start + stride) as usize > buf.len() {
				return Err(HgxTagError::InvalidDelta("Buffer overflow caused by image rows".to_string()))
			}

			for x in 0..stride {
				buf[(line_start + x) as usize] = buf[(line_start + x) as usize].wrapping_add(buf[(prev_start + x) as usize]);
			}
		}

		// bgra to rgba
		for i in (0..buf.len()).step_by(depth_bytes as usize) {
			buf.swap(i, i + 2);
		}

		Ok(buf)
	}
}

pub struct ImgJpgTag {
	pub length: u32,
	pub data_pos: u64
}

impl ImgJpgTag {
	fn parse<R: Read + Seek>(reader: &mut R, length: u32) -> Result<Self, HgxTagError> {
		let data_pos = reader.stream_position()?;
		reader.seek_relative(length as i64)?;
		Ok(Self { length, data_pos })
	}

	pub fn read<R: Read + Seek>(&self, reader: &mut R) -> Result<Vec<u8>, HgxTagError> {
		reader.seek(SeekFrom::Start(self.data_pos))?;
		let mut buf = vec![0u8; self.length as usize];
		reader.read_exact(&mut buf)?;
		Ok(buf)
	}

	pub fn stream_read<R: Read + Seek>(&self, reader: &mut R, pos: u64, chunk_size: usize) -> Result<Vec<u8>, HgxTagError> {
		let start_pos = self.data_pos + pos;
		let remaining_bytes = (self.length as u64).saturating_sub(pos);
		let read_size = chunk_size.min(remaining_bytes as usize);

		if read_size == 0 {
			return Ok(Vec::new());
		}

		reader.seek(SeekFrom::Start(start_pos))?;
		let mut buf = vec![0u8; read_size];
		reader.read_exact(&mut buf)?;
		Ok(buf)
	}
}

pub struct ImgAlTag {
	pub compressed_length: u32,
	pub decompressed_length: u32,
	pub data_pos: u64,
}

impl ImgAlTag {
	fn parse<R: Read + Seek>(reader: &mut R) -> Result<Self, HgxTagError> {
		let pos = reader.stream_position()?;
		let compressed_length = reader.read_le::<u32>()?;
		let decompressed_length = reader.read_le::<u32>()?;
		reader.seek(SeekFrom::Start(pos + compressed_length as u64))?;

		Ok(Self { compressed_length, decompressed_length, data_pos: pos + 4*2 })
	}

	pub fn read_extract<R: Read + Seek>(&self, reader: &mut R) -> Result<Vec<u8>, HgxTagError> {
		let compressed_data = self.read(reader)?;
		let mut decoder = flate2::read::ZlibDecoder::new(&compressed_data[..]);
		let mut data = Vec::new();
		decoder.read_to_end(&mut data)?;
		Ok(data)
	}

	pub fn read<R: Read + Seek>(&self, reader: &mut R) -> Result<Vec<u8>, HgxTagError> {
		reader.seek(SeekFrom::Start(self.data_pos))?;
		let mut buf = vec![0u8; self.compressed_length as usize];
		reader.read_exact(&mut buf)?;
		Ok(buf)
	}
}

pub struct ImgWbpTag {
	pub length: u32,
	pub data_pos: u64
}

impl ImgWbpTag {
	fn parse<R: Read + Seek>(reader: &mut R, length: u32) -> Result<Self, HgxTagError> {
		let data_pos = reader.stream_position()?;
		reader.seek_relative(length as i64)?;
		Ok(Self { length, data_pos })
	}

	pub fn read<R: Read + Seek>(&self, reader: &mut R) -> Result<Vec<u8>, HgxTagError> {
		reader.seek(SeekFrom::Start(self.data_pos))?;
		let mut buf = vec![0u8; self.length as usize];
		reader.read_exact(&mut buf)?;
		Ok(buf)
	}

	pub fn stream_read<R: Read + Seek>(&self, reader: &mut R, pos: u64, chunk_size: usize) -> Result<Vec<u8>, HgxTagError> {
		let start_pos = self.data_pos + pos;
		let remaining_bytes = (self.length as u64).saturating_sub(pos);
		let read_size = chunk_size.min(remaining_bytes as usize);

		if read_size == 0 {
			return Ok(Vec::new());
		}

		reader.seek(SeekFrom::Start(start_pos))?;
		let mut buf = vec![0u8; read_size];
		reader.read_exact(&mut buf)?;
		Ok(buf)
	}
}

pub struct AtsTag {
	pub x: i32,
	pub y: i32,
	pub width: u32,
	pub height: u32,
	pub color: u32
}

impl AtsTag {
	fn parse<R: Read>(reader: &mut R) -> Result<Self, HgxTagError> {
		Ok(Self {
			x: reader.read_le::<i32>()?,
			y: reader.read_le::<i32>()?,
			width: reader.read_le::<u32>()?,
			height: reader.read_le::<u32>()?,
			color: reader.read_le::<u32>()?
		})
	}
}

pub struct CpTypeTag {
	pub cp_type: u32
}

impl CpTypeTag {
	fn parse<R: Read>(reader: &mut R) -> Result<Self, HgxTagError> {
		Ok(Self { cp_type: reader.read_le::<u32>()? })
	}
}

pub struct ImgModeTag {
	pub mode: bool
}

impl ImgModeTag {
	fn parse<R: Read>(reader: &mut R) -> Result<Self, HgxTagError> {
		Ok(Self { mode: reader.read_le::<u32>()? != 0 })
	}
}
