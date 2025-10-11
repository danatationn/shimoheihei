use std::path::PathBuf;
use crate::{pe, InfoArgs};
use crate::kif::KifFile;
use crate::pe::Cs2PeFile;

pub fn display_info(args: &InfoArgs) -> () {
	let valid_exts = vec!["int", "bin", "exe"];
	let path = args.path.canonicalize().unwrap();
	let ext = path.extension().unwrap().to_str().unwrap();
	if !valid_exts.contains(&ext) {
		panic!("This format is not yet supported!");
	}

	if ext == "int" {
		let kif = KifFile::from_path(&path).expect("Couldn't load Integrated File!");
		let file_name = path.file_name().unwrap().to_str().unwrap();
		println!("-= KIF \"{file_name}\" =-");
		println!("-> {} entries total", kif.entry_count);
		if kif.blowfish_key != 0 {
			println!("-> blowfish key: {}", kif.blowfish_key);
		}
		if kif.entry_count < 32 {
			println!("-> entries:");
			for (i, entry) in kif.entries.iter().enumerate() {
				let file_name = entry.get_clean_file_name().unwrap();
				print!("{i}: {}; ", file_name);
			}
		} else {
			println!("-> not displaying entries (too many!)");
		}
	}
	else if ext == "exe" || ext == "bin" {
		let cs2_pe = Cs2PeFile::from_path(&path).unwrap();
		let file_name = path.file_name().unwrap().to_str().unwrap();
		println!("-= PE \"{file_name}\" -=");
		println!("-> KEY_CODE: {}", String::from_utf8_lossy(&cs2_pe.key_code));
		println!("-> V_CODE: {}", String::from_utf8_lossy(&cs2_pe.v_code));
		println!("-> V_CODE2: {}", String::from_utf8_lossy(&cs2_pe.v_code2));
	}
}