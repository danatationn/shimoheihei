use crate::{pe, FindPeArgs, InfoArgs};
use crate::kif::KifFile;
use crate::pe::{find_cs2_pe, Cs2PeFile};

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
		if kif.blowfish_key != 0 {
			println!("-> blowfish key: {}", kif.blowfish_key);
		}
		if let Some(toc_seed) = kif.toc_seed {
			println!("-> mersenne twister seed: {}", toc_seed);
		}
		// if kif.entry_count < 32 {
			println!("-> entries:");
			for (i, entry) in kif.entries.iter().enumerate() {
				let file_name = entry.get_file_name().unwrap();
				print!("{i}: {}; ", file_name);
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