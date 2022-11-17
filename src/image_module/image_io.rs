use image::ImageResult;
pub(crate) use image::{ImageBuffer, DynamicImage, Rgb};
use super::color_conversion::*;


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