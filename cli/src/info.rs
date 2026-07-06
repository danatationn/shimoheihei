use crate::InfoArgs;
use smhh::anm::Anm;
use smhh::cst::Cst;
use smhh::fes::Fes;
use smhh::hgx::Hgx;
use smhh::kif::Kif;
use smhh::pe::Cs2Pe;

pub fn display_info(args: &InfoArgs) -> () {
    let path = args.file_path.canonicalize().expect("The path doesn't exist!");

    if let Ok(_) = Kif::from_path(&path) {
	    let kif = match Kif::from_path(&path) {
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
	   	println!("{}", anm);
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
