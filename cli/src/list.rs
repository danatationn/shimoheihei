use std::{fmt::Write, thread};

use anyhow::Result;
use minus::Pager;
use smhh::{kif::Kif, zt::{Zt, ZtNode}};

use crate::{ArgsType, ListArgs, get_toc_seed};

pub fn from(args: &ListArgs) -> Result<()> {
	let path = args.file_path.canonicalize().expect("The path doesn't exist!");

	let pager = Pager::new();
	let mut pager_clone = pager.clone();
	pager.set_prompt(path.to_string_lossy().into_owned())?;

	if let Ok(mut kif) = Kif::from_path(&path) {
		let toc_seed = get_toc_seed(ArgsType::List(args.clone()));

		thread::spawn(move || {
			for entry in &mut kif.entries {
				if let Some(toc_seed) = toc_seed && kif.blowfish_key.is_some() {
					entry.deobfuscate_file_name(toc_seed);
				}

				let _ = writeln!(pager_clone, "{}", entry.get_file_name());
				// Make these one-liners. One-liners that look good
				/* if kif.blowfish.is_some() && !entry.file_name_raw.starts_with(b"__key__.dat") {
					let _ = writeln!(pager_clone, "\tOffset: {} bytes", entry.offset);
					let _ = writeln!(pager_clone, "\tLength: {} bytes", entry.length);
				} else {
					let _ = writeln!(pager_clone, "\tMT seed: {}", entry.length);
				} */
			}
		});

		minus::dynamic_paging(pager)?;

		return Ok(())
	}
	else if let Ok(zt) = Zt::from_path(&path) {
		thread::spawn(move || {
			let total = zt.root_nodes.len();

			for (i, node) in zt.root_nodes.iter().enumerate() {
				let is_last = i == total - 1;
				write_zt_node(&mut pager_clone, node, "", is_last);
			}
		});

		minus::dynamic_paging(pager)?;

		return Ok(())
	}

	println!("This format is not supported! (Supported formats: .int, .zt)");
	Ok(())
}

fn write_zt_node<W: Write>(writer: &mut W, node: &ZtNode, prefix: &str, is_last: bool) {
	let connector = if is_last { "└── " } else { "├── " };
	let child_prefix_extra = if is_last { "    " } else { "│   " };

	match node {
		ZtNode::File { name, entry: _ } => {
			let _ = writeln!(writer, "{prefix}{connector}{name}");
		}
		ZtNode::Directory { name, children } => {
			let _ = writeln!(writer, "{prefix}{connector}{name}/");

			let next_prefix = format!("{prefix}{child_prefix_extra}");
			let count = children.len();
			for (i, child) in children.iter().enumerate() {
				let child_is_last = i == count - 1;
				write_zt_node(writer, child, &next_prefix, child_is_last);
			}
		}
	}

}
