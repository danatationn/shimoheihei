mod kif;
use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::fs;
use std::io::Error;
use crate::kif::KifFile;

fn main() {
    let args = env::args().nth(1).expect("ERROR: expected a path");
    let path = PathBuf::from(&args);
    if path.is_file() {
        let _ = info(path);
    }
    else if path.is_dir() {
        let paths = fs::read_dir(path).expect("ERROR: couldn't glob files");
        for result in paths {
            if let Ok(direntry) = result {
                let path = direntry.path();
                if let Some(ext) = path.extension() {
                    if ext == OsStr::new("int") {
                        let _ = info(path);
                    }
                }
            }
        }
    }
    else {
        panic!("ERROR: path is invalid (\"{}\")", &args)
    }
}

fn info(path: PathBuf) -> Result<(), Error> {
    if let Some(file_name) = path.file_name().unwrap().to_str() {
        println!("name:\t\t \"{file_name}\"");
    }
    let kif = KifFile::from_path(&path)?;
    println!("entries:\t {}", kif.entry_count);
    println!("blowfish key:\t {}", kif.blowfish_key);
    for (i, entry) in kif.entries.iter().enumerate() {
        print!("entry #{i}:\t ");
        for char in entry.file_name_raw {
            if char.is_ascii_graphic() {
                print!("{}", char as char);
            } else {
                print!(".");
            }
        }
        print!("\n");
    }

    Ok(())
}