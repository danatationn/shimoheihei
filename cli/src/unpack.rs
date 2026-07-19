use std::{fs::{self, File}, io::{self, Cursor, Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}, time::Duration};
use anyhow::anyhow;

use crate::{BAR_STYLE, SPINNER_STYLE, UnpackArgs, determine_path};
use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use smhh::{cst::Cst, fes::Fes, hgx::{Hgx, HgxTag}, kif::Kif, pe::{Cs2Pe, find_cs2_pe}, zt::{Zt, ZtNode}};

pub struct Unpack {
	args: UnpackArgs,
	mpb: MultiProgress,
}

impl Unpack {
	pub fn from(args: UnpackArgs) -> Self {
		Self {
			args: args,
			mpb: MultiProgress::new(),
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
					eprintln!("WARN: No TOC Seed detected! File names will be garbled..");
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
			fs::create_dir_all(parent)?;
		}

		if self.args.no_convert {
			reader.seek(SeekFrom::Start(0))?;

			let mut file = File::create(&dest_path)?;
			io::copy(reader, &mut file)?;
			return Ok(())
		}

		if let Ok(hgx) = Hgx::from_reader(reader) {
			let image_ext = &self.args.clone().convert_hgx_to;
			let image_format = image::ImageFormat::from_extension(&image_ext).ok_or_else(||
				anyhow!("Invalid image format! Check what formats the image crate can work with")
			)?;

			for frame in &hgx.frames {
				let std_info = match &frame.tags[0] {
					HgxTag::StdInfo(std_info) => std_info,
					_ => return Err(anyhow!("The Hgx file is invalid! (no StdInfo)"))
				};

				for tag in &frame.tags {
					let frame_dest_path = if &hgx.frames.len() > &1 {
						&dest_path.with_extension(&frame.id.to_string())
					} else {
						&dest_path.with_extension("")
					};

					match &tag {
						HgxTag::Img(hg) => {
							write_file(
								&frame_dest_path.with_added_extension(&image_ext),
								self.args.no_overwrite,
								|| -> Result<Vec<u8>> { Ok(hg.convert(reader, std_info, image_format)?) }
							)?;
						},
						HgxTag::ImgAl(al) => {
							let tag_dest_path = &frame_dest_path.with_added_extension("al");
							let data = al.read_extract(reader)?;
							fs::write(tag_dest_path, data)?;
						},
						HgxTag::ImgJpg(jpg) => {
							let tag_dest_path = &frame_dest_path.with_added_extension(&image_ext);
							let data = jpg.read(reader, image_format)?;
							fs::write(tag_dest_path, data)?;
						},
						HgxTag::ImgWbp(wbp) => {
							let tag_dest_path = &frame_dest_path.with_added_extension("webp");
							let data = wbp.read(reader)?;
							fs::write(tag_dest_path, data)?;
						},
						_ => continue,
					}
				}
			}
			return Ok(())
		}

		reader.seek(SeekFrom::Start(0))?;

		if let Ok(fes) = Fes::from_reader(reader) {
			dest_path.set_extension("txt");
			fs::write(dest_path, fes.script_text)?;
			return Ok(())
		}

		reader.seek(SeekFrom::Start(0))?;

		if let Some(stem) = dest_path.file_stem() {
			if let Ok(cst) = Cst::from_reader(reader, stem.to_string_lossy().into_owned()) {
				dest_path.set_extension("txt");
				fs::write(dest_path, cst.decomp_script()?)?;
				return Ok(())
			}
		}

		reader.seek(SeekFrom::Start(0))?;

		if let Ok(zt) = Zt::from_reader(reader) {
			dest_path.set_extension("");
			for node in &zt.root_nodes {
				Self::unpack_zt_node(&self, reader, node, dest_path)?;
			}

			return Ok(())
		}

		reader.seek(SeekFrom::Start(0))?;

		// fallback
		let mut file = File::create(dest_path)?;
		io::copy(reader, &mut file)?;

		Ok(())
	}

	fn unpack_zt_node<R: Read + Seek>(&self, reader: &mut R, node: &ZtNode, dest_path: &PathBuf) -> Result<()> {
		fs::create_dir_all(&dest_path)?;

		match node {
			ZtNode::Directory { name, children } => {
				let dir_path = dest_path.join(name);
				fs::create_dir_all(&dir_path)?;

				for child in children {
					Self::unpack_zt_node(&self, reader, child, &dir_path)?;
				}
			},
			ZtNode::File { name, entry } => {
				let file_path = dest_path.join(name);
				write_file(file_path, self.args.no_overwrite, || -> Result<Vec<u8>> {
					Ok(entry.extract(reader)?)
				})?;
			},
		}

		Ok(())
	}

}

fn write_file<P, C, F>(path: P, no_overwrite: bool, data_fn: F) -> Result<bool>
where
	P: AsRef<Path>,
	C: AsRef<[u8]>,
	F: FnOnce() -> Result<C>,
{
	match fs::metadata(&path) {
		Ok(meta) if meta.len() > 0 && no_overwrite => return Ok(false),
		// Err(e) if e.kind() != ErrorKind::NotFound => return Ok(false),
		Ok(_) | Err(_) => {},
	}

	let data = data_fn()?;
	fs::write(path, data)?;

	Ok(true)
}
