use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Error};
use crate::{ExtractArgs, FindPeArgs, InfoArgs};
use core::kif::KifFile;
use core::pe::{find_cs2_pe, Cs2PeFile};

pub fn display_info(args: &InfoArgs) -> () {
	let valid_exts = vec!["int", "bin", "exe"];
	let path = args.path.canonicalize().unwrap();
	let ext = path.extension().unwrap().to_str().unwrap();
	if !valid_exts.contains(&ext) {
		panic!("This format is not yet supported!");
	}

	if ext == "int" {
		let file_name = path.file_name().unwrap().to_str().unwrap();
		let dir = path.parent().unwrap();

		let kif = if let Some(pe_path) = find_cs2_pe(&dir.to_path_buf()) {
			let cs2_pe = Cs2PeFile::from_path(&pe_path).unwrap();
			KifFile::from_path_with_seed(&path, cs2_pe.toc_seed)
				.expect("Couldn't load the KIF archive!")
		} else {
			KifFile::from_path(&path)
				.expect("Couldn't load the KIF archive!")
		};

		println!("-= KIF \"{file_name}\" =-");
		println!("-> {} entries total", kif.entry_count);
		if let Some(blowfish_key) = kif.blowfish_key {
			println!("-> blowfish key: {}", u32::from_le_bytes(blowfish_key));
		}
		if let Some(toc_seed) = kif.toc_seed {
			println!("-> mersenne twister seed: {}", toc_seed);
		}
		// if kif.entry_count < 32 {
			println!("-> entries:");
			for (i, entry) in kif.entries.iter().enumerate() {
				let file_name = entry.get_file_name();
				println!("{i}.\t{file_name} \toffset: {}, length: {}", entry.offset, entry.length);
			}
		// } else {
		// 	println!("-> not displaying entries (too many!)");
		// }
	}
	else if ext == "exe" || ext == "bin" {
		let cs2_pe = Cs2PeFile::from_path(&path).unwrap();
		let file_name = path.file_name().unwrap().to_str().unwrap();
		println!("-= PE \"{file_name}\" =-");
		println!("-> KEY_CODE: {}", String::from_utf8_lossy(&cs2_pe.key_code));
		println!("-> V_CODE: {}", String::from_utf8_lossy(&cs2_pe.v_code));
		println!("-> V_CODE2: {}", String::from_utf8_lossy(&cs2_pe.v_code2));
		println!("-> TOCSEED: {}", &cs2_pe.toc_seed);
	}
}

pub fn find_pe(args: &FindPeArgs) -> () {
	let path = &args.path;

	if let Some(pe_path) = find_cs2_pe(path) {
		println!("CS2 binary found!!!");
		println!("-> \"{}\"", pe_path.to_string_lossy());
	} else {
		println!("CS2 binary couldn't be found...");
	}
}


pub fn extract(args: &ExtractArgs) -> Result<(), Error> {
	let path = args.path.canonicalize()?;
	let parent = path.parent().unwrap().to_path_buf();
	let stem = path.file_stem().unwrap();

	if path.extension().unwrap() != OsStr::new("int") {
		panic!("File isn't an .int !!");
	}

	let pe = if let Some(pe_path) = find_cs2_pe(&parent) {
		Cs2PeFile::from_path(&pe_path).unwrap()
	} else {
		panic!("CS2 binary couldn't be found...")
	};

	let dest = if let Some(dest) = &args.destination {
		dest
	} else {
		&parent.join(stem)
	};

	if dest.is_file() {
		panic!("The destination cannot be a file!");
	}
	if !dest.exists() {
		fs::create_dir_all(dest)?;
	}

	let mut kif_file = File::open(&path)?;
	let kif = KifFile::from_path_with_seed(&path, pe.toc_seed).unwrap();
	for entry in kif.entries {
		if entry.get_file_name() == "__key__.dat" {
			continue
		}
		let file_path = dest.join(entry.get_file_name());
		if file_path.exists() {
			continue
		}
		let mut file = File::create(file_path)?;
		let mut stream_writer = BufWriter::new(&mut file);
		entry.stream_data(&mut kif_file, &mut stream_writer, kif.blowfish_key.unwrap()).unwrap();
	}

	Ok(())
}