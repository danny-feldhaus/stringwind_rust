use image::{Rgb32FImage,ImageBuffer,Rgb, Luma};
use rayon::{prelude::*};
use palette::{IntoColor, Lab, Srgb};

use super::color_conversion::{PaletteToImage, ImageConversion};

pub fn dist_3d(a: &[f32], b: &[f32]) -> f32
{
    let x_diff = a[0] - b[0];
    let y_diff = a[1] - b[1];
    let z_diff = a[2] - b[2];
    return (x_diff*x_diff + y_diff*y_diff + z_diff*z_diff).sqrt();
}

pub fn dist_2d(a: (f32,f32), b: (f32,f32)) -> f32
{
    let x_diff = b.0-a.0;
    let y_diff = b.1-a.1;
    return (x_diff*x_diff + y_diff*y_diff).sqrt();
}

pub fn avg_img_to_col(buffer : &Rgb32FImage, color : Rgb<f32>) -> f32
{
    let color_arr = [color[0], color[1], color[2]];
    let s : f32 = buffer.par_chunks(3).
        fold(|| 0., |a, p|
            {
                a + 
                dist_3d(p, &color_arr)
            }
        ).sum();
    return s / (buffer.len() as f32);
}

pub fn avg_img_to_img(image_a: &Rgb32FImage, image_b: &Rgb32FImage) -> f32
{
    let a_chunks = image_a.par_chunks(3);
    let b_chunks = image_b.par_chunks(3);
    let ab_zip = a_chunks.zip(b_chunks);
    let s : f32 = ab_zip.
        fold(|| 0., |out, (a,b)|
            {
                out + dist_3d(a, b)
            }
        ).sum();
    return s / (image_a.len() as f32);
}

pub fn diff_img_to_col(image : &Rgb32FImage, color : Rgb<f32>) -> ImageBuffer<Luma<f32>, Vec<f32>>
{
    let color_lab: Lab = Srgb::new(color[0],color[1],color[2]).into_color();
    let mut diff: ImageBuffer<Luma<f32>, Vec<f32>> = ImageBuffer::new(image.width(), image.height());
    let rgb_iter = image.par_chunks(3);
    let diff_iter = diff.par_iter_mut();
    let zip_iter = diff_iter.zip(rgb_iter);
    zip_iter.for_each(|z| *(z.0) =
        {
            let pix_lab : Lab = Srgb::new(z.1[0], z.1[1], z.1[2]).into_color();
            let l = pix_lab.l - color_lab.l;
            let a = pix_lab.a - color_lab.a;
            let b = pix_lab.b - color_lab.b;
            (l*l + a*a + b*b).sqrt()
        }
    );
    return diff;
}

pub fn diff_img_to_img(image_a : &Rgb32FImage, image_b : &Rgb32FImage) -> ImageBuffer<Luma<f32>, Vec<f32>>
{
    let mut diff: ImageBuffer<Luma<f32>, Vec<f32>> = ImageBuffer::new(image_a.width(), image_a.height());
    let diff_iter = diff.par_iter_mut();
    let image_a_iter = image_a.par_chunks(3);
    let image_b_iter = image_b.par_chunks(3);
    let zip_iter = (image_a_iter.zip(image_b_iter)).zip(diff_iter);
    zip_iter.for_each(|((p_a, p_b), p_diff)| 
    {
        *(p_diff) = dist_3d(p_a, p_b)
    });
    return diff;
}

pub fn rgb_as_lab(rgb_image : &Rgb32FImage) -> Rgb32FImage
{
    let mut lab_image = Rgb32FImage::new(rgb_image.width(), rgb_image.height());
    let rgb_chunks = rgb_image.par_chunks(3);
    let lab_chunks = lab_image.par_chunks_mut(3);
    let zip_iter = lab_chunks.zip(rgb_chunks);
    zip_iter.for_each(|(p_lab, p_rgb)|
    {
        let cs_lab : Lab = Srgb::new(p_rgb[0], p_rgb[1], p_rgb[2]).into_color();
        p_lab[0] = cs_lab.l;
        p_lab[1] = cs_lab.a;
        p_lab[2] = cs_lab.b;
    });
    return lab_image;
}

pub fn lab_as_rgb(lab_image : &Rgb32FImage) -> Rgb32FImage
{
    let mut rgb_image = Rgb32FImage::new(lab_image.width(), lab_image.height());
    let rgb_chunks = rgb_image.par_chunks_mut(3);
    let lab_chunks = lab_image.par_chunks(3);
    let zip_iter = rgb_chunks.zip(lab_chunks);
    zip_iter.for_each(|(p_rgb, p_lab)|
    {
        let cs_rgb : Srgb = Lab::new(p_rgb[0], p_rgb[1], p_rgb[2]).into_color();
        p_rgb[0] = cs_rgb.red;
        p_rgb[1] = cs_rgb.green;
        p_rgb[2] = cs_rgb.blue;
    });
    return rgb_image;
}