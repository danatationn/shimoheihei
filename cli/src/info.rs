
use anyhow::Result;
use smhh::anm::Anm;

use crate::InfoArgs;

pub fn from(args: &InfoArgs) -> Result<()> {
	let path = args.file_path.canonicalize().expect("The path doesn't exist!");

	if let Ok(anm) = Anm::from_path(&path) {
		println!("{anm:?}");
	} else {
		println!("This format is not yet supported! (Supported formats: .int, .exe/.bin, .hg3, .anm, .fes)");
	}

	Ok(())
}
