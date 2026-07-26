use std::{fmt::Write, thread};

use anyhow::{Result, bail};
use minus::Pager;
use smhh::{kif::Kif, zt::{Zt, ZtNode}};

use crate::{ArgsType, ListArgs, get_toc_seed};

pub fn from(args: &ListArgs) -> Result<()> {
	let path = dunce::canonicalize(&args.file_path)?;

	let pager = Pager::new();
	let mut pager_clone = pager.clone();
	pager.set_prompt(path.to_string_lossy().into_owned())?;

	if let Ok(mut kif) = Kif::from_path(&path) {
		let toc_seed = get_toc_seed(ArgsType::List(args.clone()), Some(path));

		thread::spawn(move || {
			for entry in &mut kif.entries {
				if let Some(toc_seed) = toc_seed && kif.blowfish_key.is_some() {
					entry.deobfuscate_file_name(toc_seed);
				}

				let _ = writeln!(pager_clone, "{}", entry.get_file_name());
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

	bail!("This format is not supported! (Supported formats: .int, .zt)");
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
