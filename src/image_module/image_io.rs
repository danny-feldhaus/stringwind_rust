use image::ImageResult;
pub(crate) use image::{ImageBuffer, DynamicImage, Rgb};
use palette::Lab;
use super::color_conversion::*;
use csv::Reader;

pub fn read_lab(path: &str) -> Result<ImageBuffer<Rgb<f32>, Vec<f32>>, image::ImageError>
{
    let binding = image::open(path);
    match binding
    {
        Ok(img) => {
            let mut rgb_img = img.into_rgb32f().clone();
            rgb_img.into_lab();
            return Ok(rgb_img);
        },
        Err(e) => return Err(e)
    };
}

pub fn save_lab(path: &str, image: &ImageBuffer<Rgb<f32>, Vec<f32>>) -> ImageResult<()>
{
    let rgb_img = image.as_rgb();
    let binding = DynamicImage::ImageRgb32F(rgb_img);
    let rgb_img = binding.into_rgb8();
    match rgb_img.save(path)
    {
        Ok(i) => return Ok(i),
        Err(e) => return Err(e)
    };
}

pub fn get_color_name(color : &Lab) -> String
{
    let mut best_name = String::default();
    let mut best_score = 0_f32;
    let mut reader = Reader::from_path("src/data/color_names.csv").unwrap();

    for result in reader.records()
    {
        let result = result.unwrap();
        let name = &result[0];
        let l: f32 = result[1].parse().unwrap();
        let a: f32 = result[2].parse().unwrap();
        let b: f32 = result[3].parse().unwrap();
        let lab = Lab::new(l,a,b);
        let score = lab.similarity_to(&color);
        if score > best_score
        {
            best_score = score;
            best_name = name.to_string();
        }

    }
    return best_name
}