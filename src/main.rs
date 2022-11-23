#![feature(path_file_prefix)]

mod image_module;
mod string_path;
mod tri_vec;

#[show_image::main]
pub fn main()
{
    let path = string_path::path_generation::generate_path("src/tests/settings.toml").unwrap();
    path.save_visual().unwrap();
}