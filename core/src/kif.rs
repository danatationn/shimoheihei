use std::cmp;
use std::convert::Infallible;
use std::path::PathBuf;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use anyhow::Result;
use blowfish::BlowfishLE;
use blowfish::cipher::{BlockDecrypt, KeyInit};
use blowfish::cipher::generic_array::GenericArray;
use crate::crypto::{beaufort_cipher, MersenneTwister};

pub struct KifFile {
	pub signature: [u8; 4],
	pub entry_count: u32,
	pub entries: Vec<KifFileEntry>,
	pub blowfish_key: Option<[u8; 4]>,
	pub toc_seed: Option<u32>,
	pub encrypted: bool,
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
		assert_eq!(signature, [b'K', b'I', b'F', 0x0]);

		let mut entry_count_bytes = [0u8; 4];
		file.read_exact(&mut entry_count_bytes)?;
		let entry_count = u32::from_le_bytes(entry_count_bytes);

		let mut entries: Vec<KifFileEntry> = Vec::with_capacity(entry_count as usize);
		for i in 0..entry_count {
			let entry = KifFileEntry::from_entry_pos(&mut file, i)?;
			entries.push(entry);
		}

		let mut blowfish_key: Option<[u8; 4]> = None;
		let mut expected_name = [0u8; 64];
		for (i, char) in String::from("__key__.dat").chars().enumerate() {
			expected_name[i] = char as u8;
		}
		let mut encrypted = false;
		if expected_name == entries[0].file_name_raw {
			entries[0].decrypted = true;
			encrypted = true;
			let mut mt = MersenneTwister::from_seed(entries[0].length);
			blowfish_key = Some(mt.next_u32().to_le_bytes());
		}

		Ok(KifFile { signature, entry_count, entries, blowfish_key, toc_seed: None, encrypted })
	}

	pub fn from_path_with_seed(path: &PathBuf, toc_seed: u32) -> Result<Self> {
		let mut kif = Self::from_path(path)?;
		kif.toc_seed = Some(toc_seed);

		for (i, entry) in kif.entries.iter_mut().enumerate() {
			if !entry.decrypted {
				entry.offset += i as u32;
			}
			if let Some(blowfish_key) = kif.blowfish_key {
				let seed = kif.toc_seed.unwrap() + i as u32;
				entry.decrypt_entry_info(blowfish_key, seed)?;
			}

		}

		Ok(kif)
	}
}

pub struct KifFileEntry {
	pub file_name_raw: [u8; 64],
	pub file_name: Option<[u8; 64]>,
	pub offset: u32,
	pub length: u32,
	pub decrypted: bool,
}

impl KifFileEntry {
	pub fn from_entry_pos(file: & mut File, pos: u32) -> Result<KifFileEntry> {
		file.seek(SeekFrom::Start(8+72*pos as u64))?;

		let mut file_name_raw = [0u8; 64];
		file.read_exact(&mut file_name_raw)?;

		let mut offset_bytes = [0u8; 4];
		file.read_exact(&mut offset_bytes)?;
		let offset = u32::from_le_bytes(offset_bytes);

		let mut length_bytes = [0u8; 4];
		file.read_exact(&mut length_bytes)?;
		let length = u32::from_le_bytes(length_bytes);

		Ok(KifFileEntry { file_name_raw, file_name: None, offset, length, decrypted: false } )
	}

	pub fn decrypt_entry_info(&mut self, blowfish_key: [u8; 4], toc_seed: u32) -> Result<()> {
		if self.decrypted == true {
			return Ok(())
		}

		self.decrypted = true;

		let mut mt = MersenneTwister::from_seed(toc_seed);
		let key = mt.next_u32();
		let shift = ((key >> 24) as u8)
			.wrapping_add((key >> 16) as u8)
			.wrapping_add((key >> 8) as u8)
			.wrapping_add(key as u8) as usize;
		self.file_name = Some(beaufort_cipher(self.file_name_raw, shift));

		let mut table_bytes = [0u8; 8];
		table_bytes[0..4].copy_from_slice(&self.offset.to_le_bytes());
		table_bytes[4..8].copy_from_slice(&self.length.to_le_bytes());

		let blowfish = BlowfishLE::new_from_slice(&blowfish_key).unwrap();
		let mut block = GenericArray::clone_from_slice(&table_bytes);
		blowfish.decrypt_block(&mut block);

		let decrypted_bytes = block.as_slice();
		self.offset = u32::from_le_bytes(decrypted_bytes[0..4].try_into()?);
		self.length = u32::from_le_bytes(decrypted_bytes[4..8].try_into()?);

		Ok(())
	}

	pub fn stream_data<W: Write>(&self, file: &mut File, writer: &mut BufWriter<W>, blowfish_key: [u8; 4]) -> Result<()> {
		let blowfish = BlowfishLE::new_from_slice(&blowfish_key).unwrap();
		let mut buffer = vec![0u8; 8];
		let mut bytes_remaining = self.length as usize;

		while bytes_remaining > 0 {
			let read_size = cmp::min(8, bytes_remaining);
			let buffer_slice = &mut buffer[..read_size];
			file.read_exact(buffer_slice)?;
			if buffer_slice.len() == 8 {
				let mut block = GenericArray::from_mut_slice(buffer_slice);
				blowfish.decrypt_block(&mut block);
			}
			writer.write_all(buffer_slice)?;
			bytes_remaining -= read_size;
		}

		writer.flush()?;
		Ok(())
	}

	pub fn get_file_name(&self) -> String {
		let mut file_name = String::new();
		let name_arr = if let Some(file_name) = self.file_name {
			file_name
		} else {
			self.file_name_raw
		};

		for byte in name_arr {
			if byte == 0 {
				break
			}
			if byte.is_ascii_graphic() {
				file_name.push(byte as char);
			}
		}

		file_name
	}
}