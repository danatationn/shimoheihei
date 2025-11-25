use crate::InfoArgs;
use core::kif::Kif;
use core::pe::Cs2PeFile;
use core::pe::check_cs2_pe_path;

pub fn display_info(args: &InfoArgs) -> () {
    let path = args.file_path.canonicalize().expect("The path doesn't exist!");
    let ext = path.extension().unwrap().to_str().unwrap();

    if ext == "int" {
	    let kif = match Kif::from(&path) {
			 Ok(kif) if kif.blowfish_key.is_some() => {
		        if args.toc_seed.is_some() {
		            Kif::from_path_with_seed(&path, args.toc_seed.unwrap()).unwrap()
		        } else if args.pe_path.is_some() {
		            let cs2_pe = Cs2PeFile::from_path(&args.pe_path.as_ref().unwrap()).unwrap();
		            Kif::from_path_with_seed(&path, cs2_pe.toc_seed).unwrap()
		        } else {
		           	println!("The KIF is encrypted and no decryption keys were provided! The file names will be garbled but the data won't be.");
		           	kif
				}
		    },
		    Ok(kif) => kif,
		    Err(_) => panic!("The KIF is invalid!"),
		};

        println!("KIF archive with {} entries detected", kif.entry_count);
        if let Some(blowfish_key) = kif.blowfish_key {
            println!(
                "Blowfish key: {}",
                u32::from_le_bytes(blowfish_key)
            );
        } else {
            println!("Archive is unencrypted");
        }
        if let Some(toc_seed) = kif.toc_seed {
            println!("TOC seed: {toc_seed}");
        }

        print!("Entries: ");
        for (i, entry) in kif.entries.iter().enumerate() {
            print!("{i}. {}; ", entry.get_file_name());
        }
    } else if ext == "exe" || ext == "bin" {
        if check_cs2_pe_path(&path).unwrap() == false {
            panic!("The PE is invalid!");
        }

        let cs2_pe_file = Cs2PeFile::from_path(&path).unwrap();
        println!("TOC seed: {}", cs2_pe_file.toc_seed);
        println!(
            "Key code: {}",
            String::from_utf8_lossy(&cs2_pe_file.key_code)
        );
        println!("VCode: {}", String::from_utf8_lossy(&cs2_pe_file.v_code));
        println!("VCode2: {}", String::from_utf8_lossy(&cs2_pe_file.v_code2));
    } else {
        panic!("This format is not yet supported!");
    }
}
