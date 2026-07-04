use std::io::Read;

pub mod anm;
pub mod cst;
pub mod crypt;
pub mod fes;
pub mod hgx;
pub mod kif;
pub mod pe;

pub trait ReadExt: Read {
	fn read_le<T: LeReadable>(&mut self) -> Result<T, std::io::Error>;
}

pub trait LeReadable {
	fn read_from<R: Read>(reader: &mut R) -> Result<Self, std::io::Error>
	where Self: Sized;
}

impl LeReadable for u32 {
	fn read_from<R: Read>(reader: &mut R) -> Result<Self, std::io::Error> {
		let mut buf = [0u8; 4];
		reader.read_exact(&mut buf)?;
		Ok(u32::from_le_bytes(buf))
	}
}

impl LeReadable for i32 {
	fn read_from<R: Read>(reader: &mut R) -> Result<Self, std::io::Error> {
		let mut buf = [0u8; 4];
		reader.read_exact(&mut buf)?;
		Ok(i32::from_le_bytes(buf))
	}
}

impl<R: Read> ReadExt for R {
	fn read_le<T: LeReadable>(&mut self) -> Result<T, std::io::Error> {
		T::read_from(self)
	}
}

#[cfg(test)]
mod tests {
	use crate::crypt::{beaufort_cipher, MersenneTwister};

	#[test]
	fn beaufort() {
		let mut message = [0u8; 64];
		for (i, byte) in b"Hello!".iter().enumerate() {
			message[i] = *byte;
		}
		let key = 255;
		let encrypted = beaufort_cipher(message, key);
		let decrypted = beaufort_cipher(encrypted, key);
		assert_eq!(message, decrypted);
	}

	#[test]
	fn mersenne() {
		let mut mt = MersenneTwister::from_seed(4357);
		assert_eq!(mt.next_u32(), 2867219139);
		assert_eq!(mt.next_u32(), 1585203162);
		assert_eq!(mt.next_u32(), 3113124129);
	}
}
