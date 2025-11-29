use crate::InfoArgs;
use core::hgx::Hgx;
use core::kif::Kif;
use core::pe::Cs2Pe;

pub fn display_info(args: &InfoArgs) -> () {
    let path = args.file_path.canonicalize().expect("The path doesn't exist!");

    if let Ok(_) = Kif::from(&path) {
	    let kif = match Kif::from(&path) {
			 Ok(kif) if kif.blowfish_key.is_some() => {
		        if args.toc_seed.is_some() {
		            Kif::from_path_with_seed(&path, args.toc_seed.unwrap()).unwrap()
		        } else if args.pe_path.is_some() {
		            let cs2_pe = Cs2Pe::from_path(&args.pe_path.as_ref().unwrap()).unwrap();
		            Kif::from_path_with_seed(&path, cs2_pe.toc_seed).unwrap()
		        } else {
		           	println!("The KIF is encrypted and no decryption keys were provided! The file names will be garbled but the data won't be.");
		           	kif
				}
		    },
		    Ok(kif) => kif,
		    Err(_) => panic!("The KIF is invalid!"),
		};

		println!("{kif}");
		for entry in kif.entries {
			println!("{entry}");
		}
		return
    }

    if let Ok(cs2_pe) = Cs2Pe::from_path(&path) {
	   	println!("{cs2_pe}");
    	return
    }

    if let Ok(hgx) = Hgx::from(&path) {
	   	println!("{hgx}");
		for frame in hgx.frames {
			println!("{frame}");
			for tag in frame.tags {
				println!("{tag}");
			}
		}
		return
    }

    println!("This format is not yet supported! (Supported formats: .int, .exe, .hg2, .hg3)")
}
