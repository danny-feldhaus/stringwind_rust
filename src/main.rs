mod image_module;


use std::thread::current;

use image::{Rgb, Rgb32FImage, ImageBuffer, Luma};
use show-image::{ImageView, ImageInfo, create_window};
use config::Config;
use itertools::{Itertools};
use palette::{Srgb, Lab, IntoColor, Pixel};
use rand::seq::IteratorRandom;
use rayon::prelude::*;
use rand::{self, Rng};

use line_drawing::XiaolinWu;

use image_module::img_analysis::{rgb_as_lab, lab_as_rgb, diff_img_to_img};
use image_module::lines::{line_diff_img_to_col_lab,line_diff_img_to_img_lab};

use crate::image_module::lines::{draw_line_lab};
use crate::image_module::color_conversion::ImageToPalette;

/*
mod lines;
use std::f32::consts::PI;
use itertools::Itertools;
use image::{DynamicImage, GenericImageView, Rgb};
use img_analysis::*;
use lines::*;
*/
/*
1) Image preparetion
    xLoad image from file
    xConvert to Lab color space
2) Color preparation
    xConvert given RGB string colors to Lab
3) Dynamic image creation
    xCurrent difference map
    xCurrent string map (Lab)
4) Pin creation
*/

fn main()
{
    let settings = Config::builder()
        .add_source(config::File::with_name("src/tests/settings.toml"))
        .build()
        .unwrap();
    let lab_image = image::open(&settings.get_string("filename").unwrap())
        .unwrap()//image::DynamicImage::ImageRgb32F(open_lab(&settings.get_string("filename").unwrap()))
        .resize(2048,2048, image::imageops::Nearest)
        .into_rgb32f();
    let lab_image = rgb_as_lab(&lab_image);
    image::DynamicImage::ImageRgb32F(lab_as_rgb(&lab_image))
        .into_rgb8()
        .save("src/tests/images/lab_img.png")
        .unwrap(); 
    let str_colors = parse_settings_lab_color_array(&settings.get("string_colors").unwrap());
    let bg_color= parse_settings_lab_color(&settings.get("background_color").unwrap());
    let pin_count = settings.get_int("pin_count").unwrap() as usize;
    let pin_radius = settings.get_float("pin_radius").unwrap() as f32;
    let line_count = settings.get_int("line_count").unwrap() as usize;

    let mut str_img = Rgb32FImage::new(lab_image.width(), lab_image.height());
    str_img.pixels_mut().for_each(|p|{p[0] = bg_color.l; p[1] = bg_color.a; p[2] = bg_color.b});
    //let _diff_img = diff_img_to_img(&lab_image, &str_img);

    let pins = pin_circle(pin_count, lab_image.dimensions(), pin_radius);
    let pin_combos = (0..pin_count).tuple_combinations::<(_,_)>().collect_vec();

    const MAX_DIFF : f32 = 375.;
    let potential_scores = pin_combos.iter().map(|(pin_a, pin_b)|
        {
            line_diff_img_to_col_lab(pins[*pin_a], pins[*pin_b], &lab_image, str_colors[0]) / MAX_DIFF
        }).collect_vec();
    let mut cur_pin: usize = 0;
    let mut score : f32 = 0.;
    println!("{line_count}");
    let mut path: Vec<usize> = Vec::with_capacity(line_count);
    path.resize(line_count, 0);
    path[0] = cur_pin;
    for i in 0..line_count
    {
        let next_pin = best_path_from(cur_pin, &pins, &pin_combos, &lab_image, &str_img, &potential_scores, &mut score);
        draw_line_lab(pins[cur_pin], pins[next_pin], &mut str_img, &str_colors[0]);
        println!("{i}: {:?}->{:?}, score: {score}", cur_pin, next_pin);
        cur_pin = next_pin;
        path[i] = cur_pin;
    }

    image::DynamicImage::ImageRgb32F(lab_as_rgb(&str_img))
        .into_rgb8()
        .save("src/tests/images/str_img.png")
        .unwrap(); 
    let path_vis = visualize_string_img(&path, &pins, lab_image.dimensions()).into_raw();
    let mut buf : [u16; 2048*2048];
    for (i,b) in path_vis.iter().enumerate()
    {
        buf[i] = *b;
    }
    image::save_buffer("src/tests/images/path_vis.png", &buf, path_vis.width(), path_vis.height(), image::ColorType::L16);

}

fn visualize_string_img(path: &Vec<usize>, pins: &Vec<(f32,f32)>, dimensions : (u32, u32)) -> ImageBuffer<Luma<u16>, Vec<u16>>
{
    let mut img = ImageBuffer::<Luma<u16>, Vec<u16>>::new(dimensions.0,dimensions.1);
    img.fill(0);
    for i in 1..path.len()
    {
        let pin_a = pins[path[i-1]];
        let pin_b = pins[path[i]];
        let xiao = XiaolinWu::<f32, i32>::new(pin_a, pin_b);
        xiao.for_each(|((x,y), value)| 
        {
            let p = img.get_pixel_mut(x as u32,y as u32);
            if(p[0] == 0) {p[0] = i as u16};
        });
    }
    return img;
}

fn best_path_from(pin_idx: usize, pins: &Vec<(f32,f32)>, pin_combos: &Vec<(usize, usize)>, lab_image: &Rgb32FImage, str_imge: &Rgb32FImage, potential_scores: &Vec<f32>, score: &mut f32) -> usize
{
    const MAX_DIFF : f32 = 375.;
    const SCORE_THRESH : f32 = -1.;
    let mut rng = rand::thread_rng();
    let z_score_combo = pin_combos.iter().zip(potential_scores.iter());

    let mut flt_score = |point_a: usize, point_b: usize, potential_score: f32| -> f32
    {
        //let potential_difference = line_diff_img_to_col_lab(pins[point_a], pins[point_b], lab_image, *color) / MAX_DIFF;
        let current_difference = line_diff_img_to_img_lab(pins[point_a], pins[point_b], lab_image, str_imge) / MAX_DIFF;
        let progress = current_difference - potential_score;
        
        //println!("{point_a}->{point_b}: Cur: {current_difference}, Pot: {potential_difference}, Prog: {progress}, Score: {:?}", (1.-potential_difference) * (1. + progress));
        if progress >= SCORE_THRESH
            {progress}// * rng.gen_range(0_f32..1_f32)}//(1.-potential_difference) * ((1. + progress)/2.).powf(1./2.)
        else {
            0.
        }
        };
    
    let max_combo = z_score_combo.max_by_key(|((point_a,point_b),potential_score)|
        -> u32 {
            if (*point_a == pin_idx )|| (*point_b == pin_idx)
            {
                ((flt_score(*point_a,*point_b,**potential_score) + 1.) * 10000.) as u32
            }
            else {
                0
            }
        }
    ).unwrap();
    *score = *max_combo.1;//flt_score(max_combo.0.0, max_combo.0.1, *max_combo.1);
    return if max_combo.0.0 == pin_idx {max_combo.0.1} else {max_combo.0.0};
}

fn open_lab(filename: &String) -> image::ImageBuffer<Rgb<f32>, Vec<f32>>
{
    let rgb_image = image::open(filename)
        .unwrap()
        .into_rgb32f();
    return rgb_as_lab(&rgb_image);
}

fn parse_settings_lab_color_array(s_colors_rgb: &config::Value) -> Vec<Lab>
{
    let rgb_colors = parse_settings_color_array(s_colors_rgb);
    let mut lab_colors: Vec<Lab> = vec![Lab::new(0.,0.,0.); rgb_colors.len()];
    for i in 0..lab_colors.len()
    {
        lab_colors[i] = rgb_colors[i].as_lab();
    }
    return lab_colors;
}

fn parse_settings_lab_color(s_color_rgb: &config::Value) -> Lab
{
    let rgb_color: Srgb = parse_settings_color(s_color_rgb);
    return  rgb_color.into_color();
}

fn parse_settings_color_array(s_colors: &config::Value) -> Vec<Srgb>
{
    let s_colors = s_colors
        .clone()
        .into_array()
        .unwrap();
    let mut colors = vec![Srgb::new(0.,0.,0.); s_colors.len()];

    for (index, s_color) in s_colors.iter().enumerate()
    {
        colors[index] = parse_settings_color(s_color);
    }
    return colors;
}

fn parse_settings_color(s_color: &config::Value) -> Srgb
{
    let c_arr = s_color.clone().into_array().unwrap();
    let color = Srgb::new(
        c_arr[0].clone().into_float().unwrap() as f32,
        c_arr[1].clone().into_float().unwrap() as f32,
        c_arr[2].clone().into_float().unwrap() as f32,
    );
    return color;
}

/*
fn main()
{
    let pin_count = 250;
    let mut image = image::open("src/tests/images/vangogh.png").unwrap().into_rgb32f();
    let mut diff_img = diff_img_to_col(&image, Rgb([0.,0.,0.]));



let pins = pin_circle(pin_count, image.dimensions(), 0.95);
    let mut line_average: f32;
    let pin_combos = pins.iter().tuple_combinations::<(_,_)>();
    for (p_a, p_b) in pin_combos
    {
        line_average = average_line(*p_a, *p_b, &diff_img);
        draw_line(*p_a, *p_b, &mut image, image::Rgb([line_average,line_average,line_average]));
        println!("Line average similarity between ({:.1},{:.1}) and ({:.1},{:.1}): {:.2}%",p_a.0, p_a.1, p_b.0, p_b.1, (1.-line_average)*100.);
    }
    let dynamic_img = DynamicImage::ImageRgb32F(image);
    println!("{:?}",dynamic_img.get_pixel(5000,5000));
    let save_img = dynamic_img.into_rgb8();
    save_img.save("src/tests/images/all_lines.png").unwrap();
}
*/

fn pin_circle(pin_count: usize, dimensions : (u32, u32), radius: f32) -> Vec<(f32,f32)>
{
    let mut pins = vec![(0.,0.);pin_count];
    let center = ((dimensions.0/2) as f32, (dimensions.1/2) as f32);
    assert!(radius > 0. && radius < 1.);
    for i in 0..pin_count
    {
        let angle = std::f32::consts::PI * 2. * (i as f32) / pin_count as f32;
        pins[i].0 = center.0 + angle.cos() * center.0 * radius;
        pins[i].1 = center.1 + angle.sin() * center.1 * radius;
    }
    return pins;
}