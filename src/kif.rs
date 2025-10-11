use std::path::PathBuf;
use std::fs::File;
use std::io::{Error, Read, Seek, SeekFrom};

pub struct KifFile {
    pub signature: [u8; 4],
    pub entry_count: u32,
    pub entries: Vec<KifFileEntry>,
    pub blowfish_key: u32,
}

impl KifFile {
    pub fn from_string(string: String) -> Result<KifFile, Error> {
        Self::from_path(&PathBuf::from(string))
    }

    pub fn from_path(path: &PathBuf) -> Result<KifFile, Error> {
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
            blowfish_key = entries[0].length;
        }

        Ok(KifFile{signature, entry_count, entries, blowfish_key})
    }
}

pub struct KifFileEntry {
    pub file_name_raw: [u8; 64],
    pub offset: u32,
    pub length: u32,
}

impl KifFileEntry {
    pub fn from_entry_pos(file: &mut File, pos: u32) -> Result<KifFileEntry, Error> {
        file.seek(SeekFrom::Start(8+72*pos as u64))?;

        let mut file_name_raw = [0u8; 64];
        file.read_exact(&mut file_name_raw)?;

        let mut offset_bytes = [0u8; 4];
        file.read_exact(&mut offset_bytes)?;
        let offset = u32::from_le_bytes(offset_bytes);

        let mut length_bytes = [0u8; 4];
        file.read_exact(&mut length_bytes)?;
        let length = u32::from_le_bytes(length_bytes);

        Ok(KifFileEntry{file_name_raw, offset, length})
    }
}