use crate::crypt::{MersenneTwister, beaufort_cipher};
use blowfish::BlowfishLE;
use blowfish::cipher::generic_array::GenericArray;
use blowfish::cipher::{BlockDecrypt, KeyInit};
use std::array::TryFromSliceError;
use std::cmp::min;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::rc::Rc;
use thiserror::Error;

pub struct Kif {
pub entry_count: u32,
pub entries: Vec<KifEntry>,
pub blowfish_key: Option<[u8; 4]>,
pub blowfish: Option<Rc<BlowfishLE>>,
pub toc_seed: Option<u32>,
}

#[derive(Error, Debug)]
pub enum KifError {
	#[error("This KIF is invalid! ({0:?})")]
	InvalidSignature([u8; 4]),
	#[error("Io error: {0}")]
	Io(#[from] std::io::Error),
	#[error("Entry error: {0}")]
	Entry(#[from] KifEntryError)
}

impl Kif {
	pub fn from<P: AsRef<Path>>(path: &P) -> Result<Self, KifError> {
		let mut file = File::open(path)?;
		
		let mut signature = [0u8; 4];
		file.read_exact(&mut signature)?;
		if signature != [b'K', b'I', b'F', 0x0] {
		return Err(KifError::InvalidSignature(signature));
		}
		
		let mut entry_count_bytes = [0u8; 4];
		file.read_exact(&mut entry_count_bytes)?;
		let entry_count = u32::from_le_bytes(entry_count_bytes);
		
		let mut entries: Vec<KifEntry> = Vec::with_capacity(entry_count as usize);
		
		let entry = KifEntry::from(&mut file, 0)?;
		let mut blowfish_key: Option<[u8; 4]> = None;
		let mut blowfish: Option<Rc<BlowfishLE>> = None;
		if entry.file_name_raw.starts_with(b"__key__.dat") {
		let mut mt = MersenneTwister::from_seed(entry.length);
		blowfish_key = Some(mt.next_u32().to_le_bytes());
		let blowfish_inst = BlowfishLE::new_from_slice(&blowfish_key.unwrap()).unwrap();
		blowfish = Some(Rc::new(blowfish_inst));
		}
		entries.push(entry);
		
		for i in 1..entry_count {
		let entry = if let Some(blowfish) = &blowfish {
		KifEntry::from_with_blowfish(&mut file, i, blowfish.clone())?
		} else {
		KifEntry::from(&mut file, i)?
		};
		entries.push(entry);
		}
		
		Ok(Kif { entry_count, entries, blowfish_key, blowfish, toc_seed: None })
	}
	
	pub fn from_path_with_seed<P: AsRef<Path>>(path: &P, toc_seed: u32) -> Result<Self, KifError> {
		let mut kif = Self::from(path)?;
		kif.toc_seed = Some(toc_seed);
		
		if let Some(blowfish) = &kif.blowfish {
			for entry in &mut kif.entries {
				entry.blowfish = Some(Rc::clone(blowfish));
				entry.deobfuscate_file_name(toc_seed);
			}
		}
		
		Ok(kif)
	}
}

pub struct KifEntry {
	pub index: u32,
	pub file_name_raw: [u8; 64],
	pub file_name: Option<[u8; 64]>,
	pub offset: u32,
	pub length: u32,
	pub blowfish: Option<Rc<BlowfishLE>>,
	pub decrypted: bool,
}

#[derive(Error, Debug)]
pub enum KifEntryError {
	#[error("{0}")]
	KeyData(String),
	#[error("Io error: {0}")]
	Io(#[from] std::io::Error),
	#[error("Slice error: {0}")]
	Slice(#[from] TryFromSliceError)
}

impl KifEntry {
	pub fn from<F: Seek + Read>(file: &mut F, index: u32) -> Result<KifEntry, KifEntryError> {
		file.seek(SeekFrom::Start(8 + (8 + 64) * index as u64))?;
		
		let mut file_name_raw = [0u8; 64];
		file.read_exact(&mut file_name_raw)?;
		
		let mut offset_bytes = [0u8; 4];
		file.read_exact(&mut offset_bytes)?;
		let offset = u32::from_le_bytes(offset_bytes);
		
		let mut length_bytes = [0u8; 4];
		file.read_exact(&mut length_bytes)?;
		let length = u32::from_le_bytes(length_bytes);
		
		Ok(KifEntry { index, file_name_raw, file_name: None, offset, length, blowfish: None, decrypted: false })
	}
	
	pub fn from_with_blowfish<F: Seek + Read>(file: &mut F, pos: u32, blowfish: Rc<BlowfishLE>) -> Result<KifEntry, KifEntryError> {
		let mut entry = Self::from(file, pos)?;
		entry.blowfish = Some(Rc::clone(&blowfish));
		entry.deobfuscate_info(Rc::clone(&blowfish))?;
		Ok(entry)
	}
		
	pub fn deobfuscate_file_name(&mut self, toc_seed: u32) -> () {
		if self.file_name_raw.starts_with(b"__key__.dat") {
			return ();
		}
		
		let mut mt = MersenneTwister::from_seed(toc_seed + self.index);
		let key = mt.next_u32();
		let shift = ((key >> 24) as u8)
			.wrapping_add((key >> 16) as u8)
			.wrapping_add((key >> 8) as u8)
			.wrapping_add(key as u8) as usize;
		self.file_name = Some(beaufort_cipher(self.file_name_raw, shift));
		
		()
	}
	
	pub fn deobfuscate_info(&mut self, blowfish: Rc<BlowfishLE>) -> Result<(), KifEntryError> {
		if self.decrypted == true {
			return Ok(());
		}
		self.decrypted = true;
		
		self.offset = self.offset.wrapping_add(self.index);
		
		let mut table_bytes = [0u8; 8];
		table_bytes[0..4].copy_from_slice(&self.offset.to_le_bytes());
		table_bytes[4..8].copy_from_slice(&self.length.to_le_bytes());
		
		let mut block = GenericArray::clone_from_slice(&table_bytes);
		blowfish.decrypt_block(&mut block);
		
		let decrypted_bytes = block.as_slice();
		self.offset = u32::from_le_bytes(decrypted_bytes[0..4].try_into()?);
		self.length = u32::from_le_bytes(decrypted_bytes[4..8].try_into()?);
		
		Ok(())
	}
	
	pub fn extract_data<F: Read + Seek>(&mut self, file: &mut F) -> Result<Vec<u8>, KifEntryError> {
		if self.file_name_raw.starts_with(b"__key__.dat") {
			return Err(KifEntryError::KeyData("Attempted to extract __key__.dat".to_string()));
		}
		if self.blowfish.is_none() {
			panic!("Entry couldn't be decrypted");
		}
		
		let blowfish = self.blowfish.as_ref().unwrap();
		
		let mut data: Vec<u8> = Vec::with_capacity(self.length as usize);
		let mut buffer = [0u8; 8];
		let mut bytes_remaining = self.length as usize;
		
		file.seek(SeekFrom::Start(self.offset as u64))?;
		
		while bytes_remaining > 0 {
			let bytes_to_read = min(bytes_remaining, 8);
			file.read_exact(&mut buffer[..bytes_to_read])?;
			
			if bytes_to_read == 8 {
				let mut block = GenericArray::clone_from_slice(&buffer);
				blowfish.decrypt_block(&mut block);
				data.extend_from_slice(block.as_slice());
			} else {
				data.extend_from_slice(&buffer[..bytes_to_read]);
			}
			
			bytes_remaining -= bytes_to_read;
		}
		Ok(data)
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
				break;
			}
			if byte.is_ascii_graphic() {
				file_name.push(byte as char);
			}
		}
		
		file_name
	}
}
