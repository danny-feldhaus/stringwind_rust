#![feature(path_file_prefix)]
#![feature(test)]

mod image_module;
mod string_path;
mod tri_vec;
mod utils;

use crate::image_module::lab::LabBuf;
use image::{DynamicImage, EncodableLayout};
use show_image::{create_window, ImageInfo, ImageView};
use string_path::StringPath;
#[show_image::main]
pub fn main() {
    log::debug!("Starting path generation...");
    const SETTINGS_PATH: &str = "src/tests/settings.toml";
    let mut sp = StringPath::from_file(SETTINGS_PATH).unwrap();
    //sp.fill_unique_pixels();
    let window = create_window("Image", Default::default()).unwrap();
    while sp.step() {
        if sp.cur_step % 50 == 0 {
            let binding =
                DynamicImage::ImageRgba32F(sp.strings_drawn.as_rgb_image_buffer()).into_rgba8();
            let window_image = ImageView::new(
                ImageInfo::rgba8(sp.strings_drawn.width(), sp.strings_drawn.height()),
                binding.as_bytes(),
            );
            window.set_image("input_image", window_image).unwrap();
        }
        if (sp.cur_step + 1) % 100 == 0 {
            sp.save_progress(true).unwrap();
            //sp.write_to_csv("src/tests/path.csv").unwrap();
            // sp.write_to_svg("src/tests/path.svg").unwrap();
        }
        println!(
            "{:?}:\t{:?} \tScores: {:?}",
            sp.cur_step,
            sp.cur_idxs,
            sp.cur_scores
                .iter()
                .map(|s| s.map_or("None, ".to_string(), |s| (100. * s).to_string() + "%, "))
                .collect::<String>()
        );
    }
    let _path = sp.cleaned_path();
    sp.write_to_csv("src/tests/path.csv").unwrap();
    sp.write_to_svg("src/tests/path.svg").unwrap();
}
