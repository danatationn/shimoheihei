#![allow(dead_code)]

use std::{ffi::OsStr, path::{PathBuf}};

pub fn determine_path(src_path: PathBuf, dest_path: Option<PathBuf>, extension: Option<&OsStr>) -> PathBuf {
	let mut target = dest_path.unwrap_or(src_path);

	if let Some(ext) = extension {
		target.set_extension(ext);
	}

	target
}
