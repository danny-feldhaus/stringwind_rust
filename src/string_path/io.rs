use super::StringPath;
use crate::image_module::lab::{get_color_name, LabBuf};
use image::{DynamicImage, EncodableLayout, ImageBuffer, ImageError, Pixel, RgbImage};
use std::path::Path;

impl StringPath {
    fn get_file_prefix(&self) -> String {
        Path::new(&self.input_image_path)
            .file_prefix()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    fn get_color_names(&self) -> String {
        let mut names = self
            .colors
            .iter()
            .fold(String::new(), |s, color| s + &get_color_name(color) + ",");
        names.pop(); //remove that trailing comma
        names
    }

    fn merge_images(images: &Vec<DynamicImage>) -> RgbImage {
        assert!(!images.is_empty());
        let width = images[0].width();
        let mut height = 0;
        let mut buf = Vec::<u8>::new();
        for image in images {
            buf.extend_from_slice(image.to_rgb8().as_bytes());
            height += image.height();
            //Make sure that all images have the same height
            assert!(width == image.width());
        }
        RgbImage::from_vec(width, height, buf).unwrap()
    }

    fn visualize_coverage(&self) -> DynamicImage {
        let visual = ImageBuffer::from_fn(
            self.coverage_map.width(),
            self.coverage_map.height(),
            |x, y| {
                let p = self.coverage_map.get_pixel(x, y)[0];
                *image::Rgb::<u8>::from_slice(&[
                    if p >= 2 { 255 } else { 0 },
                    if p > 2 { 255 } else { 0 },
                    if p > 2 { 255 } else { 0 },
                ])
            },
        );
        DynamicImage::ImageRgb8(visual)
    }

    pub fn save_progress(&self, verbose: bool) -> Result<(), ImageError> {
        let mut images = vec![DynamicImage::ImageRgba32F(
            self.strings_drawn.as_rgb_image_buffer(),
        )];
        if verbose {
            images.push(DynamicImage::ImageRgba32F(
                self.input_image.as_rgb_image_buffer(),
            ));
            images.push(self.visualize_coverage());
        };
        let merged_image = Self::merge_images(&images);

        let prefix = self.get_file_prefix();
        let names = self.get_color_names();
        let directory = format!(
            "{output_path}{prefix}_wg:{edge_weight}_c:{names}",
            output_path = self.output_path,
            edge_weight = self.edge_weight
        );
        let save_path = format!("{directory}/{cur_step}.png", cur_step = self.cur_step);
        std::fs::create_dir(directory.clone()).unwrap_or_default();
        merged_image.save(&save_path)
    }
}
