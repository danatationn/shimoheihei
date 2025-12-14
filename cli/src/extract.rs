use crate::ExtractArgs;
use core::{hgx::{Hgx, HgxTag}, kif::Kif, pe::Cs2Pe};
use anyhow::Result;
use std::{fs::{self, File}, io::{Seek, SeekFrom, Write}};

pub fn extract(args: &ExtractArgs) -> Result<()> {
	let path = args.kif_path.canonicalize()?;

    if let Ok(mut kif) = Kif::from(&path) {
		if let Some(toc_seed) = args.toc_seed {
			kif.deobfuscate_entries(toc_seed)?;
		} else if let Some(pe_path) = &args.pe_path {
			let cs2_pe = Cs2Pe::from_path(pe_path)?;
			kif.deobfuscate_entries(cs2_pe.toc_seed)?;
		}
		let mut kif_file = File::open(&path)?;

		let dir = path.with_extension("");
		fs::create_dir_all(&dir)?;

		for entry in kif.entries {
			if entry.file_name_raw.starts_with(b"__key__.dat") {
				continue
			}
			let file_path = dir.join(entry.get_file_name() + ".part");
			let mut file = File::create(&file_path)?;
			kif_file.seek(SeekFrom::Start(entry.offset as u64))?;

			let mut pos = 0;
			while pos < entry.length {
				let data = entry.stream_extract(&mut kif_file)?;
				file.write(&data)?;
				pos += data.len() as u32;
			}
			fs::rename(&file_path, &file_path.with_extension(""))?;
		}
		return Ok(())
	}
	if let Ok(hgx) = Hgx::from(&path) {
		let mut hgx_file = File::open(&path)?;

	   	for frame in hgx.frames {
			for tag in &frame.tags {
				if let HgxTag::Img(img_tag) = &tag {
					let std_info = if let HgxTag::StdInfo(std_info) = &frame.tags[0] {
						Some(std_info)
					} else {
						panic!("what");
					};

					let data = img_tag.convert(&mut hgx_file, std_info.unwrap(), image::ImageFormat::Bmp)?;

					let file_path = path.with_extension("bmp");
					let mut file = File::create(&file_path)?;
					file.write(&data)?;
					println!("Extracted to {file_path:?}!");
				}
				if let HgxTag::ImgJpg(_img_jpg_tag) = &tag {
					println!("img jpg tag");
				}
				if let HgxTag::ImgAl(_img_al_tag) = &tag {
					println!("img al tag");
				}
				if let HgxTag::ImgWbp(_img_wbp_tag) = &tag {
					println!("img webp tag");
				}
			}
		}
		return Ok(())
	}

	Hgx::from(&path)?;

	println!("This file format is not yet supported!");
	Ok(())
}
