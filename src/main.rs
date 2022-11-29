#![feature(path_file_prefix)]

mod image_module;
mod string_path;
mod tri_vec;

use crate::string_path::string_setting::read_string_settings;
use crate::string_path::string_path::*;
use show_image::{ImageInfo,create_window,ImageView};
use crate::image_module::lab::LabBuf;
use image::{DynamicImage, EncodableLayout};
#[show_image::main]
pub fn main()
{
    const SETTINGS_PATH: &str = "src/tests/settings.toml";
    let settings = read_string_settings(SETTINGS_PATH).map_err(|e| e.to_string()).unwrap();
    let mut sp = StringPath::new(settings).unwrap();
    //sp.fill_unique_pixels();
    let window = create_window("Image", Default::default()).unwrap();
    while sp.step() 
    {
        if sp.cur_step % 50 == 0
        {
            let binding =  DynamicImage::ImageRgba32F(sp.strings_drawn.as_rgb_image_buffer()).into_rgba8();
            let window_image  = ImageView::new(ImageInfo::rgba8(sp.strings_drawn.width(), sp.strings_drawn.height()), binding.as_bytes());
            window.set_image("input_image", window_image).unwrap();
        }
        if (sp.cur_step+1) % 100 == 0
        {
            sp.save_visual(true).unwrap();
            sp.write_to_csv("src/tests/path.csv").unwrap();
            sp.write_to_svg("src/tests/path.svg").unwrap();
        }
        println!("{:?}:\t{:?} \tScores: {:?}",sp.cur_step, sp.cur_idxs, sp.cur_scores.iter().map(|s| s.map_or("None, ".to_string(), |s| (100.*s).to_string() + "%, ")).collect::<String>());

    }
    let _path = sp.cleaned_path();
    sp.write_to_csv("src/tests/path.csv").unwrap();
    sp.write_to_svg("src/tests/path.svg").unwrap();
}