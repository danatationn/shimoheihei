
use anyhow::Result;
use smhh::{anm::Anm, kcs::Kcs};

use crate::InfoArgs;

pub fn from(args: &InfoArgs) -> Result<()> {
	let path = args.file_path.canonicalize().expect("The path doesn't exist!");
	let ext = path.extension()
		.unwrap_or_default()
		.to_string_lossy()
		.into_owned();

	if let Ok(anm) = Anm::from_path(&path) {
		println!("{anm:?}");
	} else if ext == "kcs" && let Ok(kcs) = Kcs::from_path(&path) {
		todo!()
	} else {
		println!("This format is not yet supported! (Supported formats: .int, .exe/.bin, .hg3, .anm, .fes)");
	}

	Ok(())
}
