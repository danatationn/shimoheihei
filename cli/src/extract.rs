use crate::ExtractArgs;
use core::kif::Kif;
use core::pe::Cs2PeFile;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Error, Write};
use std::{fs, io};

pub fn extract_kif(args: &ExtractArgs) -> Result<(), Error> {
    let path = args.kif_path.canonicalize()?;
    let parent = path.parent().unwrap().to_path_buf();
    let stem = path.file_stem().unwrap();

    if path.extension().unwrap() != OsStr::new("int") {
        panic!("File isn't an .int !!");
    }

    let dest = if let Some(dest) = &args.dest_path {
        dest
    } else {
        &parent.join(stem)
    };

    if dest.is_file() {
        panic!("The destination cannot be a file!");
    }
    if !dest.exists() {
        fs::create_dir_all(dest)?;
    }

    let mut kif_file = File::open(&path)?;
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

    if kif.blowfish_key.is_some() {
        println!("Extracting {:?} {}", stem, kif.entry_count);
        let mut i = 0;
        for mut entry in kif.entries {
            if entry.file_name_raw.starts_with(b"__key__.dat") {
                continue;
            }
            let file_path = dest.join(entry.get_file_name());

            i += 1;
            let fraction = i as f32 / kif.entry_count as f32;
            print!(
                "\r\x1B[K{:.1}% - {}",
                fraction * 100.0,
                entry.get_file_name()
            );
            io::stdout().flush()?;
            let mut file = File::create(file_path)?;
            let data = entry.extract_data(&mut kif_file).unwrap();
            file.write_all(&data)?;
        }
        println!("\r\x1B[K100.0% - Done!");
    } else {
        println!("no blowfhs");
    }

    Ok(())
}
