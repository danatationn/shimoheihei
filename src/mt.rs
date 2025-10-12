use std::num::Wrapping;
use anyhow::Result;

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: Wrapping<u32> = Wrapping(0x9908B0DF);
const UPPER_MASK: Wrapping<u32> = Wrapping(0x80000000);
const LOWER_MASK: Wrapping<u32> = Wrapping(0x7FFFFFFF);
const TEMPERING_MASK_B: Wrapping<u32> = Wrapping(0x9D2C5680);
const TEMPERING_MASK_C: Wrapping<u32> = Wrapping(0xEFC60000);

pub struct Mt {
	pub mt: [Wrapping<u32>; N],
	pub mti: usize,
}

impl Mt {
	pub fn from_seed(mut seed: u32) -> Result<Self> {
		let mut mt = [Wrapping(0u32); N];
		let mut mti: usize = N + 1;

		for i in 0..N {
			mt[i] = Wrapping(seed & 0xFFFF0000);
			seed = seed.wrapping_mul(69069).wrapping_add(1);
			mt[i] |= (seed & 0xFFFF0000) >> 16;
			seed = seed.wrapping_mul(69069).wrapping_add(1);
		}

		mti = N;

		Ok(Mt { mt, mti })
	}

	pub fn from_seed_arr(seed_array: [Wrapping<u32>; N]) -> Result<Self> {
		let mut mt = [Wrapping(0u32); N];
		let mut mti: usize = N + 1;

		for i in 0..N {
			mt[i] = seed_array[i];
		}
		mti = N;

		Ok(Mt { mt, mti } )
	}

	pub fn genrand(&mut self) -> Result<u32> {
		let mut y: Wrapping<u32>;
		let mag01: Vec<u32> = vec![0x0, MATRIX_A.0];

		if self.mti >= N {
			let mut kk: i32 = 0;

			if self.mti == (N + 1) {
				Self::from_seed(4357)?;
			}

			while kk < (N - M) as i32 {
				y = (self.mt[kk as usize] & UPPER_MASK) | (self.mt[(kk + 1) as usize] & LOWER_MASK);
				self.mt[kk as usize] = self.mt[(kk as usize + M)] ^ (y >> 1) ^ Wrapping(mag01[(y.0 & 0x1) as usize]);

				kk += 1;
			}
			while kk < (N - 1) as i32 {
				y = (self.mt[kk as usize] & UPPER_MASK) | (self.mt[(kk + 1) as usize] & LOWER_MASK);
				self.mt[kk as usize] = self.mt[kk as usize + (M - N)] ^ (y >> 1) ^ Wrapping(mag01[(y.0 & 0x1) as usize]);

				kk += 1;
			}
			y = (self.mt[N - 1] & UPPER_MASK) | (self.mt[0] & LOWER_MASK);
			self.mt[N - 1] = self.mt[M - 1] ^ (y >> 1) ^ Wrapping(mag01[(y.0 & 0x1) as usize]);

			self.mti = 0;
		}

	y = self.mt[self.mti];
	self.mti += 1;
	y ^= y >> 11;
	y ^= (y << 7) & TEMPERING_MASK_B;
	y ^= (y << 15) & TEMPERING_MASK_C;
	y ^= y >> 18;

	Ok(y.0)
	}
}