use std::{fs::{self, File}, io::{self, Cursor, Read, Seek, SeekFrom}, path::{Path, PathBuf}, time::Duration};

use anyhow::Result;
use image::GrayImage;
use indicatif::{MultiProgress, ProgressBar};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use smhh::{cst::Cst, fes::Fes, hgx::{Hgx, HgxTag}, kif::Kif, pe::{Cs2Pe, find_cs2_pe}, zt::{Zt, ZtNode}};

use crate::{BAR_STYLE, SPINNER_STYLE, UnpackArgs};

pub struct Unpack {
	args: UnpackArgs,
	mpb: MultiProgress,
	image_format: image::ImageFormat,
}

impl Unpack {
	pub fn from_args(args: UnpackArgs) -> Self {
		let image_format = image::ImageFormat::from_extension(&args.convert_hgx_to)
		    .unwrap_or_else(|| panic!("`{}` is not a valid image format!", &args.convert_hgx_to));

		Self {
			args,
			mpb: MultiProgress::new(),
			image_format,
		}
	}

	pub fn unpack(&self) -> Result<()> {
		let total_pb = if &self.args.paths.len() > &1 {
			&self.mpb.add(ProgressBar::new(self.args.paths.len() as u64))
				.with_style(BAR_STYLE.clone())
		} else {
			// Invisible :eyes:
			&self.mpb.add(ProgressBar::new_spinner())
		};

		for path in &self.args.paths {
			let file_name = path.file_name()
				.expect("Path `{path}` isn't valid!")
				.to_string_lossy()
				.into_owned();


			if &self.args.paths.len() > &1 {
				// This would make the spinner show
				total_pb.set_message(file_name);
				total_pb.enable_steady_tick(Duration::from_secs(1));
			}

			let parent_path = path.parent().expect("Root can't be unpacked!").to_path_buf();
			let dest_path = self.args.dest_path.clone().unwrap_or(parent_path.to_owned());

			// Look if toc_seed exists anywhere
			let toc_seed = if let Some(toc_seed) = &self.args.toc_seed {
				Some(toc_seed.to_owned())
			} else if let Some(pe_path) = &self.args.pe_path {
				let cs2_pe = Cs2Pe::from_path(&pe_path)?;
				println!("TOC seed: {}", cs2_pe.toc_seed);
				Some(cs2_pe.toc_seed)
			} else if let Some(pe_path) = find_cs2_pe(parent_path.to_owned()) {
				println!("Using TOC seed from {pe_path:?}");
				let cs2_pe = Cs2Pe::from_path(&pe_path)?;
				println!("TOC seed: {}", cs2_pe.toc_seed);
				Some(cs2_pe.toc_seed)
			} else {
				None
			};

			Self::unpack_path(&self, &path, &dest_path, &total_pb, toc_seed)?;

			total_pb.inc(1);
		}

		total_pb.finish_and_clear();
		println!("Done!");

		Ok(())
	}

	fn unpack_path(&self, file_path: &PathBuf, dest_path: &PathBuf, total_pb: &ProgressBar, toc_seed: Option<u32>) -> Result<()> {
		// KIFs and PEs MUST be loaded from a file as they use mmap . everything else is fed in unpack_reader()
		let mut file = File::open(file_path)?;
		let file_name = file_path.file_name()
			.expect("Path `{path}` isn't valid!")
			.to_string_lossy()
			.into_owned();

		if let Ok(mut kif) = Kif::from_file(&mut file) {
			let kif_pb = &self.mpb.insert_before(total_pb, ProgressBar::new(kif.entry_count as u64))
				.with_style(BAR_STYLE.clone())
				.with_message(file_name);

			if let Some(toc_seed) = toc_seed {
				kif.deobfuscate_entries(toc_seed)?;
			}

			let _ = &kif.entries.par_iter().try_for_each(|entry| {
				if entry.file_name_raw.starts_with(b"__key__.dat") {
					kif_pb.inc(1);
					return Ok(())
				}

				let reader_pb = &self.mpb.insert_before(kif_pb, ProgressBar::new_spinner())
					.with_style(SPINNER_STYLE.clone())
				    .with_prefix("extracting")
					.with_message(entry.get_file_name());
				reader_pb.enable_steady_tick(Duration::from_millis(500));

				let data = entry.extract(&file)?;
				let mut cursor = Cursor::new(&data);

				// `./image.int` -> `./image/`
				let mut dest_path = file_path.with_extension("");
				fs::create_dir_all(&dest_path)?;

				// `./image/` -> `./image/cg_01.hg3`
				let entry_path = dest_path.join(entry.get_file_name());

				Self::unpack_reader(&self, &mut cursor, &entry_path, &mut dest_path, reader_pb)?;

				kif_pb.inc(1);

				Ok::<(), anyhow::Error>(())
			})?;
		} else if let Ok(_cs2_pe) = Cs2Pe::from_path(file_path) {
			todo!()
		} else {
			let mut file = File::open(file_path)?;

			let spinner = &self.mpb.add(ProgressBar::new_spinner()
				.with_style(SPINNER_STYLE.clone()));

			// todo: Change to expect()
			let dest_path = file_path.parent().unwrap().to_path_buf();
			fs::create_dir_all(&dest_path)?;

			if self.args.no_convert && !file_path.ends_with(".zt") {
				eprintln!("`--no-convert` does not work on single files! Try this on archives instead (.int, .zt)...")
			}

			Self::unpack_reader(&self, &mut file, file_path, &dest_path, spinner)?;
		}

		Ok(())
	}

	fn unpack_reader<R: Read + Seek>(&self, reader: &mut R, file_path: &PathBuf, dest_path: &PathBuf, reader_pb: &ProgressBar) -> Result<()> {
		let file_stem = file_path.file_stem()
			.expect("Path {file_path} is not valid!")
			.to_string_lossy()
			.into_owned();

		let file_name = file_path.file_name()
			.expect("Path is not valid!")
			.to_string_lossy()
			.into_owned();

		if self.args.no_convert {
			let mut file = File::create(dest_path.join(file_name))?;
			io::copy(reader, &mut file)?;
			return Ok(())
		}

		if let Ok(cst) = Cst::from_reader(reader, &file_stem) {
			reader_pb.set_prefix("decompiling");
			write_file(
				dest_path.join(&file_stem).with_extension("txt"),
				self.args.no_overwrite,
				reader_pb,
				&self.mpb,
				|| { Ok(cst.decomp_script()?) }
			)?;

			return Ok(())
		}

		reader.seek(SeekFrom::Start(0))?;

		if let Ok(fes) = Fes::from_reader(reader) {
			reader_pb.set_prefix("decoding");
			write_file(
				dest_path.join(&file_stem).with_extension("txt"),
				self.args.no_overwrite,
				reader_pb,
				&self.mpb,
				|| { Ok(fes.extract()?) },
			)?;

			return Ok(())
		}

		reader.seek(SeekFrom::Start(0))?;

		if let Ok(hg3) = Hgx::from_reader(reader) {
			for frame in &hg3.frames {
				let std_info_tag = match &frame.tags.get(0) {
					Some(HgxTag::StdInfo(std_info_tag)) => std_info_tag,
					_ => continue,  // not possible
				};

				let hg3_path = dest_path.join(&file_stem);

				let frame_path = if &hg3.frames.len() > &1 {
					hg3_path.with_extension(&frame.id.to_string())  // cg_02_anim.24
				} else {
					hg3_path.with_extension("")
				};

				for tag in &frame.tags {
					match tag {
						HgxTag::Img(hg) => {
							reader_pb.set_prefix("converting");
							write_file(
								&frame_path.with_added_extension(&self.args.convert_hgx_to),
								self.args.no_overwrite,
								reader_pb,
								&self.mpb,
								|| {
									Ok(hg.convert(reader, std_info_tag, self.image_format)?)
								},
							)?;
						},
						HgxTag::ImgAl(al) => {
							reader_pb.set_prefix("converting");
							let raw_data = al.read_extract(reader)?;
							let image = GrayImage::from_raw(std_info_tag.width, std_info_tag.height, raw_data)
								.expect("Std Info has bad image dimensions!");

							write_file(
								&frame_path.with_added_extension(String::from("mask.") + &self.args.convert_hgx_to),
								self.args.no_overwrite,
								reader_pb,
								&self.mpb,
								|| {
									let mut buf: Vec<u8> = Vec::new();
									let mut cursor = Cursor::new(&mut buf);
									image.write_to(&mut cursor, self.image_format)?;
									Ok(buf)
								},
							)?;
						},
						HgxTag::ImgJpg(jpg) => {
							reader_pb.set_prefix("converting");
							write_file(
								&frame_path.with_added_extension(String::from("lossy.") + &self.args.convert_hgx_to),
								self.args.no_overwrite,
								reader_pb,
								&self.mpb,
								|| { Ok(jpg.read(reader, self.image_format)?) }
							)?;
						},
						HgxTag::ImgWbp(wbp) => {
							write_file(
								&frame_path.with_added_extension("webp"),
								self.args.no_overwrite,
								reader_pb,
								&self.mpb,
								|| { Ok(wbp.read(reader)?) }
							)?;
						},
						_ => continue,
					}
				}
			}
			return Ok(())
		}

		reader.seek(SeekFrom::Start(0))?;

		if let Ok(zt) = Zt::from_reader(reader) {
			for node in &zt.root_nodes {
				Self::unpack_zt_node(&self, reader, node, dest_path, reader_pb)?;
			}

			return Ok(())
		}

		reader.seek(SeekFrom::Start(0))?;

		// Fallback
		let mut file = File::create(dest_path.join(file_name))?;
		io::copy(reader, &mut file)?;

		Ok(())
	}

	fn unpack_zt_node<R: Read + Seek>(&self, reader: &mut R, node: &ZtNode, dest_path: &PathBuf, pb: &ProgressBar) -> Result<()> {
		match node {
			ZtNode::Directory { name, children } => {
				let dir_path = dest_path.join(name);
				fs::create_dir_all(&dir_path)?;

				for child in children {
					Self::unpack_zt_node(&self, reader, child, &dir_path, pb)?;
				}
			},
			ZtNode::File { name, entry } => {
				let file_path = dest_path.join(name);

				let file_pb = &self.mpb.insert_before(pb, ProgressBar::new_spinner())
			        .with_style(SPINNER_STYLE.clone())
			        .with_prefix("extracting");
				file_pb.enable_steady_tick(Duration::from_millis(500));

				write_file(
					file_path,
					self.args.no_overwrite,
					file_pb,
					&self.mpb,
					|| -> Result<Vec<u8>> { Ok(entry.extract(reader)?) },
				)?;
			},
		}

		Ok(())
	}
}


fn write_file<P, C, F>(path: P, no_overwrite: bool, pb: &ProgressBar, mpb: &MultiProgress, data_fn: F) -> Result<bool>
	where
		P: AsRef<Path>,
		C: AsRef<[u8]>,
		F: FnOnce() -> Result<C>,
	{
		let path = path.as_ref();
		pb.set_message(path.to_string_lossy().into_owned());

		let write = match fs::metadata(&path) {
			Ok(meta) if meta.len() > 0 && no_overwrite => false,
			_ => true,
		};

		if write {
			let data = data_fn()?;
			fs::write(path, data)?;
		}

		pb.finish_and_clear();
		mpb.println(format!(
			"{} {}", if write { "O unpacked" } else { "X skipped" }, path.display()
		))?;

		Ok(write)
	}
