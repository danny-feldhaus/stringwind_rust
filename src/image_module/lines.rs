use image::Rgb32FImage;
use line_drawing::XiaolinWu;
use lerp::Lerp;
use palette::Lab;

extern crate geo;


#[allow(dead_code)]
pub struct StepLR
{
    pub center : ((i32, i32), f32),
    pub left : ((i32, i32), f32),
    pub right : ((i32, i32), f32)
}

#[allow(dead_code)]
pub fn lr_steps(start : (f32, f32), end : (f32, f32), edge_weight : f32) -> Vec<StepLR>
{
    let diff = (end.0-start.0, end.1 - start.1);
    let length = (diff.0*diff.0 + diff.1*diff.1).sqrt();
    let norm = ((diff.0 / length).round() as i32, (diff.1/length).round() as i32);
    let left_offset = (norm.1, -norm.0);
    let right_ofset = (-norm.1, norm.0);
    let line = XiaolinWu::<f32, i32>::new(start, end);
    let mut steps = Vec::<StepLR>::new();
    for (coord, weight) in line.into_iter()
    {
        let left = (coord.0 + left_offset.0, coord.1 + left_offset.1);
        let right = (coord.0 + right_ofset.0, coord.0 + right_ofset.1);
        steps.push(StepLR {
            center: (coord, weight),
            left: (left, weight * edge_weight),
            right: (right, weight * edge_weight)}
        );
    }
    steps
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