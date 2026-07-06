#![allow(dead_code)]

use std::{ffi::OsStr, io::{Read, Seek}, path::PathBuf};

use smhh::hgx::{Hgx, HgxTag};

use anyhow::Result;

pub fn determine_path(src_path: PathBuf, dest_path: Option<PathBuf>, extension: Option<&OsStr>) -> PathBuf {
	let mut target = dest_path.unwrap_or(src_path);

	if let Some(ext) = extension {
		target.set_extension(ext);
	}

	target
}

pub fn convert_hgx<R: Read + Seek>(reader: &mut R, format: image::ImageFormat) -> Result<Vec<u8>> {
	let hgx = Hgx::from_reader(reader)?;
	let mut data: Vec<u8> = Vec::new();

	for frame in hgx.frames {
		let mut img_found = false;
		let std_info = match &frame.tags[0] {
			HgxTag::StdInfo(std_info) => std_info,
			_ => panic!("The Hgx file is invalid!")
		};

		for tag in &frame.tags {
			if let HgxTag::Img(img) = &tag {
				img_found = true;
				data = img.convert(reader, std_info, format).unwrap();
			}
		}
		if img_found == false {
			eprintln!("Couldn't find the image for frame {}", frame.id);
		}
	}
	Ok(data)
}
