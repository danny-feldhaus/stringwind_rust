use crate::image_module::color_conversion::{RgbConversion, LabDifference, ImageConversion};
use crate::image_module::lines::draw_line_lab;

use super::image_module::image_io::{read_lab,save_lab,get_color_name};
use super::image_module::color_conversion::{ColorSpace, PaletteToImage};
use super::string_setting::*;
use super::tri_vec::*;

use image:: {Rgb32FImage, ImageResult, EncodableLayout, DynamicImage};
use itertools::Itertools;
use palette::{Lab, Mix};
use std::collections::HashMap;
use std::path::Path;
use line_drawing::XiaolinWu;
use show_image::{ImageView, ImageInfo, create_window};
use rand::distributions::{WeightedIndex,Distribution};

#[derive(Clone, Copy)]
pub struct PathStep
{
    pub from_idx : usize,
    pub to_idx : usize,
    pub color_idx : usize,
    pub score : f32
}

pub struct StringPath
{
    pub path : Vec<PathStep>, //Each element of vector is (fron_index, to_index, color_index)
    pub pin_positions : Vec<(f32, f32)>, //Pin positions in unit space
    input_image_path : String,
    input_image : Rgb32FImage, //Input image in Lab color space
    output_path : String,
    colors : Vec<Lab>,
    _background : Lab,
    path_length : usize,
    //Internally generated
    combo_scores : Vec<HashMap<(usize,usize), (f32,f32)>>,
    strings_drawn : Rgb32FImage,
    cur_step : usize,
    cur_idxs : Vec<usize>,
    cur_scores : Vec<f32>,
    edge_weight : f32
}

impl Default for StringPath
{
    fn default() -> Self {
        return StringPath{
            path: Vec::default(), 
            pin_positions: Vec::default(), 
            input_image_path : Default::default(),
            input_image: Rgb32FImage::default(),
            output_path: String::default(),
            colors: Vec::default(),
            _background: Lab::default(),
            path_length: usize::default(),
            //Internally generated
            combo_scores: Vec::<HashMap<(usize,usize), (f32,f32)>>::default(), 
            strings_drawn: Rgb32FImage::default(),
            cur_step: 0,
            cur_idxs : Vec::default(),
            cur_scores : Vec::default(),
            edge_weight : 0.
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
        //let width = *settings.get::<usize>("width")? as u32;
        //let height = *settings.get::<usize>("height")? as u32;

        let input_image_path = settings.get::<String>("in_image_path")?.clone();
        let binder = read_lab(&input_image_path);
        if binder.is_err()  {return Err(binder.is_err().to_string())}
        //let input_image = image::imageops::resize::<Rgb32FImage>(
        //    &binder.unwrap(), 
        //    width,
        //    height,
        //    image::imageops::FilterType::Nearest); 
        let input_image = binder.unwrap();

        let output_path = settings.get::<String>("out_image_path")?.clone();
        let dimensions = input_image.dimensions();
        //Make pins
        let pin_positions = pin_circle(pin_count, *settings.get::<f32>("pin_radius")?, input_image.dimensions());
        let strings_drawn = Rgb32FImage::from_pixel(dimensions.0, dimensions.1,  background.to_rgb(ColorSpace::Lab));
        //Make combo scores iterator

        let cur_idxs = vec![0;colors.len()];
        let cur_scores = vec![0.;colors.len()];
        let edge_weight = *settings.get::<f32>("edge_weight")?;
        let mut sp = StringPath
        {
            path: Vec::new(),
            pin_positions,
            input_image_path,
            input_image,
            output_path,
            colors,
            _background : background,
            path_length,
            combo_scores : Vec::<HashMap<(usize,usize), (f32,f32)>>::default(),
            strings_drawn,
            cur_step: 0,
            cur_idxs,
            cur_scores,
            edge_weight
        };
        sp.calculate_initial_scores();

        Ok(sp)
    }

    //Save a visual representation of the current path
    pub fn save_visual(&self) -> ImageResult<()>
    {
        let prefix = Path::new(&self.input_image_path).file_prefix().unwrap().to_str().unwrap();
        let color_names : Vec<String> =  self.colors.iter()
            .map(|c| get_color_name(c)).collect();
        let name_string = color_names.iter().fold("".to_string(),|a,b| format!("{a},{b}"));
        let path = format!("{output_path}{prefix}_edgeweight:{edge_weight}_lines:{cur_step}{name_string}.png", output_path = self.output_path, edge_weight = self.edge_weight, cur_step = self.cur_step);
        save_lab(&path, &self.strings_drawn)
    } 

    //Add a step to the path
    fn step(&mut self) -> bool
    {
        self.cur_step+= 1;
        if self.cur_step == self.path_length {return false};

        let next_steps = self.get_best_steps();
        let dist = WeightedIndex::new(next_steps.iter().map(|p| p.score.clamp(0.,1.))).unwrap();
        let mut rng = rand::thread_rng();
        let step = next_steps[dist.sample(&mut rng)];
            
        self.cur_idxs[step.color_idx] = step.to_idx;
        let from_coord = self.pin_positions[step.from_idx];
        let to_coord = self.pin_positions[step.to_idx];
        draw_line_lab(from_coord,to_coord, &mut self.strings_drawn, &self.colors[step.color_idx]);
        self.path.push(step);
        /*
        let next_step = self.get_best_step();
        self.cur_idxs[next_step.color_idx] = next_step.to_idx;
        let from_coord = self.pin_positions[next_step.from_idx];
        let to_coord = self.pin_positions[next_step.to_idx];
        draw_line_lab(from_coord,to_coord, &mut self.strings_drawn, &self.colors[next_step.color_idx]);
        self.path.push(next_step);
        */
        true
    }

    //Calculate tehe color / pin combination with the best score
    pub fn get_best_steps(&mut self) -> Vec<PathStep>
    {
        let mut best_steps = Vec::<PathStep>::new();
        for color_idx in 0..self.colors.len()
        {
            best_steps.push(self.get_best_step(color_idx));
            self.cur_scores[color_idx] = best_steps.last().unwrap().score;
        }   
        best_steps.sort_by(|a,b| b.score.partial_cmp(&a.score).unwrap());
        best_steps
    }

    //Calculate the best pin to move to for the given color
    fn get_best_step(&self, color_idx : usize) -> PathStep
    {
        let from_idx = self.cur_idxs[color_idx];
        let mut best_step = PathStep {from_idx, color_idx, to_idx : 0, score : -1.};
        for to_idx in 0..self.pin_positions.len()
        {
            let pin_combo = (std::cmp::min(from_idx, to_idx), std::cmp::max(from_idx,to_idx));
            let current_score_opt = self.calculate_current_score(color_idx, &pin_combo);
            if current_score_opt.is_some() && current_score_opt.unwrap() > best_step.score
            {
                best_step.score  = current_score_opt.unwrap();
                best_step.to_idx = if pin_combo.0 == from_idx {pin_combo.1} else {pin_combo.0};
                best_step.color_idx = color_idx;
            }            
        }
        best_step
    }
    
    //Calculate the initial score of every possible line
    fn calculate_initial_scores(&mut self)
    {
        let pin_combos = (0..self.pin_positions.len()).into_iter().tuple_combinations::<(usize,usize)>();
        let pin_combo_scores: HashMap<(usize, usize), (f32,f32)> = HashMap::from_iter(pin_combos.zip(std::iter::repeat((0.,0.))));
        let mut color_pin_combo_scores = vec![pin_combo_scores; self.colors.len()];

        for (color_idx, combos) in color_pin_combo_scores.iter_mut().enumerate()
        {   
            for ((pin_a_idx, pin_b_idx), score) in combos.iter_mut()
            {
                *score = (0.,0.);//self.calculate_intiial_score(*pin_a_idx, *pin_b_idx, color_idx);
            }
        }   
        self.combo_scores = color_pin_combo_scores;
    }
    
    //Calculate the current score of the given line (its similarity to the image vs the similarity without it)
    fn calculate_current_score(&self, color_idx: usize, pin_combo: &(usize, usize)) -> Option<f32>
    {

        let initial_score_opt = self.combo_scores[color_idx].get(&pin_combo);
        if initial_score_opt.is_none() {return None};
       // let initial_score = initial_score_opt.unwrap();

        let pin_a = self.pin_positions[pin_combo.0];
        let pin_b = self.pin_positions[pin_combo.1];
        let line = XiaolinWu::<f32, i32>::new(pin_a, pin_b);
        let mut score_sum = 0_f32;
        let mut weight_sum = 0_f32;

        let center_drawn = self.colors[color_idx];

        for ((x,y), weight) in line
        {
            //Calculate the mixed color of input_image at the current point
            let center_input = self.input_image.get_pixel(x as u32,y as u32).as_lab(&ColorSpace::Lab);
            let lr_mix_input = StringPath::left_right_mix(&pin_a, &pin_b, &(x,y), &self.input_image);
            let mix_input = center_input.mix(&lr_mix_input, self.edge_weight);
            //Calculate the mixed color of strings_drawn, without the line drawn over it, at the current point
            let center_undrawn =  self.strings_drawn.get_pixel(x as u32,y as u32).as_lab(&ColorSpace::Lab);
            let lr_mix_undrawn = StringPath::left_right_mix(&pin_a, &pin_b, &(x,y), &self.strings_drawn);
            let mix_undrawn = center_undrawn.mix(&lr_mix_undrawn, self.edge_weight);
            //Calculate the mixed color of strings_drawn, with the line drawn over it, at the current point
            let mix_drawn = center_drawn.mix(&lr_mix_undrawn, self.edge_weight);
            //Score using the unmixed colors at (x,y). Meant to represent the score with the assumption that the current color will be dense in this area.
            let score_unmixed = center_drawn.similarity_to(&center_input) - center_undrawn.similarity_to(&center_input);
            //Score using the mixed colors at (x,y). Meant to represent the score with the assumption that that the current color will be sparse in this area.
            let score_mixed = mix_drawn.similarity_to(&mix_input) - mix_undrawn.similarity_to(&mix_input);
            //Choose the best of the two scores 
            let score = score_unmixed.max(score_mixed);
            score_sum += score;
            weight_sum += weight;
        }
        let score = score_sum / weight_sum;
        Some(score)
    }

    /*
    //Calculate the initial score of the given line (its similarity to the input image)
    fn calculate_intiial_score(&self, pin_a_idx : usize, pin_b_idx : usize, color_idx: usize) -> (f32, f32)
    {
        let pin_a = self.pin_positions[pin_a_idx];
        let pin_b = self.pin_positions[pin_b_idx];
        let str_color = self.colors[color_idx];
        let line = XiaolinWu::<f32, i32>::new(pin_a, pin_b);
        let mut sum_mixed = 0_f32;
        let mut sum_unmixed = 0_f32;
        let mut weight_sum = 0_f32;
        for ((x,y), weight) in line
        {
            let center_input = self.input_image.get_pixel(x as u32,y as u32).as_lab(&ColorSpace::Lab);
            let lr_mix_input = StringPath::left_right_mix(&pin_a, &pin_b, &(x,y), &self.input_image);
            let mix_input = center_input.mix(&lr_mix_input, self.edge_weight);

            let center_undrawn =  self.strings_drawn.get_pixel(x as u32,y as u32).as_lab(&ColorSpace::Lab);
            let lr_mix_undrawn = StringPath::left_right_mix(&pin_a, &pin_b, &(x,y), &self.strings_drawn);
            let mix_undrawn = center_undrawn.mix(&lr_mix_undrawn, self.edge_weight);

            let mix_drawn = str_color.mix(&lr_mix_undrawn, self.edge_weight);

            let score_unmixed = str_color.similarity_to(&center_input) - center_undrawn.similarity_to(&center_input);
            let score_mixed = mix_drawn.similarity_to(&mix_input) - mix_undrawn.similarity_to(&mix_input);
            let score = score_unmixed.max(score_mixed);
            sum_unmixed += score;
            weight_sum += weight;
        }
        (sum_unmixed / weight_sum, sum_mixed / weight_sum)
    }
    */

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
    let window = create_window("Image", Default::default()).unwrap();
    while sp.step() 
    {
        if sp.cur_step % 100 == 0
        {
            let binding =  DynamicImage::ImageRgb32F(sp.strings_drawn.as_rgb()).into_rgb8();
            let window_image  = ImageView::new(ImageInfo::rgb8(sp.input_image.width(), sp.input_image.height()), binding.as_bytes());
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