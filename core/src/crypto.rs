/*
	-= MersenneTwister =-
	this tries to replicate the 29-10-99 variant of MT19937, used in cs2
	https://www.math.sci.hiroshima-u.ac.jp/m-mat/MT/VERSIONS/C-LANG/991029/mt19937int.c
*/

const N: usize = 624;
const M: usize = 397;
const DEFAULT_SEED: u32 = 4357;
const MATRIX_A: u32 = 0x9908B0DF;  // constant vector a
const UPPER_MASK: u32 = 0x80000000; // most significant w-r bits
const LOWER_MASK: u32 = 0x7FFFFFFF; // least significant r bits
const TEMPERING_MASK_B: u32 = 0x9d2c5680;
const TEMPERING_MASK_C: u32 = 0xefc60000;

pub struct MersenneTwister {
	pub state_array: [u32; N],
	pub mt_index: usize,
}

impl MersenneTwister {
	pub fn new() -> Self {
		// mti==N+1 means mt[N] is not initialized
		Self { state_array: [0u32; N], mt_index: N + 1 }
	}

	pub fn from_seed(mut seed: u32) -> Self {
		let mut state_array = [0u32; N];

		for i in 0..N {
			state_array[i] = seed & 0xFFFF0000;
			seed = seed.wrapping_mul(69069).wrapping_add(1);
			state_array[i] |= (seed & 0xFFFF0000).wrapping_shr(16);
			seed = seed.wrapping_mul(69069).wrapping_add(1);
		}

		Self { state_array, mt_index: N }
	}

	pub fn from_state_array(state_array: [u32; N]) -> Self {
		Self { state_array, mt_index: N }
	}

	pub fn next_u32(&mut self) -> u32 {
		let mut number: u32;
		let mag01 = [0x0, MATRIX_A];

		if self.mt_index >= N {
			let mut i: usize = 0;

			if self.mt_index == N + 1 {
				// if from_seed() has not been called a default initial seed is used
				let temp_mt = MersenneTwister::from_seed(DEFAULT_SEED);
				self.mt_index = temp_mt.mt_index;
				self.state_array = temp_mt.state_array;
			}

			while i < N - M {
				number = (self.state_array[i] & UPPER_MASK) | (self.state_array[i + 1] & LOWER_MASK);
				self.state_array[i] = self.state_array[i + M] ^ number.wrapping_shr(1) ^ mag01[(number & 0x1) as usize];
				i += 1;
			}
			while i < N - 1 {
				number = (self.state_array[i] & UPPER_MASK) | (self.state_array[i + 1] & LOWER_MASK);
				self.state_array[i] = self.state_array[i.wrapping_add(M.wrapping_sub(N))] ^ number.wrapping_shr(1) ^ mag01[(number & 0x1) as usize];
				i += 1;
			}
			number = (self.state_array[N - 1] & UPPER_MASK) | (self.state_array[0] & LOWER_MASK);
			self.state_array[N - 1] = self.state_array[M - 1] ^ number.wrapping_shr(1) ^ mag01[(number & 0x1) as usize];
			self.mt_index = 0;
		}

		number = self.state_array[self.mt_index];
		self.mt_index += 1;
		number ^= number.wrapping_shr(11);
		number ^= number.wrapping_shl(7) & TEMPERING_MASK_B;
		number ^= number.wrapping_shl(15) & TEMPERING_MASK_C;
		number ^= number.wrapping_shr(18);

		number
	}
}

/*
	-= Beaufort cipher =-
	Hi Beaufort
*/

const CHARSET: &[u8; 52] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

pub fn beaufort_cipher(text: [u8; 64], mut key: usize) -> [u8; 64] {
	let mut result = text;

	for i in 0..64 {
		let byte = text[i];

		if !byte.is_ascii_alphabetic() {
			// key += 1;
			continue
		}
		for j in 0..52 {
			if CHARSET[(52 - (key % 52) + j) % 52] == byte {
				result[i] = CHARSET[j];
			}
		}
		key += 1;
	}
	result
}
