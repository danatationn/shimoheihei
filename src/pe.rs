use std::ffi::OsStr;
use std::path::PathBuf;
use pelite::{FileMap, PeFile};
use pelite::resources::Name;
use std::fs;
use std::io::{Error, ErrorKind};
use blowfish::BlowfishLE;
use blowfish::cipher::{Block, BlockDecrypt, KeyInit};
use blowfish::cipher::consts::U16;
use blowfish::cipher::generic_array::GenericArray;

pub struct Cs2PeFile {
	pub key_code: Vec<u8>,
	pub v_code: Vec<u8>,
	pub v_code2: Vec<u8>,
}

impl Cs2PeFile {
	pub fn from_path(path: &PathBuf) -> Result<Cs2PeFile, Error> {
		let map = FileMap::open(&path)
			.expect("Couldn't read the executable!");
		let file = PeFile::from_bytes(&map)
			.expect("idk either man");
		Self::from_file(file)
	}

	pub fn from_file(file: PeFile) -> Result<Cs2PeFile, Error> {
		let mut valid: bool = false;
		let resources = file.resources()
			.expect("This executable has no resources???");
		let info = resources.version_info()
			.expect("This executable has no info!");
		let lang = info.translation()[0];
		info.strings(lang, |key, value| {
			if key == "InternalName" && value == "cs2" {
				valid = true;
			}
		});
		if !valid {
			panic!("This executable is invalid!")
		}

		let key_code_res = resources.find_resource_ex(
			&[Name::Str("KEY_CODE"), Name::Str("KEY"), Name::Id(lang.lang_id as u32)]
		).unwrap();
		let v_code_res = resources.find_resource_ex(
			&[Name::Str("V_CODE"), Name::Str("DATA"), Name::Id(lang.lang_id as u32)]
		).unwrap();
		let v_code2_res = resources.find_resource_ex(
			&[Name::Str("V_CODE2"), Name::Str("DATA"), Name::Id(lang.lang_id as u32)]
		).unwrap();

		let mut key_code = key_code_res.to_vec();
		for i in 0..key_code.len() {
			key_code[i] ^= 0xCD;
		}

		let v_code = v_code_res.to_vec();  // not important in the slightest

		let blowfish = BlowfishLE::new_from_slice(&key_code).unwrap();
		let mut v_code2 = v_code2_res.to_vec();
		for i in (0..v_code2.len()).step_by(8) {
			let chunk = &mut v_code2[i..i+8];
			let mut block = GenericArray::clone_from_slice(chunk);
			blowfish.decrypt_block(&mut block);
			chunk.copy_from_slice(&block);
		}

		Ok(Cs2PeFile {key_code, v_code, v_code2})
	}
}

pub fn find_cs2_pe(top_dir: PathBuf) -> Option<PathBuf> {
	if !top_dir.is_dir() {
		return None
	}

	let valid_exts = vec![OsStr::new("exe"), OsStr::new("bin")];

	for entry in fs::read_dir(&top_dir).ok()? {

		let path = entry.ok()?.path();
		let ext = match path.extension() {
			Some(e) => e,
			None => continue
		};
		if !valid_exts.contains(&ext) {
			continue
		}
		return Some(path)
	}
	None
}

pub fn get_pe_vcode(pe_path: PathBuf) -> () {
	()
}