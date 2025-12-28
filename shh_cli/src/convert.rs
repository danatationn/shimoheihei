use shh_core::hgx::{Hgx, HgxTag};
use std::{fs::File, io::Write};
use image;

use crate::ConvertArgs;

pub fn convert(args: &ConvertArgs) {
	let file_path = args.file_path.canonicalize().expect("The path doesn't exist!");
	let ext = args.dest_path.extension().unwrap().to_str().unwrap();

	if let Ok(hgx) = Hgx::from(&file_path) {
		let format = match ext.to_lowercase().as_str() {
			"png" => image::ImageFormat::Png,
			"bmp" => image::ImageFormat::Bmp,
			"avif" => image::ImageFormat::Avif,
			"jpeg" => image::ImageFormat::Jpeg,
			"jpg" => image::ImageFormat::Jpeg,
			"webp" => image::ImageFormat::WebP,
			"tiff" => image::ImageFormat::Tiff,
			"tga" => image::ImageFormat::Tga,
			"gif" => image::ImageFormat::Gif,
			_ => panic!("This format is not supported! (png, bmp, avif, jpg, webp, tiff, tga, gif)")
		};

		let mut hgx_file = File::open(file_path).unwrap();
		for frame in hgx.frames {
			let mut img_found = false;
			let std_info = match &frame.tags[0] {
				HgxTag::StdInfo(std_info) => std_info,
				_ => panic!("The hgx file is invalid!")
			};

			for tag in &frame.tags {
				if let HgxTag::Img(img) = &tag {
					img_found = true;
					let data = img.convert(&mut hgx_file, std_info, format).unwrap();

					let img_path = args.dest_path.with_extension(ext);
					let mut img_file = File::create(&img_path).unwrap();
					img_file.write(&data).unwrap();
					println!("Extracted frame {} at {img_path:?}", frame.id);
				}
			}
			if img_found == false {
				println!("Couldn't find the image for frame {}", frame.id);
			}
		}
	}
}
