use crate::image_module::lines::draw_line_lab;
use super::string_setting::*;
use super::string_path::*;
use super::super::tri_vec::*;
use super::super::image_module::lab::{LabImageBuffer, LabaImageBuffer, LabBuf};

use geo::{Line, coord, algorithm::line_intersection::line_intersection, LineIntersection};
use image:: {RgbaImage, Rgba, Rgba32FImage, ImageResult, EncodableLayout, DynamicImage};
use palette::{Lab, Laba, Mix};
use std::path::Path;
use line_drawing::XiaolinWu;
use show_image::{ImageView, ImageInfo, create_window};
use rand::distributions::{WeightedIndex,Distribution};



trait StringPathTests
{   
    fn fill_unique_pixels(&self);
}
/*
impl StringPathTests for StringPath
{
    fn fill_unique_pixels(&self)
    {
        let mut coverage_map = image::GrayImage::new(self.strings_drawn.width(), self.strings_drawn.height());
        for x in 0..self.pin_positions.len()
        {
            for y in x+1..self.pin_positions.len()
            {
                let pin_a = self.pin_positions[x];
                let pin_b = self.pin_positions[y];
                let line =  XiaolinWu::<f32, i32>::new(pin_a, pin_b);
                for ((x,y), weight) in line
                {
                    if weight > 0.3
                    {
                        coverage_map.get_pixel_mut(x as u32, y as u32)[0] += 1;
                    }
                }
            }
        }
        coverage_map.save(self.output_path.clone() + "coverage_map.png").unwrap();
        let max_coverage = coverage_map.pixels().max_by(|a,b| a[0].cmp(&b[0])).unwrap()[0];
        let mut coverage_distribution = vec![0; max_coverage as usize + 1];
        let mut pixels_in_circle = 0;
        let max_d_from_center = self.pin_radius * coverage_map.width() as f32 / 2.;
        for (x,y, pixel) in coverage_map.enumerate_pixels()
        {
            let d_from_center = (((x as f32 - coverage_map.width() as f32/2. )).powf(2.) + ((y as f32 - coverage_map.height() as f32/2.)).powf(2.)).sqrt();
            if d_from_center <= max_d_from_center
            {
                coverage_distribution[pixel[0] as usize] += 1;
                pixels_in_circle += 1;
            }
        }
        let unique_string_coverage_map = RgbaImage::new(coverage_map.width(), coverage_map.height());
        let mut unique_lines = 0;
        for x in 0..self.pin_positions.len()
        {
            for y in x+1..self.pin_positions.len()
            {
                let pin_a = self.pin_positions[x];
                let pin_b = self.pin_positions[y];
                let line =  XiaolinWu::<f32, i32>::new(pin_a, pin_b);
                let mut uniques_in_line = 0;
                for ((x,y), weight) in line
                {
                    if weight > 0.3
                    {
                        if coverage_map.get_pixel_mut(x as u32, y as u32)[0] == 1 
                        {
                            uniques_in_line += 1;
                        }
                    }
                }
                if uniques_in_line != 0
                {
                    unique_lines += 1
                }
            }
        }
        println!("Total lines containing unique pixels: {unique_lines}");

    }
}
*/

pub fn generate_path(settings_path: &'static str) -> Result<StringPath, String>
{
    let settings = read_string_settings(settings_path).map_err(|e| e.to_string())?;
    let mut sp = StringPath::new(settings)?;
    //sp.fill_unique_pixels();
    let window = create_window("Image", Default::default()).unwrap();
    while sp.step() 
    {
        if sp.cur_step % 100 == 0
        {
            let binding =  DynamicImage::ImageRgb32F(sp.strings_drawn.as_rgb_image_buffer()).into_rgb8();
            let window_image  = ImageView::new(ImageInfo::rgb8(sp.strings_drawn.width(), sp.strings_drawn.height()), binding.as_bytes());
            window.set_image("input_image", window_image).unwrap();
        }
        if (sp.cur_step+1) % 500 == 0
        {
            sp.save_visual().unwrap();
        }
        println!("{:?}:\t{:?} \tScores: {:?}%",sp.cur_step, sp.cur_idxs, sp.cur_scores);
    }
    return Ok(sp);
}

