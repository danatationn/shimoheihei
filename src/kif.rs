use std::convert::Infallible;
use std::path::PathBuf;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::num::Wrapping;
use anyhow::Result;
use rand_mt::Mt;

pub struct KifFile {
	pub signature: [u8; 4],
	pub entry_count: u32,
	pub entries: Vec<KifFileEntry>,
	pub blowfish_key: u32,
	pub toc_seed: Option<u32>,
}

impl KifFile {
	pub fn from_string(string: String) -> Result<Self> {
		Self::from_path(&PathBuf::from(string))
	}

	pub fn from_string_with_seed(string: String, toc_seed: u32) -> Result<Self> {
		Self::from_path_with_seed(&PathBuf::from(string), toc_seed)
	}

	pub fn from_path(path: &PathBuf) -> Result<Self> {
		let location = path.to_string_lossy().into_owned();
		let mut file = File::open(location)?;

		let mut signature = [0u8; 4];
		file.read_exact(&mut signature)?;
		assert_eq!(signature, [b'K', b'I', b'F', 0x00]);

		let mut entry_count_bytes = [0u8; 4];
		file.read_exact(&mut entry_count_bytes)?;
		let entry_count = u32::from_le_bytes(entry_count_bytes);

		let mut entries: Vec<KifFileEntry> = Vec::with_capacity(entry_count as usize);
		for i in 0..entry_count {
			let entry = KifFileEntry::from_entry_pos(&mut file, i)?;
			entries.push(entry);
		}

		let mut blowfish_key: u32 = 0;
		let mut expected_name = [0u8; 64];
		for (i, char) in String::from("__key__.dat").chars().enumerate() {
			expected_name[i] = char as u8;
		}
		if expected_name == entries[0].file_name_raw {
			entries[0].file_name = Some(entries[0].file_name_raw);
			blowfish_key = entries[0].length;
		}

		Ok(KifFile { signature, entry_count, entries, blowfish_key, toc_seed: None })
	}

	pub fn from_path_with_seed(path: &PathBuf, toc_seed: u32) -> Result<Self> {
		let mut kif = Self::from_path(path)?;
		kif.toc_seed = Some(toc_seed);

		let mut i = 0;
		let mut seed = Wrapping(toc_seed);
		for entry in &mut kif.entries {
			seed = seed * Wrapping(69069) + Wrapping(1);
			seed = seed * Wrapping(69069) + Wrapping(1);
			seed += i;
			i += 1;
			entry.deobfuscate_filename(seed.0)?;
		}

		Ok(kif)
	}
}

pub struct KifFileEntry {
	pub file_name_raw: [u8; 64],
	pub file_name: Option<[u8; 64]>,
	pub offset: u32,
	pub length: u32,
}

impl KifFileEntry {
	pub fn from_entry_pos(file: &mut File, pos: u32) -> Result<KifFileEntry> {
		file.seek(SeekFrom::Start(8+72*pos as u64))?;

		let mut file_name_raw = [0u8; 64];
		file.read_exact(&mut file_name_raw)?;

		let mut offset_bytes = [0u8; 4];
		file.read_exact(&mut offset_bytes)?;
		let offset = u32::from_le_bytes(offset_bytes);

		let mut length_bytes = [0u8; 4];
		file.read_exact(&mut length_bytes)?;
		let length = u32::from_le_bytes(length_bytes);

		Ok(KifFileEntry { file_name_raw, file_name: None, offset, length } )
	}

	pub fn deobfuscate_filename(&mut self, toc_seed: u32) -> Result<(), Infallible> {
		const FWD: &[u8; 52] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
		const REV: &[u8; 52] = b"zyxwvutsrqponmlkjihgfedcbaZYXWVUTSRQPONMLKJIHGFEDCBA";
		const LENGTH: i32 = 52;
		let key = Wrapping(Mt::from(toc_seed).next_u32());
		println!("{key}");
		let mut shift = ((key >> 24) + (key >> 16) + (key >> 8) + key).0 as u8 as i32;
		let obs_name = self.file_name_raw;
		let file_name = if let Some(_) = self.file_name {
			return Ok(());
		} else {
			self.file_name = Some([0u8; 64]);
			self.file_name.as_mut().unwrap()
		};

		for i in 0..64 {
			let byte = obs_name[i];

			if byte.is_ascii_alphabetic() {
				let mut index: i32 = 0;
				let mut index2: i32 = shift;

				while REV[(index2 % LENGTH) as usize] != byte {
					if REV[((shift + index + 1) % LENGTH) as usize] == byte {
						index += 1;
						break;
					}

					if REV[((shift + index + 2) % LENGTH) as usize] == byte {
						index += 2;
						break;
					}

					if REV[((shift + index + 3) % LENGTH) as usize] == byte {
						index += 3;
						break;
					}

					index += 4;
					index2 += 4;

					if index >= LENGTH { // We're outside the array, no need to continue
						break;
					}
				}

				if index < LENGTH { // Only assign if we're inside the array
					file_name[i] = FWD[index as usize];
				}
				shift += 1;
			} else {
				file_name[i] = obs_name[i];
			}
		}
		Ok(())
	}

	pub fn get_file_name(&self) -> Result<String, Infallible> {
		let mut file_name = String::new();
		let name_arr = if let Some(file_name) = self.file_name {
			file_name
		} else {
			self.file_name_raw
		};

		for byte in name_arr {
			if byte.is_ascii_graphic() {
				file_name.push(byte as char);
			}
		}

		Ok(file_name)
	}
}