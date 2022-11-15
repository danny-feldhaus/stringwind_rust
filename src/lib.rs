#![feature(test)]
extern crate test;

use std::f32::consts::PI;
use image::{ImageBuffer,Rgb};
use test::Bencher;
mod image_module;


use image_module::img_analysis::*;
use image_module::lines::*;
#[bench]
fn b_avg_img_to_col(b: &mut Bencher)
{
    let image = image::open("src/tests/images/vangogh.png").unwrap().into_rgb32f();
    b.iter(|| avg_img_to_col(&image, Rgb([0.,0.,0.])));
}

#[bench]
fn b_avg_img_to_img(b: &mut Bencher)
{
    let image = image::open("src/tests/images/vangogh.png").unwrap().into_rgb32f();
    let comparison_img = ImageBuffer::from_fn(image.width(), image.height(),
        |x,y|{
            let g = (x as f32 / image.width() as f32) * (y as f32 / image.height() as f32);
            Rgb([g,g,g])
        }
    );
    b.iter(|| avg_img_to_img(&image, &comparison_img));
}


#[bench]
fn b_img_to_col(b: &mut Bencher)
{
    let image = image::open("src/tests/images/vangogh.png").unwrap().into_rgb32f();
    b.iter(|| diff_img_to_col(&image, Rgb([0.,0.,0.])));
}

#[bench]
fn b_img_to_img(b: &mut Bencher)
{
    let image = image::open("src/tests/images/vangogh.png").unwrap().into_rgb32f();
    let comparison_img = ImageBuffer::from_fn(image.width(), image.height(),
    |x,y|{
        let g = (x as f32 / image.width() as f32) * (y as f32 / image.height() as f32);
        Rgb([g,g,g])
    });
    b.iter(||  diff_img_to_img(&image, &comparison_img));
}

#[bench]
fn b_line_comparison_lab(b: &mut Bencher)
{
    let pin_count = 20;
    let image = image::open("src/tests/images/vangogh.png").unwrap().into_rgb32f();
    let lab_image = rgb_as_lab(&image);
    let mut angle: f32;
    let center = ((image.width()/2) as f32, (image.height()/2) as f32);
    let mut pins = vec![(0.,0.);pin_count];
    for i in 0..pin_count
    {
        angle = PI * 2. * (i as f32) / pin_count as f32;
        pins[i].0 = center.0 + angle.cos() * center.0 * 0.95;
        pins[i].1 = center.1 + angle.sin() * center.1 * 0.95;
    }
    b.iter(|| {
            for i in 1..pin_count
            {
                line_diff_img_to_col(pins[0], pins[i], &lab_image, image::Rgb([0.,0.,0.]), true);
            }
        }
    );
}

#[bench]
fn b_line_comparison_rgb(b: &mut Bencher)
{
    let pin_count = 20;
    let image = image::open("src/tests/images/vangogh.png").unwrap().into_rgb32f();
    let mut angle: f32;
    let center = ((image.width()/2) as f32, (image.height()/2) as f32);
    let mut pins = vec![(0.,0.);pin_count];
    for i in 0..pin_count
    {
        angle = PI * 2. * (i as f32) / pin_count as f32;
        pins[i].0 = center.0 + angle.cos() * center.0 * 0.95;
        pins[i].1 = center.1 + angle.sin() * center.1 * 0.95;
    }
    b.iter(|| {
        for i in 1..pin_count
        {
            line_diff_img_to_col(pins[0], pins[i], &image, image::Rgb([0.,0.,0.]), false);
        }
    });
}

#[bench]
fn b_line_comparison_diff(b: &mut Bencher)
{
    let pin_count = 20;
    let image = image::open("src/tests/images/vangogh.png").unwrap().into_rgb32f();
    let diff_image = diff_img_to_col(&image, image::Rgb([0.,0.,0.]));
    let mut angle: f32;
    let center = ((image.width()/2) as f32, (image.height()/2) as f32);
    let mut pins = vec![(0.,0.);pin_count];
    for i in 0..pin_count
    {
        angle = PI * 2. * (i as f32) / pin_count as f32;
        pins[i].0 = center.0 + angle.cos() * center.0 * 0.95;
        pins[i].1 = center.1 + angle.sin() * center.1 * 0.95;
    }
    b.iter(|| {
        for i in 1..pin_count
        {
            average_line(pins[0], pins[i], &diff_image);
        }
    }
    );
}
