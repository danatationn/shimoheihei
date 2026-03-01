use smhh::pe::find_cs2_pe;
use crate::FindPeArgs;

pub fn find_pe(args: &FindPeArgs) -> () {
	let path = &args.folder_path;

	if let Some(pe_path) = find_cs2_pe(path) {
		println!("CS2 binary found at: \"{}\"", pe_path.to_string_lossy());
	} else {
		println!("CS2 binary couldn't be found");
	}
}