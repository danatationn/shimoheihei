use std::{fs::{self, File}, io::{self, Cursor, Read, Seek, SeekFrom}, path::{Path, PathBuf}, process, time::Duration};

use anyhow::Result;
use image::GrayImage;
use indicatif::{MultiProgress, ProgressBar};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use smhh::{anm::Anm, cst::Cst, fes::Fes, hgx::{Hgx, HgxTag}, kcs::Kcs, kif::Kif, pe::Cs2Pe, zt::{Zt, ZtNode}};

use crate::{BAR_STYLE, SPINNER_STYLE, UnpackArgs, get_toc_seed};

pub struct Unpack {
    args: UnpackArgs,
    mpb: MultiProgress,
    image_format: image::ImageFormat,
}

impl Unpack {
    pub fn from(args: UnpackArgs) -> Self {
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
        	let path = dunce::canonicalize(path)
		         .expect("The path provided does not exist!");

            let file_name = path.file_name()
                .expect("Path `{path}` isn't valid!")
                .to_string_lossy()
                .into_owned();

            if &self.args.paths.len() > &1 {
                // This would make the spinner show
                total_pb.set_message(file_name.to_owned());
                total_pb.enable_steady_tick(Duration::from_secs(1));
            }

            // This can either be a file or a directory, so do .with_extension("") when needed
            // todo: stuff dest_path in a folder when there's multiple paths
            let dest_path = if let Some(dest_path) = &self.args.dest_path {
            	// ~/export/image, ~/export/pcm
	           	dest_path.join(&file_name)
            } else {
            	// ~/image.int | ~/export/
            	path.to_owned()
            };

            Self::unpack_path(&self, &path, &dest_path, &total_pb)?;

            total_pb.inc(1);
        }

        total_pb.finish_and_clear();
        println!("Done!");

        Ok(())
    }

    fn unpack_path(&self, file_path: &PathBuf, dest_path: &PathBuf, total_pb: &ProgressBar) -> Result<()> {
        // KIFs and PEs MUST be loaded from a file as they use mmap . everything else is fed in unpack_reader()
        let mut file = File::open(file_path)?;
        let file_name = file_path
            .file_name()
            .expect("Path `{path}` isn't valid!")
            .to_string_lossy()
            .into_owned();

        if let Ok(mut kif) = Kif::from_file(&mut file) {
            let kif_pb = &self.mpb.insert_before(total_pb, ProgressBar::new(kif.entry_count as u64))
                .with_style(BAR_STYLE.clone())
                .with_message(file_name);

            let toc_seed = get_toc_seed(crate::ArgsType::Unpack(self.args.to_owned()), Some(file_path.to_owned()));

            if let Some(toc_seed) = toc_seed && kif.blowfish_key.is_some() {
	           	println!("TOC seed: {toc_seed}");
                kif.deobfuscate_entries(toc_seed)?;
            }

            let _ = &kif.entries.par_iter().try_for_each(|entry| {
                if entry.file_name_raw.starts_with(b"__key__.dat") {
                    kif_pb.inc(1);
                    return Ok(());
                }

                // `./image.int` -> `./image/`
		        let dest_path = dest_path.with_extension("");

				fs::create_dir_all(&dest_path)?;

                // `./image/` -> `./image/cg_01.hg3`
                let entry_path = dest_path.join(entry.get_file_name());

                let reader_pb = &self.mpb.insert_before(kif_pb, ProgressBar::new_spinner())
                    .with_style(SPINNER_STYLE.clone())
                    .with_prefix("extracting")
                    .with_message(entry_path.to_string_lossy().into_owned());
                reader_pb.enable_steady_tick(Duration::from_millis(500));

                let data = entry.extract(&file)?;
                let mut cursor = Cursor::new(&data);

                Self::unpack_reader(&self, &mut cursor, &entry_path, reader_pb)?;

                kif_pb.inc(1);

                Ok::<(), anyhow::Error>(())
            })?;
        } else if let Ok(_cs2_pe) = Cs2Pe::from_path(file_path) {
            todo!()
        } else {
            if self.args.no_convert && !(file_path.extension().unwrap_or_default() == "zt") {
                eprintln!("`--no-convert` does not work on single files! Try this on archives instead (.int, .zt)...");
                process::exit(1);
            }

            let spinner = &self.mpb.add(ProgressBar::new_spinner().with_style(SPINNER_STYLE.clone()));

            Self::unpack_reader(&self, &mut file, &dest_path, spinner)?;
        }

        Ok(())
    }

    fn unpack_reader<R: Read + Seek>(&self, reader: &mut R, dest_path: &PathBuf, reader_pb: &ProgressBar) -> Result<()> {
    	let file_stem = dest_path.file_stem()
            .expect("Path {dest_path} is not valid!")
            .to_string_lossy()
            .into_owned();

     	let file_ext = dest_path.extension()
	        .unwrap_or_default()
			.to_string_lossy()
			.into_owned();

    	// Remove the duped if statement
        if self.args.no_convert && !(dest_path.extension().unwrap_or_default() == "zt") {
            write_file(
			dest_path,
			self.args.no_overwrite,
			reader_pb,
			&self.mpb,
			|| {
				let mut buf = Vec::new();
				io::copy(reader, &mut buf)?;
				Ok(buf)
			}
            )?;

            return Ok(())
        }

        reader.seek(SeekFrom::Start(0))?;

        if file_ext == "anm" && let Ok(anm) = Anm::from_reader(reader) {
            reader_pb.set_prefix("decompiling");
            write_file(
                dest_path.with_added_extension("txt"),
                self.args.no_overwrite,
                reader_pb,
                &self.mpb,
                || Ok(anm.decompile()?),
            )?;

            return Ok(())
        }

        reader.seek(SeekFrom::Start(0))?;

        if file_ext == "cst" && let Ok(cst) = Cst::from_reader(reader, &file_stem) {
            reader_pb.set_prefix("decompiling");
            write_file(
                dest_path.with_added_extension("txt"),
                self.args.no_overwrite,
                reader_pb,
                &self.mpb,
                || Ok(cst.decomp_script()?),
            )?;

            return Ok(())
        }

        reader.seek(SeekFrom::Start(0))?;

        if file_ext == "fes" && let Ok(fes) = Fes::from_reader(reader) {
            reader_pb.set_prefix("decoding");
            write_file(
                dest_path.with_added_extension("txt"),
                self.args.no_overwrite,
                reader_pb,
                &self.mpb,
                || Ok(fes.extract()?),
            )?;

            return Ok(())
        }

        reader.seek(SeekFrom::Start(0))?;

        if file_ext == "hg3" && let Ok(hg3) = Hgx::from_reader(reader) {
            for frame in &hg3.frames {
                let std_info_tag = match &frame.tags.get(0) {
                    Some(HgxTag::StdInfo(std_info_tag)) => std_info_tag,
                    _ => continue, // not possible
                };

                let hg3_path = dest_path.with_extension("");

                let frame_path = if &hg3.frames.len() > &1 {
                    hg3_path.with_extension(&frame.id.to_string()) // cg_02_anim.24
                } else {
                    hg3_path
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
                                || Ok(hg.convert(reader, std_info_tag, self.image_format)?),
                            )?;
                        }
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
                        }
                        HgxTag::ImgJpg(jpg) => {
                            reader_pb.set_prefix("converting");
                            write_file(
                                &frame_path.with_added_extension(String::from("lossy.") + &self.args.convert_hgx_to),
                                self.args.no_overwrite,
                                reader_pb,
                                &self.mpb,
                                || Ok(jpg.read(reader, self.image_format)?),
                            )?;
                        }
                        HgxTag::ImgWbp(wbp) => {
                            write_file(
                                &frame_path.with_added_extension("webp"),
                                self.args.no_overwrite,
                                reader_pb,
                                &self.mpb,
                                || Ok(wbp.read(reader)?),
                            )?;
                        }
                        _ => continue,
                    }
                }
            }

            return Ok(())
        }

        // reader.seek(SeekFrom::Start(0))?;

        // if file_ext == "kcs" && let Ok(kcs) = Kcs::from_reader(reader) {
        // 	write_file(
        //  		dest_path.with_added_extension("bin"),
        //    		self.args.no_overwrite,
        //      	reader_pb,
        //       	&self.mpb,
        //        	|| Ok(kcs.data),
        // 	)?;

        //  	return Ok(())
        // }

        reader.seek(SeekFrom::Start(0))?;

        if file_ext == "zt" && let Ok(zt) = Zt::from_reader(reader) {
            for node in &zt.root_nodes {
                Self::unpack_zt_node(&self, reader, node, &dest_path.with_extension(""), reader_pb)?;
            }

            return Ok(())
        }

        reader.seek(SeekFrom::Start(0))?;

        write_file(
			dest_path,
			self.args.no_overwrite,
			reader_pb,
			&self.mpb,
			|| {
				let mut buf = Vec::new();
				io::copy(reader, &mut buf)?;
				Ok(buf)
			}
        )?;

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
            }
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
            }
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
        "{} {}",
        if write { "* unpacked" } else { "x skipped" },
        path.display()
    ))?;

    Ok(write)
}
