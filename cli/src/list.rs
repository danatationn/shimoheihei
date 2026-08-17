use anyhow::{Result, bail};
use smhh::{kif::Kif, zt::{Zt, ZtNode}};

use crate::{ArgsType, ListArgs, get_toc_seed};

pub fn from(args: &ListArgs) -> Result<()> {
	let path = dunce::canonicalize(&args.file_path)?;

	if let Ok(mut kif) = Kif::from_path(&path) {
		let toc_seed = get_toc_seed(ArgsType::List(args.clone()), Some(path));

		for entry in &mut kif.entries {
			if let Some(toc_seed) = toc_seed && kif.blowfish_key.is_some() {
				entry.deobfuscate_file_name(toc_seed);
			}

			println!("{}", entry.get_file_name());
		}

		return Ok(())
	}
	else if let Ok(zt) = Zt::from_path(&path) {
		let total = zt.root_nodes.len();

		for (i, node) in zt.root_nodes.iter().enumerate() {
			let is_last = i == total - 1;
			write_zt_node(node, "", is_last);
		}

		return Ok(())
	}

	bail!("This format is not supported! (Supported formats: .int, .zt)");
}

fn write_zt_node(node: &ZtNode, prefix: &str, is_last: bool) {
	let connector = if is_last { "└── " } else { "├── " };
	let child_prefix_extra = if is_last { "    " } else { "│   " };

	match node {
		ZtNode::File { name, entry: _ } => {
			println!("{prefix}{connector}{name}");
		}
		ZtNode::Directory { name, children } => {
			println!("{prefix}{connector}{name}/");

			let next_prefix = format!("{prefix}{child_prefix_extra}");
			let count = children.len();
			for (i, child) in children.iter().enumerate() {
				let child_is_last = i == count - 1;
				write_zt_node(child, &next_prefix, child_is_last);
			}
		}
	}
}
