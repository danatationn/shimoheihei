use std::{fs::{self, File, create_dir_all}, io::{Cursor, Read, Seek, SeekFrom, Write, copy}, path::PathBuf, time::Duration};

use crate::{BAR_STYLE, SPINNER_STYLE, UnpackArgs, determine_path};
use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar,};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use smhh::{cst::Cst, fes::Fes, hgx::{Hgx, HgxTag}, kif::Kif, pe::{Cs2Pe, find_cs2_pe}, zt::{Zt, ZtNode}};

pub struct Unpack {
	args: UnpackArgs,
	mpb: MultiProgress,
	total_pb: Option<ProgressBar>,
}

impl Unpack {
	pub fn from(args: UnpackArgs) -> Self {
		Self {
			args: args,
			mpb: MultiProgress::new(),
			total_pb: None,
		}
	}

	pub fn unpack(&self) -> Result<()> {
		for file_path in &self.args.paths {
			let mut dest_path = determine_path(&file_path, &self.args.dest_path);
			let mut file = File::open(&file_path)?;

			if let Ok(mut kif) = Kif::from_file(&mut file) {
				if let Some(toc_seed) = self.args.toc_seed {
					kif.deobfuscate_entries(toc_seed)?;
				}
				else if let Some(pe_path) = &self.args.pe_path {
					if let Ok(cs2_pe) = Cs2Pe::from_path(&pe_path) {
						kif.deobfuscate_entries(cs2_pe.toc_seed)?;
					}
				}
				// if no seed is supplied, try and find it yourself
				else if let Some(parent) = &file_path.parent() {
					if let Some(pe_path) = find_cs2_pe(parent) {
						if let Ok(cs2_pe) = Cs2Pe::from_path(&pe_path) {
							kif.deobfuscate_entries(cs2_pe.toc_seed)?;
						}
					}
				}
				else {
					eprintln!("WARN: no toc seed detected!");
				}

				let total_pb = &self.mpb.add(ProgressBar::new(kif.entry_count as u64))
			        .with_style(BAR_STYLE.clone());

				if let Some(name) = file_path.file_name() {
					total_pb.set_message(name.to_string_lossy().into_owned());
				}

				dest_path = dest_path.with_extension("");

				kif.entries.par_iter().try_for_each(|entry| {
					if entry.file_name_raw.starts_with(b"__key__.dat") {
						total_pb.inc(1);
						return Ok::<(), anyhow::Error>(())
					}

					let pb = self.mpb.insert_before(&total_pb, ProgressBar::new_spinner()
						.with_style(SPINNER_STYLE.clone()))
						.with_message(entry.get_file_name())
						.with_prefix("unpacking");
					pb.enable_steady_tick(Duration::from_millis(100));

					let mut entry_dest_path = dest_path.join(entry.get_file_name());
					let data = entry.extract(&file)?;
					let mut cursor = Cursor::new(&data);
					Self::unpack_stream(&self, &mut cursor, &mut entry_dest_path)?;

					total_pb.inc(1);
					pb.finish_and_clear();
					self.mpb.println(format!("O unpacked {}", entry.get_file_name()))?;

					Ok(())
				})?;
			}
			else if let Ok(pe) = Cs2Pe::from_file(&file) {
				let mut data = String::from(format!("KEY_CODE: {}\n", String::from_utf8_lossy(&pe.key_code)));
				data.push_str(&format!("V_CODE: {}\n", String::from_utf8_lossy(&pe.v_code)));
				data.push_str(&format!("V_CODE2: {}\n", String::from_utf8_lossy(&pe.v_code2)));
				data.push_str(&format!("TOC_SEED: {}", pe.toc_seed));
				let dest_path = dest_path.with_added_extension("txt");
				File::create(dest_path)?.write_all(data.as_bytes())?;
			}
			else {
				Self::unpack_stream(&self, &mut file, &mut dest_path)?;
			}
		}
		Ok(())
	}

	pub fn unpack_stream<R: Read + Seek>(&self, reader: &mut R, dest_path: &mut PathBuf) -> Result<()> {
		if let Some(parent) = &dest_path.parent() {
			create_dir_all(parent)?;
		}

		if self.args.raw {
			reader.seek(SeekFrom::Start(0))?;

			let mut file = File::create(&dest_path)?;
			copy(reader, &mut file)?;
			return Ok(())
		}

		let mut data: Vec<u8> = Vec::new();

		if let Ok(hgx) = Hgx::from_reader(reader) {
			let image_ext = &self.args.clone().convert_hgx_to;
			let image_format = image::ImageFormat::from_extension(&image_ext).unwrap_or_else(||
				panic!("invalid image format"));

			for frame in &hgx.frames {
				let mut img_found = false;
				let std_info = match &frame.tags[0] {
					HgxTag::StdInfo(std_info) => std_info,
					_ => panic!("The Hgx file is invalid!")
				};

				for tag in &frame.tags {
					if let HgxTag::Img(img) = &tag {
						img_found = true;
						let tag_dest_path = &dest_path.with_extension(format!("{}.{}", &frame.id, &image_ext));
						let data = img.convert(reader, std_info, image_format).unwrap();
						File::create(tag_dest_path)?.write_all(&data)?;
					}
				}
				if img_found == false {
					eprintln!("Couldn't find the image for frame {}", frame.id);
				}
			}
			return Ok(())
		}
		reader.seek(SeekFrom::Start(0))?;
		if let Ok(fes) = Fes::from_reader(reader) {
			data = fes.script_text;
			dest_path.set_extension("txt");
		}
		reader.seek(SeekFrom::Start(0))?;
		if let Some(stem) = dest_path.file_stem() {
			if let Ok(cst) = Cst::from_reader(reader, stem.to_string_lossy().into_owned()) {
				data = cst.decomp_script()?;
				dest_path.set_extension("txt");
			}
		}
		reader.seek(SeekFrom::Start(0))?;
		if let Ok(zt) = Zt::from_reader(reader) {
			dest_path.set_extension("");
			for node in &zt.root_nodes {
				unpack_zt_node(reader, node, dest_path)?;
			}

			return Ok(())
		}
		reader.seek(SeekFrom::Start(0))?;
		if data.is_empty() {
			// fallback
			let mut file = File::create(dest_path)?;
			copy(reader, &mut file)?;
			return Ok(())
		}

		File::create(dest_path)?.write_all(&data)?;

		Ok(())
	}
}

fn unpack_zt_node<R: Read + Seek>(reader: &mut R, node: &ZtNode, dest_path: &PathBuf) -> Result<()> {
	match node {
		ZtNode::Directory { name, children } => {
			let dir_path = dest_path.join(name);
			fs::create_dir_all(&dir_path)?;

			for child in children {
				unpack_zt_node(reader, child, &dir_path)?;
			}
		},
		ZtNode::File { name, entry } => {
			let file_path = dest_path.join(name);
			let data = entry.extract(reader)?;
			fs::write(file_path, data)?;
		},
	}

	Ok(())
}
