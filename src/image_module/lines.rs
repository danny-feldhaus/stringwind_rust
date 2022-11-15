use image::{Rgb32FImage,ImageBuffer,Rgb, Luma};
use rayon::{prelude::*};
use line_drawing::XiaolinWu;
use lerp::Lerp;
use palette::{Lab};

use super::color_conversion::*;

extern crate geo;

pub fn average_line(point_a: (f32, f32), point_b: (f32, f32), image: &ImageBuffer<Luma<f32>,Vec<f32>>) -> f32
{
    let xiao: Vec<((i32,i32), f32)> = XiaolinWu::<f32, i32>::new(point_a, point_b).collect();
    let sum : (f32,f32) = xiao.par_iter().fold(|| (0.,0.), |a: (f32, f32), ((x,y), value)|
    {   
        (a.0 + image.get_pixel(*x as u32,*y as u32)[0] * value, a.1 + value) 
    }).reduce(||(0.,0.), |a,b| (a.0+b.0,a.1+b.1));
    return sum.0 / sum.1;
}

pub fn draw_line(point_a: (f32, f32), point_b: (f32, f32), image: &mut Rgb32FImage, color: Rgb<f32>)
{
    let xiao = XiaolinWu::<f32, i32>::new(point_a, point_b);
    xiao.for_each(|((x,y), value)| 
        {
            let pixel = image.get_pixel_mut(x as u32,y as u32);
            for c in 0..3
            {
                pixel[c] = pixel[c].lerp(color[c], value);
            }
        }
    );
}

pub fn draw_line_lab(point_a: (f32, f32), point_b: (f32, f32), image: &mut Rgb32FImage, color: &Lab)
{
    let xiao = XiaolinWu::<f32, i32>::new(point_a, point_b);
    xiao.for_each(|((x,y), value)| 
        {
            let pixel = image.get_pixel_mut(x as u32,y as u32);
            pixel[0] = pixel[0].lerp(color.l, value);
            pixel[1] = pixel[1].lerp(color.a, value);
            pixel[2] = pixel[2].lerp(color.b, value);
        }
    );
}

//pub fn draw_line_img_col_difference(point_a: (f32, f32), point_b: (f32, f32), diff_image: &mut &ImageBuffer<Luma<f32>,Vec<f32>>, rgb_image: &Rgb32FImage,  color: Rgb<f32>)
pub fn line_diff_img_to_col(point_a: (f32, f32), point_b: (f32, f32), image: &Rgb32FImage, color: Rgb<f32>, is_lab: bool) -> f32
{
    let xiao: Vec<((i32,i32), f32)> = XiaolinWu::<f32, i32>::new(point_a, point_b).collect();
    let color_lab: Lab = if is_lab {color.lab_as_lab()} else {color.as_lab()};
    let sum : (f32,f32) = xiao.par_iter().fold(|| (0.,0.), |a: (f32, f32), ((x,y), value)|
    {   
        let pixel_lab: Lab = {
            if is_lab {image.get_pixel(*x as u32,*y as u32).lab_as_lab()}
            else      {image.get_pixel(*x as u32,*y as u32).as_lab()}
        };
        let d = {
            let ldiff = pixel_lab - color_lab; 
            (ldiff.l*ldiff.l + ldiff.a*ldiff.a + ldiff.b*ldiff.b).sqrt()
        };
        (a.0 + d * value, a.1 + value) 
    }).reduce(||(0.,0.), |a,b| (a.0+b.0,a.1+b.1));
    return sum.0 / sum.1;
}


//pub fn draw_line_img_col_difference(point_a: (f32, f32), point_b: (f32, f32), diff_image: &mut &ImageBuffer<Luma<f32>,Vec<f32>>, rgb_image: &Rgb32FImage,  color: Rgb<f32>)
pub fn line_diff_img_to_col_lab(point_a: (f32, f32), point_b: (f32, f32), image: &Rgb32FImage, color_lab: Lab) -> f32
{
    let xiao: Vec<((i32,i32), f32)> = XiaolinWu::<f32, i32>::new(point_a, point_b).collect();
    let sum : (f32,f32) = xiao.par_iter().fold(|| (0.,0.), |a: (f32, f32), ((x,y), value)|
    {   
        let pixel_lab : Lab = (*image.get_pixel(*x as u32,*y as u32)).lab_as_lab();
        let d = {
            let ldiff = pixel_lab - color_lab; 
            (ldiff.l*ldiff.l + ldiff.a*ldiff.a + ldiff.b*ldiff.b).sqrt()
        };
        (a.0 + d * value, a.1 + value) 
    }).reduce(||(0.,0.), |a,b| (a.0+b.0,a.1+b.1));
    return sum.0 / sum.1;
}

pub fn line_diff_img_to_img_lab(point_a: (f32, f32), point_b: (f32, f32), image_a: &Rgb32FImage, image_b: &Rgb32FImage) -> f32
{
    let xiao: Vec<((i32,i32), f32)> = XiaolinWu::<f32, i32>::new(point_a, point_b).collect();
    let sum : (f32,f32) = xiao.par_iter().fold(|| (0.,0.), |a: (f32, f32), ((x,y), value)|
    {   
        let pixel_a : Lab = (*image_a.get_pixel(*x as u32,*y as u32)).lab_as_lab();
        let pixel_b : Lab = (*image_b.get_pixel(*x as u32,*y as u32)).lab_as_lab();
        let d = {
            let ldiff = pixel_a - pixel_b; 
            (ldiff.l*ldiff.l + ldiff.a*ldiff.a + ldiff.b*ldiff.b).sqrt()
        };
        (a.0 + d * value, a.1 + value) 
    }).reduce(||(0.,0.), |a,b| (a.0+b.0,a.1+b.1));
    return sum.0 / sum.1;
}