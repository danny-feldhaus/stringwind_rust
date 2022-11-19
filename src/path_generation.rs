use crate::image_module::color_conversion::{RgbConversion, LabDifference};
use crate::image_module::lines::draw_line_lab;

use super::image_module::image_io::{read_lab,save_lab};
use super::image_module::color_conversion::{ColorSpace, PaletteToImage};
use super::string_setting::*;

use image:: {Rgb32FImage, ImageResult};
use itertools::Itertools;
use palette::{Lab, Mix};
use std::collections::HashMap;
use line_drawing::XiaolinWu;
pub struct PathStep
{
    pub from_idx : usize,
    pub to_idx : usize,
    pub color_idx : usize
}

pub struct StringPath
{
    pub path : Vec<PathStep>, //Each element of vector is (fron_index, to_index, color_index)
    pub pin_positions : Vec<(f32, f32)>, //Pin positions in unit space
    input_image : Rgb32FImage, //Input image in Lab color space
    output_path : String,
    colors : Vec<Lab>,
    _background : Lab,
    path_length : usize,
    //Internally generated
    combo_scores : Vec<HashMap<(usize,usize), f32>>,
    strings_drawn : Rgb32FImage,
    cur_step : usize,
    cur_idxs : Vec<usize>
}

impl Default for StringPath
{
    fn default() -> Self {
        return StringPath{
            path: Vec::default(), 
            pin_positions: Vec::default(), 
            input_image: Rgb32FImage::default(),
            output_path: String::default(),
            colors: Vec::default(),
            _background: Lab::default(),
            path_length: usize::default(),
            //Internally generated
            combo_scores: Vec::<HashMap<(usize,usize), f32>>::default(), 
            strings_drawn: Rgb32FImage::default(),
            cur_step: 0,
            cur_idxs : Vec::default()
        }
    }
}
impl StringPath
{
    pub fn new(settings: StringSettings) -> Result<StringPath, String>
    {
        let background = *settings.get::<Lab>("bg_color")?;
        let colors = settings.get::<Vec<Lab>>("str_colors")?.clone();

        let pin_count = *settings.get::<usize>("pin_count")?;
        let path_length = *settings.get::<usize>("line_count")?;
        let binder = read_lab(settings.get::<String>("in_image_path")?);
        if binder.is_err()  {return Err(binder.is_err().to_string())}
        let input_image = binder.unwrap(); 
        let output_path = settings.get::<String>("out_image_path")?.clone();
        let dimensions = input_image.dimensions();
        //Make pins
        let pin_positions = pin_circle(pin_count, *settings.get::<f32>("pin_radius")?, input_image.dimensions());
        let strings_drawn = Rgb32FImage::from_pixel(dimensions.0, dimensions.1,  background.to_rgb(ColorSpace::Lab));
        //Make combo scores iterator

        let cur_idxs = vec![0;colors.len()];
        let mut sp = StringPath
        {
            path: Vec::new(),
            pin_positions,
            input_image,
            output_path,
            colors,
            _background : background,
            path_length,
            combo_scores : Vec::<HashMap<(usize,usize), f32>>::default(),
            strings_drawn,
            cur_step: 0,
            cur_idxs
        };
        sp.calculate_initial_scores();

        Ok(sp)
    }

    fn step(&mut self) -> bool
    {
        self.cur_step+= 1;
        if self.cur_step == self.path_length {return false};
        let next_step = self.best_step();
        self.cur_idxs[next_step.color_idx] = next_step.to_idx;
        let from_coord = self.pin_positions[next_step.from_idx];
        let to_coord = self.pin_positions[next_step.to_idx];
        draw_line_lab(from_coord,to_coord, &mut self.strings_drawn, &self.colors[next_step.color_idx]);
        self.path.push(next_step);

        return true;
    }

    pub fn best_step(&self) -> PathStep
    {
        let mut best = PathStep
        {
            from_idx : 0,
            to_idx : 0,
            color_idx : 0
        };
        let mut best_score = f32::MIN;

        for (color_idx, from_idx) in self.cur_idxs.iter().enumerate()
        {
            let combos: &HashMap<_,_> = &self.combo_scores[color_idx];
            combos.iter()
                .filter(|((a,b), _score)| a == from_idx || b == from_idx)
                .for_each(|((pin_a_idx, pin_b_idx), line_score)|{
                    let score = *line_score -  self.current_similarity(*pin_a_idx,*pin_b_idx);
                    if score > best_score
                    {
                        best_score = score;
                        best.from_idx = *from_idx;
                        best.to_idx = if *pin_a_idx == *from_idx {*pin_b_idx} else {*pin_a_idx};
                        best.color_idx = color_idx;
                    }
                });
        }   
        best
    }

    pub fn save_visual(&self) -> ImageResult<()>
    {
        save_lab(&(self.output_path.clone() + "visual.png"), &self.strings_drawn)
    } 

    //Calculate the potential score for each pin combinatio and color
    fn calculate_initial_scores(&mut self)
    {
        let pin_combos = (0..self.pin_positions.len()).into_iter().tuple_combinations::<(usize,usize)>();
        let pin_combo_scores: HashMap<(usize, usize), f32> = HashMap::from_iter(pin_combos.zip(std::iter::repeat(0.)));
        let mut color_pin_combo_scores = vec![pin_combo_scores; self.colors.len()];

        for (color_idx, combos) in color_pin_combo_scores.iter_mut().enumerate()
        {   
            for ((pin_a_idx, pin_b_idx), score) in combos.iter_mut()
            {
                *score = self.string_similarity(*pin_a_idx, *pin_b_idx, color_idx);
            }
        }   
        self.combo_scores = color_pin_combo_scores;
    }

    fn string_similarity(&self, pin_a_idx : usize, pin_b_idx : usize, color_idx: usize) -> f32
    {
        let pin_a = self.pin_positions[pin_a_idx];
        let pin_b = self.pin_positions[pin_b_idx];
        let str_color = self.colors[color_idx];
        let line = XiaolinWu::<f32, i32>::new(pin_a, pin_b);
        let mut similarity_sum = 0_f32;
        let mut weight_sum = 0_f32;
        for ((x,y), weight) in line
        {
            let center_input = self.input_image.get_pixel(x as u32,y as u32).as_lab(&ColorSpace::Lab);
            let lr_mix_input = StringPath::left_right_mix(&pin_a, &pin_b, &(x,y), &self.input_image);
            let mix_input = center_input.mix(&lr_mix_input, 0.2);

            let lr_mix_strings = StringPath::left_right_mix(&pin_a, &pin_b, &(x,y), &self.strings_drawn);
            let mix_strings = str_color.mix(&lr_mix_strings, 0.2);
            similarity_sum += mix_input.similarity_to(&mix_strings);
            weight_sum += weight;
        }
        similarity_sum / weight_sum
    }

    fn current_similarity(&self, pin_a_idx : usize, pin_b_idx : usize) -> f32
    {
        let pin_a = self.pin_positions[pin_a_idx];
        let pin_b = self.pin_positions[pin_b_idx];
        let line = XiaolinWu::<f32, i32>::new(pin_a, pin_b);
        let mut similarity_sum = 0_f32;
        let mut weight_sum = 0_f32;
        for ((x,y), weight) in line
        {
            let center_input = self.input_image.get_pixel(x as u32,y as u32).as_lab(&ColorSpace::Lab);
            let lr_mix_input = StringPath::left_right_mix(&pin_a, &pin_b, &(x,y), &self.input_image);
            let mix_input = center_input.mix(&lr_mix_input, 0.2);

            let center_strings = self.strings_drawn.get_pixel(x as u32,y as u32).as_lab(&ColorSpace::Lab);
            let lr_mix_strings = StringPath::left_right_mix(&pin_a, &pin_b, &(x,y), &self.strings_drawn);
            let mix_strings = center_strings.mix(&lr_mix_strings, 0.2);
            similarity_sum += mix_input.similarity_to(&mix_strings);
            weight_sum += weight;
        }
        similarity_sum / weight_sum
    }

    fn left_right_mix(start : &(f32,f32), end : &(f32, f32), center: &(i32, i32), image : &Rgb32FImage) -> Lab
    {
        fn left_right_colors(start : &(f32,f32), end : &(f32, f32), center: &(i32, i32), image : &Rgb32FImage) -> (Lab, Lab)
        {
            let diff = (end.0 - start.0, end.1 - start.1);
            let len = (diff.0*diff.0 + diff.1*diff.1).sqrt();
            let offset = ((diff.0 / len).round() as i32,  (diff.1 / len).round() as i32);
            let l_coord = ((center.0 + offset.1) as u32, (center.1 - offset.0) as u32);
            let r_coord  = ((center.0 - offset.1) as u32, (center.1 + offset.0) as u32);
            return (image.get_pixel(l_coord.0, l_coord.1).as_lab(&ColorSpace::Lab),
                    image.get_pixel(r_coord.0, r_coord.1).as_lab(&ColorSpace::Lab));
        }
        let (color_left,color_right) = left_right_colors(start, end, center, image);
        color_left.mix(&color_right, 0.5)
    }

}

pub fn generate_path(settings_path: &'static str) -> Result<StringPath, String>
{
    let settings = read_string_settings(settings_path).map_err(|e| e.to_string())?;
    let mut sp = StringPath::new(settings)?;
    while sp.step() 
    {
        println!("{:?}:\t{:?}",sp.cur_step, sp.cur_idxs);
    }
    return Ok(sp);
}

fn pin_circle(pin_count: usize, radius: f32, dimensions: (u32, u32)) -> Vec<(f32,f32)>
{
    let mut pins = vec![(0.,0.);pin_count];
    assert!(radius > 0. && radius < 1.);
    let center = ((dimensions.0/2) as f32, (dimensions.1/2) as f32);
    for i in 0..pin_count
    {
        let angle = std::f32::consts::PI * 2. * (i as f32) / pin_count as f32;
        pins[i].0 = center.0 + angle.cos() * center.0 * radius;
        pins[i].1 = center.1 + angle.sin() * center.1 * radius;
    }
    return pins;
}