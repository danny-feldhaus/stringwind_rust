mod image_module;
mod string_setting;
mod path_generation;

#[show_image::main]
pub fn main()
{
    let path = path_generation::generate_path("src/tests/settings.toml").unwrap();
    path.save_visual().unwrap();
}