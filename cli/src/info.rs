use anyhow::{Result, bail};
use smhh::{anm::Anm, cst::Cst, fes::Fes, hgx::Hgx, kif::Kif, pe::Cs2Pe, zt::Zt};

use crate::InfoArgs;

pub fn from(args: &InfoArgs) -> Result<()> {
	let path = dunce::canonicalize(&args.file_path)?;
	let ext = path.extension()
		.unwrap_or_default()
		.to_string_lossy()
		.into_owned();

	if let Ok(anm) = Anm::from_path(&path) {
		println!("{anm:?}");
	} else if let Ok(cst) = Cst::from_path(&path) {
		println!("{cst:?}");
	} else if let Ok(fes) = Fes::from_path(&path) {
		println!("{fes:?}");
	} else if let Ok(hg3) = Hgx::from_path(&path) {
		println!("{hg3:?}");
	} else if let Ok(kif) = Kif::from_path(&path) {
		println!("{kif:?}");
	} else if let Ok(pe) = Cs2Pe::from_path(&path) {
		println!("{pe:?}");
	} else if ext == "exe" || ext == "bin" {
		bail!("This is not the main Cs2 executable.");
	} else if ext == "zt" && let Ok(zt) = Zt::from_path(&path) {
		println!("{zt:?}");
	} else {
		bail!("This format is not yet supported! (Supported formats: .int, .exe/.bin, .hg3, .anm, .fes, .cst, .zt)");
	}

	Ok(())
}
