use image::{ImageBuffer, Rgb, Rgba, ImageResult, DynamicImage};
use palette::{Lab, Srgb, Laba, Srgba, IntoColor, Mix};
use rayon::prelude::*;
use csv::Reader;
use line_drawing::XiaolinWu;

#[derive(Default)]
pub struct LabImageBuffer
{
    buffer: ImageBuffer<Rgb<f32>, Vec<f32>>,
}
#[allow(dead_code)]
impl LabImageBuffer
{
    pub fn width(&self) -> u32 {self.buffer.width()}
    pub fn height(&self) -> u32 {self.buffer.height()}
    pub fn dimensions(&self) -> (u32, u32) {self.buffer.dimensions()}
    pub fn draw_line(&mut self, start: (f32, f32), end: (f32, f32), color: &Lab)
    {
        let line = XiaolinWu::<f32, i32>::new(start, end);
        line.for_each(|((x,y), weight)| 
        {                        
            let background = self.get_pixel(x as u32,y as u32);
            let pix_color = background.mix(color, weight);
            self.put_pixel(x as u32, y as u32, &pix_color);
        });
    }
}

#[derive(Default)]
pub struct LabaImageBuffer
{
    buffer: ImageBuffer<Rgba<f32>, Vec<f32>>
}

#[allow(dead_code)]
impl LabaImageBuffer
{
    pub fn width(&self) -> u32 {self.buffer.width()}
    pub fn height(&self) -> u32 {self.buffer.height()}
    pub fn dimensions(&self) -> (u32, u32) {self.buffer.dimensions()}
    pub fn draw_line(&mut self, start: (f32, f32), end: (f32, f32), color: &Laba, alpha_weight: bool)
    {
        let line = XiaolinWu::<f32, i32>::new(start, end);
        line.for_each(|((x,y), weight)| 
        {
            let pix_color = if alpha_weight 
                    {
                        Laba::new(color.l, color.a, color.b, weight)
                    }
                else
                    {
                        let background = self.get_pixel(x as u32,y as u32);
                        background.mix(color, weight)
                    };

            self.put_pixel(x as u32, y as u32, &pix_color);
        });
    }
}

pub trait LabBuf
where Self: Sized
{
    type LabType;
    type RgbType: image::Pixel;
    type BufferType;
    fn get_pixel(&self, x: u32, y: u32) -> Self::LabType;
    fn put_pixel(&mut self, x: u32, y: u32, value: &Self::LabType);
    fn new(width: u32, height: u32) -> Self;
    fn as_rgb_image_buffer(&self) -> Self::BufferType;
    fn from_rgb_image_buffer(buffer: &Self::BufferType) -> Self;
    fn from_file(path: &str) -> Result<Self, image::ImageError>;
    fn from_lab(width: u32, height: u32, color: &Self::LabType) -> Self;
    fn save(&self, path: &str) -> ImageResult<()>;
}

impl LabBuf for LabImageBuffer
{
    type LabType = Lab;
    type RgbType = Rgb<f32>;
    type BufferType = ImageBuffer<Self::RgbType, Vec<f32>>;
    fn get_pixel(&self, x: u32, y: u32) -> Self::LabType
    {
        let pix_rgb = self.buffer.get_pixel(x,y);
        return Lab::new(pix_rgb[0], pix_rgb[1], pix_rgb[2]);
    }
    fn put_pixel(&mut self, x: u32, y: u32, value: &Self::LabType)
    {
        self.buffer.put_pixel(x, y, Rgb::<f32>{0: [value.l, value.a, value.b]});
    }
    fn new(width: u32, height: u32) -> Self
    {
        Self
        {
            buffer: Self::BufferType::new(width, height)
        }
    }
    fn as_rgb_image_buffer(&self) -> Self::BufferType 
    {
        let mut rgb_buffer = self.buffer.clone();
        rgb_buffer.par_chunks_mut(3).for_each(|p|
            {
                let srgb: Srgb = Lab::new(p[0], p[1], p[2]).into_color();
                p[0] = srgb.red;
                p[1] = srgb.green;
                p[2] = srgb.blue;
            });
        return rgb_buffer;
    }
    fn from_rgb_image_buffer(buffer: &Self::BufferType) -> Self 
    {
        let mut lab_buff = buffer.clone();
        lab_buff.par_chunks_mut(3).for_each(|p|
            {
                let srgb: Lab = Srgb::new(p[0], p[1], p[2]).into_color();
                p[0] = srgb.l;
                p[1] = srgb.a;
                p[2] = srgb.b;
            }
        );
        Self
        {
            buffer: lab_buff
        }
    }
    fn from_file(path: &str) -> Result<Self, image::ImageError>
    {
        let binding = image::open(path)?;
        let rgb_img = binding.into_rgb32f().clone();
        Ok(Self::from_rgb_image_buffer(&rgb_img))
    }
    fn from_lab(width: u32, height: u32, color: &Lab) -> Self
    {
        let mut img = Self::new(width, height);
        for x in 0..width
        {
            for y in 0..height
            {
                img.put_pixel(x, y, color)
            }
        }
        img
    }

    fn save(&self, path: &str) -> ImageResult<()>
    {
        let rgb_img = self.as_rgb_image_buffer();
        let binding = DynamicImage::ImageRgb32F(rgb_img);
        let rgb_img = binding.into_rgb8();
        rgb_img.save(path)
    }

}

impl LabBuf for LabaImageBuffer
{
    type LabType = Laba;
    type RgbType = Rgba<f32>;
    type BufferType = ImageBuffer<Self::RgbType, Vec<f32>>;
    fn get_pixel(&self, x: u32, y: u32) -> Self::LabType
    {
        let pix_rgb = self.buffer.get_pixel(x,y);
        return Laba::new(pix_rgb[0], pix_rgb[1], pix_rgb[2], pix_rgb[3]);
    }
    fn put_pixel(&mut self, x: u32, y: u32, value: &Self::LabType)
    {
        self.buffer.put_pixel(x, y, Rgba::<f32>{0: [value.l, value.a, value.b, value.alpha]});
    }
    fn new(width: u32, height: u32) -> Self
    {
        Self
        {
            buffer: Self::BufferType::new(width, height)
        }
    }
    fn as_rgb_image_buffer(&self) -> Self::BufferType 
    {
        let mut rgb_buffer = self.buffer.clone();
        rgb_buffer.par_chunks_mut(4).for_each(|p|
            {
                let srgb: Srgba = Laba::new(p[0], p[1], p[2], p[3]).into_color();
                p[0] = srgb.red;
                p[1] = srgb.green;
                p[2] = srgb.blue;
                p[3] = srgb.alpha;
            });
        return rgb_buffer;
    }
    fn from_rgb_image_buffer(buffer: &Self::BufferType) -> Self 
    {
        let mut lab_buff = buffer.clone();
        lab_buff.par_chunks_mut(4).for_each(|p|
            {
                let srgb: Laba = Srgba::new(p[0], p[1], p[2], p[3]).into_color();
                p[0] = srgb.l;
                p[1] = srgb.a;
                p[2] = srgb.b;
                p[3] = srgb.alpha;
            });
        Self
        {
            buffer: lab_buff
        }
    }
    fn from_file(path: &str) -> Result<Self, image::ImageError>
    {
        let binding = image::open(path)?;
        let rgba_img = binding.into_rgba32f().clone();
        Ok(Self::from_rgb_image_buffer(&rgba_img))
    }
    fn from_lab(width: u32, height: u32, color: &Laba) -> Self
    {
        let mut img = Self::new(width, height);
        for x in 0..width
        {
            for y in 0..height
            {
                img.put_pixel(x, y, color)
            }
        }
        img
    }
    fn save(&self, path: &str) -> ImageResult<()>
    {
        let rgb_img = self.as_rgb_image_buffer();
        let binding = DynamicImage::ImageRgba32F(rgb_img);
        let rgb_img = binding.into_rgba8();
        rgb_img.save(path)
    }
}


pub trait LabDifference
{
    fn difference_from(&self, other: &Lab) -> f32;
    fn similarity_to(&self, other: &Lab) -> f32;
}

//Implements a color difference algorithm based on compile-time ettings.
impl LabDifference for Lab
{
    fn difference_from(&self, other: &Lab) -> f32 {
        const MAX_DIFF : f32 = 374.232548023; //maximum difference between two Lab colors
        //Use palette's built-in difference function. 
        //  Preliminary tests show that this algorithm is a lot slower, and potentially results in less detail. More testing needed.
        #[cfg(feature = "cielab_difference")] 
        {
            return self.get_color_difference(other) / max_diff;
        }
        //Use linear euclidean distance between Lab colors.
        //  Currently seems like the best option.
        #[cfg(not(feature = "cielab_difference"))]
        {
            let diff = *self - *other;
            return (diff.l*diff.l + diff.a*diff.a + diff.b*diff.b).sqrt() / MAX_DIFF;
        }
    }
    fn similarity_to(&self, other: &Lab) -> f32 {
        1.0 - self.difference_from(other)
    }
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