pub mod kif;
pub mod pe;
pub mod crypt;

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