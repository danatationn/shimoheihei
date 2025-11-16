use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use pelite::{FileMap, PeFile};
use pelite::resources::Name;
use std::fs;
use blowfish::BlowfishLE;
use blowfish::cipher::{BlockDecrypt, KeyInit};
use blowfish::cipher::generic_array::GenericArray;
use anyhow::Result;

pub struct Cs2PeFile {
	pub key_code: Vec<u8>,
	pub v_code: Vec<u8>,
	pub v_code2: Vec<u8>,
	pub toc_seed: u32,
}

impl Cs2PeFile {
	pub fn from_path<P: AsRef<Path>>(path: &P) -> Result<Self> {
		let map = FileMap::open(&path)
			.expect("Couldn't read the executable!");
		let file = PeFile::from_bytes(&map)?;
		Self::from_file(file)
	}

	pub fn from_file(file: PeFile) -> Result<Self> {
		let valid = check_cs2_pe(&file)?;
		if !valid {
			panic!("This executable is invalid!")
		}

		let resources = file.resources()
			.expect("This executable has no resources???");
		let info = resources.version_info()
			.expect("This executable has no info!");
		let lang = info.translation()[0];

		let key_code_res = resources.find_resource_ex(
			&[Name::Str("KEY_CODE"), Name::Str("KEY"), Name::Id(lang.lang_id as u32)]
		)?;
		let v_code_res = resources.find_resource_ex(
			&[Name::Str("V_CODE"), Name::Str("DATA"), Name::Id(lang.lang_id as u32)]
		)?;
		let v_code2_res = resources.find_resource_ex(
			&[Name::Str("V_CODE2"), Name::Str("DATA"), Name::Id(lang.lang_id as u32)]
		)?;

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

		if let Some(pos) = v_code2.iter().position(|&byte| byte == 0) {
			v_code2.truncate(pos);
		}

		const MAGIC: u32 = 0x04C11DB7;
		let mut toc_seed: u32 = 0xFFFFFFFF;

		for byte in &v_code2 {
			toc_seed ^= (*byte as u32) << 24;
			for _ in 0..8 {
				if (toc_seed & 0x80000000) != 0 {
					toc_seed <<= 1;
					toc_seed ^= MAGIC;
				} else {
					toc_seed <<= 1;
				}
			}
			toc_seed = !toc_seed;
		}

		Ok(Cs2PeFile {key_code, v_code, v_code2, toc_seed})
	}
}


pub fn find_cs2_pe<P: AsRef<Path>>(top_dir: P) -> Option<PathBuf> {
	let top_dir = top_dir.as_ref();
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
		let map = FileMap::open(&path).ok()?;
		let file = PeFile::from_bytes(&map).ok()?;
		if let Ok(valid) = check_cs2_pe(&file) {
			if valid {
				return Some(path)
			}
		}
	}
	None
}

pub fn check_cs2_pe_path<P: AsRef<Path>>(path: P) -> Result<bool> {
	let map = FileMap::open(&path)?;
	let file = PeFile::from_bytes(&map)?;
	check_cs2_pe(&file)
}

pub fn check_cs2_pe(file: &PeFile) -> Result<bool> {
	let mut valid: bool = false;

	let resources = file.resources()?;
	let info = resources.version_info()?;
	let lang = info.translation()[0];
	info.strings(lang, |key, value| {
		if key == "InternalName" && value == "cs2" {
			valid = true;
		}
	});

	Ok(valid)
}