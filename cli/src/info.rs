use crate::InfoArgs;
use smhh::{anm::Anm, cst::Cst, fes::Fes, hgx::Hgx, kif::Kif, pe::Cs2Pe};

pub fn display_info(args: &InfoArgs) -> () {
    let path = args.file_path.canonicalize().expect("The path doesn't exist!");

    if let Ok(_kif) = Kif::from_path(&path) {
	    eprintln!("WIP");
		return
    }
    else if let Ok(cs2_pe) = Cs2Pe::from_path(&path) {
	   	println!("{cs2_pe}");
    	return
    }
    else if let Ok(hgx) = Hgx::from_path(&path) {
	   	println!("{hgx}");
		for frame in hgx.frames {
			println!("\n{frame}");
			for tag in frame.tags {
				println!("{tag}");
			}
		}
		return
    }
    else if let Ok(anm) = Anm::from_path(&path) {
	   	println!("{:?}", anm);
		return
    }
    else if let Ok(fes) = Fes::from_path(&path) {
	   	println!("{:?}", fes);
		return
    }
    else if let Ok(cst) = Cst::from_path(&path) {
    	println!("{:?}", cst);
    }
    else {
    	println!("This format is not yet supported! (Supported formats: .int, .exe/.bin, .hg3, .anm, .fes)")
    }
}
