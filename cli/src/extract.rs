use crate::ExtractArgs;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use smhh::{hgx::{Hgx, HgxTag}, kif::Kif, pe::Cs2Pe};
use anyhow::Result;
use std::{fs::{self, File}, io::{Seek, SeekFrom, Write}, sync::Arc, time::Duration};
use image::ImageFormat;

pub fn extract(args: &ExtractArgs) -> Result<()> {
	let path = args.kif_path.canonicalize()?;

	// initializing the kif object
	// checks if it's encrypted or not
	println!("{} (path: {:?})", style("Opening archive...").bold(), path);
    if let Ok(mut kif) = Kif::from(&path) {
		if let Some(toc_seed) = args.toc_seed {
			println!("{} (seed: {toc_seed})", style("Decrypting archive...").bold());
			kif.deobfuscate_entries(toc_seed)?;
		} else if let Some(pe_path) = &args.pe_path {
			let cs2_pe = Cs2Pe::from_path(pe_path)?;
			println!("{} (seed: {})", style("Decrypting archive...").bold(), cs2_pe.toc_seed);
			kif.deobfuscate_entries(cs2_pe.toc_seed)?;
		}

		let kif_file = Arc::new(File::open(&path)?);
		let dir = if let Some(dest_path) = &args.dest_path {
			dest_path
		} else {
			&path.with_extension("")
		};
		println!("{} (dir: {dir:?})", style("Extracting archive...").bold());
		fs::create_dir_all(&dir)?;

		let entries: Vec<_> = kif.entries
			.iter()
			.filter(|entry| !entry.file_name_raw.starts_with(b"__key__.dat"))
			.collect();

		let entries_size: u32 = entries
			.iter()
			.map(|entry| entry.length)
			.sum();

		let pb_style = ProgressStyle::with_template(" {spinner:.cyan} {elapsed_precise} | ETA: {eta} [{wide_bar:.cyan/yellow}] {msg} {bytes}/{total_bytes}")
			.unwrap()
		    .progress_chars("#=-")
			.tick_chars("-\\|/");
		let pb = ProgressBar::new(entries_size as u64);
		pb.set_style(pb_style.clone());
		pb.enable_steady_tick(Duration::from_millis(250));

		// extracts each file
		entries.par_iter().try_for_each(|entry| {
			if entry.file_name_raw.starts_with(b"__key__.dat") {
				return Ok(());
			}
			// pb.println(format!("Extracting \"{}\"...", entry.get_file_name()));
			pb.set_message(entry.get_file_name());

			let file_path = dir.join(entry.get_file_name());
			let mut file = kif_file.clone();
			file.seek(SeekFrom::Start(entry.offset as u64))?;

			let data = entry.extract(&mut file)?;
			// if entry.get_file_name().ends_with(".hg3") && !args.dont_convert_images {
			// 	let image_format = args.convert_images_to
			// 		.as_deref()
			// 		.and_then(ImageFormat::from_extension)
			// 		.unwrap_or(ImageFormat::Bmp);
			// }
			File::create(&file_path)?.write_all(&data)?;
			pb.inc(data.len() as u64);

			Ok::<(), anyhow::Error>(())
        })?;

		println!("{}", style("Extracted succesfully!").green());
		return Ok(())
	}

	// if let Ok(hgx) = Hgx::from(&path) {
	// 	let mut hgx_file = File::open(&path)?;
	// 	let mut img_found = false;

	//    	for frame in hgx.frames {
	// 		for tag in &frame.tags {
	// 			if let HgxTag::Img(img_tag) = &tag {
	// 				img_found = true;
	// 				let std_info = if let HgxTag::StdInfo(std_info) = &frame.tags[0] {
	// 					Some(std_info)
	// 				} else {
	// 					panic!("what");
	// 				};

	// 				let data = img_tag.convert(&mut hgx_file, std_info.unwrap(), image::ImageFormat::Bmp)?;

	// 				let file_path = path.with_extension("bmp");
	// 				let mut file = File::create(&file_path)?;
	// 				file.write(&data)?;
	// 				println!("Extracted to {file_path:?}!");
	// 			}
	// 			if let HgxTag::ImgJpg(_img_jpg_tag) = &tag {
	// 				println!("img jpg tag");
	// 			}
	// 			if let HgxTag::ImgAl(_img_al_tag) = &tag {
	// 				println!("img al tag");
	// 			}
	// 			if let HgxTag::ImgWbp(_img_wbp_tag) = &tag {
	// 				println!("img webp tag");
	// 			}
	// 		}
	// 	}

	// 	if img_found == false {
	// 		println!("ERROR: This image doesn't use the proprietary format!");
	// 	}
	// 	return Ok(())
	// }

	println!("ERROR: This file format is not yet supported!");
	Ok(())
}
