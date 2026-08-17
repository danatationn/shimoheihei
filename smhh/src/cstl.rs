use std::{fs::File, io::{self, Read, Seek, SeekFrom}, path::Path};

use thiserror::Error;

use crate::ReadExt;

/// CatScene Localization
/// Note: the latest sdk was released before nekopara 3 (first game with cstl)
/// so decomp is just my guess on how it would look like

#[derive(Debug)]
pub struct Cstl {
    pub lang_count: u32,
    pub languages: Vec::<String>,
    pub msg_count: u32,
}

#[derive(Error, Debug)]
pub enum CstlError {
	#[error("Io error: {0}")]
	IoError(#[from] io::Error),
	#[error("Invalid signature!")]
	InvalidSignature(),
}

impl Cstl {
	pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, CstlError> {
		let mut file = File::open(&path)?;

		Self::from_reader(&mut file)
	}

	pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, CstlError> {
	    let mut magic_buf = [0u8; 4];
		reader.read_exact(&mut magic_buf)?;

		if magic_buf != *b"CSTL" {
		    return Err(CstlError::InvalidSignature())
		}

		reader.seek(SeekFrom::Current(4))?;

		let lang_count = reader.read_ffint()?;

		let mut languages: Vec::<String> = Vec::new();
		for _ in 0..lang_count {
            languages.push(reader.read_ffstr()?);
		}

		let msg_count = reader.read_ffint()?;

		Ok(Self { lang_count, languages, msg_count })
	}

	pub fn decomp_script<R: Read + Seek>(&self, reader: &mut R) -> Result<Vec::<u8>, CstlError> {
    	// TODO make this prettier . will require some bs
        let mut string = String::new();

        for _ in 0..self.msg_count as i32 {
            for _ in 0..self.lang_count {
                let name = reader.read_ffstr()?;
                let content = reader.read_ffstr()?;

                string.push_str(&name);
                string.push('\t');
                string.push_str(&content);
                string.push('\n');
            }
        }

        Ok(string.into_bytes())
	}
}
