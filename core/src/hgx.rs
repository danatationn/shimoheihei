use std::{fmt, fs::File, io::{Cursor, Read, Seek, SeekFrom}, path::Path};
use flate2::read::ZlibDecoder;
use thiserror::Error;
use image::{self, Rgb, Rgba};

use crate::{ReadExt, crypt::unpack_value};
use crate::crypt::BitReader;

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

		file.seek(SeekFrom::Start(4*2))?;

		let mut version_pair = [0u8; 2];
		file.read_exact(&mut version_pair)?;
		let version = version_pair[0] * 10 + version_pair[1];

		let mut pos = 4*3;
		let mut frames = Vec::new();
		let mut i = 0;
		loop {
			i += 1;
			println!("{i}/{}, frames", frames.len());
			let frame = HgxFrame::from(&mut file, pos)?;
			pos += frame.next_pos as u64;
			frames.push(frame);
			if frames.last().unwrap().next_pos == 0 {
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

	#[error("The image info was not read first!")]
	UnexpectedTagAtStart(),
	#[error("Nor CG page type or image mode were read last!")]
	UnexpectedTagAtEnd(),
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
			// println!("{name_bytes:?}");
			let next_offset = file.read_le::<u32>()?;
			// println!("{next_offset}");
			let length = file.read_le::<u32>()?;
			// println!("{length}");
			println!("\"{}\" offset={next_offset}; length={length}", String::from_utf8_lossy(&name_bytes));
			let tag = HgxTag::from(file, name_bytes, length)?;
			tags.push(tag);
			if next_offset == 0 {
				match tags.last() {
					Some(HgxTag::CpType(_)) => break,
					Some(HgxTag::ImgMode(_)) => break,
					_ => return Err(HgxFrameError::UnexpectedTagAtEnd())
				}
			}
			println!("got past offset check");
			if tags.len() == 1 {
				match tags.first() {
					Some(HgxTag::StdInfo(_)) => continue,
					_ => return Err(HgxFrameError::UnexpectedTagAtStart())
				}
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
	pub fn from<F: Read + Seek>(file: &mut F, name_bytes: [u8; 8], length: u32) -> Result<Self, HgxTagError> {
		let name: String = name_bytes.iter()
		    .take_while(|&&b| b != 0)
		    .map(|&b| b as char)
			.collect();

		if name.starts_with("img") && !name.starts_with("img_") {
			return Ok(Self::Img(ImgTag::parse(file, length)?))
		}
		if name.starts_with("ats") {
			return Ok(Self::Ats(AtsTag::parse(file)?))
		}

		match name.as_str() {
			"stdinfo" => Ok(Self::StdInfo(StdInfoTag::parse(file)?)),
			"img_jpg" => Ok(Self::ImgJpg(ImgJpgTag::parse(file, length)?)),
			"img_al"=> Ok(Self::ImgAl(ImgAlTag::parse(file)?)),
			"img_jpa" => Ok(Self::ImgJpa()),
			"img_wbp" => Ok(Self::ImgWbp(ImgWbpTag::parse(file, length)?)),
			"cptype"=> Ok(Self::CpType(CpTypeTag::parse(file)?)),
			"imgmode" => Ok(Self::ImgMode(ImgModeTag::parse(file)?)),
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

	pub fn read_raw<F: Read + Seek>(&self, file: &mut F, std_info_tag: &StdInfoTag) -> Result<Vec<u8>, HgxTagError> {
		file.seek(SeekFrom::Start(self.data_pos))?;
		let mut inflated_data_buf = vec![0u8; self.compressed_data_length as usize];
		file.read_exact(&mut inflated_data_buf)?;
		let mut inflated_cmd_buf = vec![0u8; self.compressed_cmd_length as usize];
		file.read_exact(&mut inflated_cmd_buf)?;

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

	pub fn convert<F: Read + Seek>(&self, file: &mut F, std_info_tag: &StdInfoTag, format: image::ImageFormat) -> Result<Vec<u8>, HgxTagError> {
		let raw_data = self.read_raw(file, std_info_tag).unwrap();
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
		let sect_length = unrle_buf.len() / 4;
		let sect1 = &unrle_buf[0..sect_length];
		let sect2 = &unrle_buf[sect_length..sect_length*2];
		let sect3 = &unrle_buf[sect_length*2..sect_length*3];
		let sect4 = &unrle_buf[sect_length*3..sect_length*4];

		let mut table1 = [0u32; 256];
		let mut table2 = [0u32; 256];
		let mut table3 = [0u32; 256];
		let mut table4 = [0u32; 256];
		for i in 0..256 {
			let mut value = (i as u32) & 0xC0 as u32;
			value <<= 6;
			value |= (i as u32) & 0x30;
			value <<= 6;
			value |= (i as u32) & 0x0C;
			value <<= 6;
			value |= (i as u32) & 0x03;

			table4[i] = value;
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
			if out_pos+4 as usize > buf.len() {
				return Err(HgxTagError::InvalidDelta("Buffer overflow".to_string()))
			}
			let value = table1[sect1[i] as usize] |
				table2[sect2[i] as usize] |
				table3[sect3[i] as usize] |
			 	table4[sect4[i] as usize];

			buf[out_pos + 2] = unpack_value((value >> 0) as u8);  // r
			buf[out_pos + 1] = unpack_value((value >> 8) as u8);  // g
			buf[out_pos + 0] = unpack_value((value >> 16) as u8);  // b
			buf[out_pos + 3] = unpack_value((value >> 24) as u8);  // a

			out_pos += 4;
		}

		for x in depth_bytes..stride {
			let x = x as usize;
			if x >= buf.len() {
				return Err(HgxTagError::InvalidDelta(2.to_string()));
			}
			buf[x] = buf[x].wrapping_add(buf[x - depth_bytes as usize]);
		}

		for y in 1..height {
			let line_start = y * stride;
			let prev_start = (y - 1) * stride;

			if (line_start + stride) as usize > buf.len() {
				return Err(HgxTagError::InvalidDelta(3.to_string()))
			}

			for x in 0..stride {
				buf[(line_start + x) as usize] = buf[(line_start + x) as usize].wrapping_add(buf[(prev_start + x) as usize]);
			}
		}

		Ok(buf)
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

	pub fn read<F: Read + Seek>(&self, file: &mut F) -> Result<Vec<u8>, HgxTagError> {
		file.seek(SeekFrom::Start(self.data_pos))?;
		let mut buf = vec![0u8; self.length as usize];
		file.read_exact(&mut buf)?;
		Ok(buf)
	}

	pub fn stream_read<F: Read + Seek>(&self, file: &mut F, pos: u64, chunk_size: usize) -> Result<Vec<u8>, HgxTagError> {
		let start_pos = self.data_pos + pos;
		let remaining_bytes = (self.length as u64).saturating_sub(pos);
		let read_size = chunk_size.min(remaining_bytes as usize);

		if read_size == 0 {
			return Ok(Vec::new());
		}

		file.seek(SeekFrom::Start(start_pos))?;
		let mut buf = vec![0u8; read_size];
		file.read_exact(&mut buf)?;
		Ok(buf)
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

	pub fn read_extract<F: Read + Seek>(&self, file: &mut F) -> Result<Vec<u8>, HgxTagError> {
		let compressed_data = self.read(file)?;
		let mut decoder = flate2::read::ZlibDecoder::new(&compressed_data[..]);
		let mut data = Vec::new();
		decoder.read_to_end(&mut data)?;
		Ok(data)
	}

	pub fn read<F: Read + Seek>(&self, file: &mut F) -> Result<Vec<u8>, HgxTagError> {
		file.seek(SeekFrom::Start(self.data_pos))?;
		let mut buf = vec![0u8; self.compressed_length as usize];
		file.read_exact(&mut buf)?;
		Ok(buf)
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

	pub fn read<F: Read + Seek>(&self, file: &mut F) -> Result<Vec<u8>, HgxTagError> {
		file.seek(SeekFrom::Start(self.data_pos))?;
		let mut buf = vec![0u8; self.length as usize];
		file.read_exact(&mut buf)?;
		Ok(buf)
	}

	pub fn stream_read<F: Read + Seek>(&self, file: &mut F, pos: u64, chunk_size: usize) -> Result<Vec<u8>, HgxTagError> {
		let start_pos = self.data_pos + pos;
		let remaining_bytes = (self.length as u64).saturating_sub(pos);
		let read_size = chunk_size.min(remaining_bytes as usize);

		if read_size == 0 {
			return Ok(Vec::new());
		}

		file.seek(SeekFrom::Start(start_pos))?;
		let mut buf = vec![0u8; read_size];
		file.read_exact(&mut buf)?;
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
	fn parse<F: Read>(file: &mut F) -> Result<Self, HgxTagError> {
		Ok(Self {
			x: file.read_le::<i32>()?,
			y: file.read_le::<i32>()?,
			width: file.read_le::<u32>()?,
			height: file.read_le::<u32>()?,
			color: file.read_le::<u32>()?
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
