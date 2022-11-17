use image::{Rgb,ImageBuffer};
use palette::{IntoColor, Srgb, Lab};
use rayon::{prelude::ParallelSliceMut, iter::ParallelIterator};

pub enum ColorSpace
{
    Srgb, Lab
}

pub trait ImageConversion
{
    fn into_lab(&mut self) -> &ImageBuffer<Rgb<f32>, Vec<f32>>;
    fn into_rgb(&mut self) -> &ImageBuffer<Rgb<f32>, Vec<f32>>;
    fn as_lab(&self) -> ImageBuffer<Rgb<f32>, Vec<f32>>;
    fn as_rgb(&self) -> ImageBuffer<Rgb<f32>, Vec<f32>>;
}

impl ImageConversion for ImageBuffer<Rgb<f32>, Vec<f32>>
{
    fn into_lab(&mut self) -> &ImageBuffer<Rgb<f32>, Vec<f32>>
    {
        self.par_chunks_mut(3).for_each(|p|
            {
                let lab: Lab = Srgb::new(p[0], p[1], p[2]).into_color();
                p[0] = lab.l;
                p[1] = lab.a;
                p[2] = lab.b;
            });
        return self;
    }
    fn into_rgb(&mut self) -> &ImageBuffer<Rgb<f32>, Vec<f32>>
    {
        self.par_chunks_mut(3).for_each(|p|
            {
                let srgb: Srgb = Lab::new(p[0], p[1], p[2]).into_color();
                p[0] = srgb.red;
                p[1] = srgb.green;
                p[2] = srgb.blue;
            });
        return self;
    }
    fn as_lab(&self) -> ImageBuffer<Rgb<f32>, Vec<f32>>
    {
        let mut img = self.clone();
        img.par_chunks_mut(3).for_each(|p|
            {
                let lab: Lab = Srgb::new(p[0], p[1], p[2]).into_color();
                p[0] = lab.l;
                p[1] = lab.a;
                p[2] = lab.b;
            });
        return img;
    }
    fn as_rgb(&self) -> ImageBuffer<Rgb<f32>, Vec<f32>>
    {
        let mut img = self.clone();
        img.par_chunks_mut(3).for_each(|p|
            {
                let srgb: Srgb = Lab::new(p[0], p[1], p[2]).into_color();
                p[0] = srgb.red;
                p[1] = srgb.green;
                p[2] = srgb.blue;
            });
        return img;
    }
}

pub trait RgbConversion
{
    fn as_lab(&self, in_color_space: &ColorSpace ) -> Lab;
    fn as_srgb(&self, in_color_space: &ColorSpace ) -> Srgb;
}

impl RgbConversion for Rgb<f32>
{
    fn as_lab(&self, in_color_space: &ColorSpace) -> Lab
    {
        match in_color_space
        {
            ColorSpace::Srgb => 
            {
                return Srgb::new(self[0], self[1], self[2]).into_color();
            }
            ColorSpace::Lab =>
            {
                return Lab::new(self[0], self[1], self[2]);
            }
        }
    }
    fn as_srgb(&self, in_color_space: &ColorSpace) -> Srgb
    {
        match in_color_space
        {
            ColorSpace::Srgb => 
            {
                return Srgb::new(self[0], self[1], self[2]);
            }
            ColorSpace::Lab =>
            {
                return Lab::new(self[0], self[1], self[2]).into_color();
            }
        }
    }
}


pub trait PaletteToImage
{
    fn to_rgb(&self, out_color_space: ColorSpace) -> Rgb<f32>;
}

impl PaletteToImage for Lab
{
    fn to_rgb(&self, out_color_space: ColorSpace) -> Rgb<f32>
    {
        match out_color_space
        {
            ColorSpace::Srgb => 
            {
                let srgb: Srgb = self.clone().into_color();
                return Rgb([srgb.red, srgb.green, srgb.blue]);
            }
            ColorSpace::Lab =>
            {
                return Rgb([self.l, self.a, self.b]);
            }
        }
    }
}

impl PaletteToImage for Srgb
{
    fn to_rgb(&self, out_color_space: ColorSpace) -> Rgb<f32>
    {
        match out_color_space
        {
            ColorSpace::Srgb => 
            {
                return Rgb([self.red, self.green, self.blue]);
            }
            ColorSpace::Lab =>
            {
                let lab: Lab = self.clone().into_color();
                return Rgb([lab.l, lab.a, lab.b]);
            }
        }
    }
}
