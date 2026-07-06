use std::{ffi::OsStr, fs::{self, File}, io::{Cursor, Write}, path::PathBuf, sync::atomic::{AtomicUsize, Ordering}};

use crate::{ExtractArgs, utils::convert_hgx};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use smhh::{cst::Cst, fes::Fes, kif::Kif, pe::{Cs2Pe, find_cs2_pe}};
use cli::utils::determine_path;
use anyhow::{Context, Result};

pub fn extract(args: &ExtractArgs) -> Result<()> {
	let file_path = args.path.canonicalize()?;
	let file_name = file_path.file_name()
		.context("Invalid file name")?
		.to_string_lossy();
	let file_stem_path = PathBuf::from(file_path.file_stem()
		.context("Invalid file stem")?);
	let top_dir = file_path.parent()
	    .context("File has no top directory")?;

	if let Ok(mut kif) = Kif::from_path(&file_path) {
		if let Some(toc_seed) = args.toc_seed {
			kif.deobfuscate_entries(toc_seed)?;
		}
		else if let Some(pe_path) = &args.pe_path {
			if let Ok(cs2_pe) = Cs2Pe::from_path(&pe_path) {
				kif.deobfuscate_entries(cs2_pe.toc_seed)?;
			}
		}
		// if no seed is supplied, try and find it yourself
		else if let Some(pe_path) = find_cs2_pe(top_dir) {
			if let Ok(cs2_pe) = Cs2Pe::from_path(&pe_path) {
				kif.deobfuscate_entries(cs2_pe.toc_seed)?;
			}
		}

		if let Some(toc_seed) = kif.toc_seed {
			println!("{file_name} seed: {toc_seed}");
		} else if let Some(_) = kif.blowfish_key {
			println!("Warning: TOC seed couldn't be found. Archive is encrypted and file names will be jarbled.")
		}

		let dest_dir = if let Some(dest_path) = &args.dest_path {
			dest_path
		} else {
			&top_dir.join(file_stem_path)
		};

		println!("Extracting at \"{}\"", dest_dir.to_string_lossy());
		fs::create_dir_all(dest_dir)?;

		let progress = AtomicUsize::new(0);
		kif.entries.par_iter().try_for_each(|entry| {
			if entry.file_name_raw.starts_with(b"__key__.dat") {
				progress.fetch_add(1, Ordering::Relaxed);
				return Ok::<(), anyhow::Error>(());
			}

			let file_name = entry.get_file_name();
			let mut entry_file_path = dest_dir.join(&file_name);

			let entry_data = kif.extract_entry(entry.index as usize)?;

			let data = if file_name.ends_with(".hg3") && !args.dont_convert_images {
				// image conversion
				let ext = args.convert_images_to.as_deref().unwrap_or("bmp");
				entry_file_path.set_extension(ext);

				let image_format = image::ImageFormat::from_extension(ext)
			        .to_owned()
					.unwrap_or_else(|| panic!("Error: `.{ext}` is not a supported image format!\n
							Check https://docs.rs/image/latest/image/codecs/index.html#supported-formats for supported formats"));

				let mut cursor = Cursor::new(&entry_data);
				convert_hgx(&mut cursor, image_format)?
			} else {
				entry_data
			};

			File::create(&entry_file_path)?.write_all(&data)?;
			let file_name = entry_file_path.file_name().unwrap();
			let current_progress = progress.fetch_add(1, Ordering::Relaxed);
			println!("{:?} extracted ({}/{})", file_name, current_progress, kif.entries.len());

			Ok(())
		})?;

		println!("Done!");

		return Ok(());
	}
	else if let Ok(fes) = Fes::from_path(&args.path) {
		let fes_file_path = determine_path(args.path.clone(), args.dest_path.clone(), Some(OsStr::new("txt")));
		File::create(&fes_file_path)?.write_all(&fes.script_text)?;
		println!("Extracted to `{fes_file_path:?}`");
		return Ok(())
	}
	else if let Ok(cst) = Cst::from_path(&args.path) {
		let cst_file_path = determine_path(args.path.clone(), args.dest_path.clone(), Some(OsStr::new("txt")));
		File::create(&cst_file_path)?.write_all(&cst.decomp_script()?)?;
		println!("Extracted to `{cst_file_path:?}`");
		return Ok(())
	}
	else {
		println!("ERROR: This file format is not yet supported!");
		Ok(())
	}
}
