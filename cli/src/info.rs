use core::kif::Kif;
use core::pe::Cs2PeFile;
use core::pe::check_cs2_pe_path;
use crate::InfoArgs;

pub fn display_info(args: &InfoArgs) -> () {
	let path = args.file_path.canonicalize().expect("The path doesn't exist!");
	let ext = path.extension().unwrap().to_str().unwrap();

	if ext == "int" {
		let kif = if let Some(pe_path) = args.pe_path.as_ref() {
			let pe_file = Cs2PeFile::from_path(pe_path).expect("The PE file couldn't be read!");
			Kif::from_path_with_seed(&path, pe_file.toc_seed).unwrap()
		} else if let Some(toc_seed) = args.toc_seed {
			Kif::from_path_with_seed(&path, toc_seed).unwrap()
		} else {
			panic!("Neither a PE path nor a TOC seed was supplied!");
		};

		println!("KIF Archive with {} entries detected", kif.entry_count);
		if kif.blowfish_key.is_some() {
			println!("Blowfish key: {}", u32::from_le_bytes(kif.blowfish_key.unwrap()));
			println!("TOC seed: {}", kif.toc_seed.unwrap());
		} else {
			println!("Archive is unencrypted");
		}

		print!("Entries: ");
		for (i, entry) in kif.entries.iter().enumerate() {
			print!("{i}. {}; ", entry.get_file_name());
		}
	} else if ext == "exe" || ext == "bin" {
		if check_cs2_pe_path(&path).unwrap() == false {
			panic!("The PE is invalid!");
		}

		let cs2_pe_file = Cs2PeFile::from_path(&path).unwrap();
		println!("TOC seed: {}", cs2_pe_file.toc_seed);
		println!("Key code: {}", String::from_utf8_lossy(&cs2_pe_file.key_code));
		println!("VCode: {}", String::from_utf8_lossy(&cs2_pe_file.v_code));
		println!("VCode2: {}", String::from_utf8_lossy(&cs2_pe_file.v_code2));
	} else {
		panic!("This format is not yet supported!");
	}
}