use crate::image_module::color_conversion::{RgbConversion, LabDifference, ImageConversion};
use crate::image_module::lines::draw_line_lab;

use super::image_module::image_io::{read_lab,save_lab,get_color_name};
use super::image_module::color_conversion::{ColorSpace, PaletteToImage};
use super::string_setting::*;
use super::tri_vec::*;

use image:: {Rgb32FImage, ImageResult, EncodableLayout, DynamicImage};
use palette::{Lab, Mix};
use std::path::Path;
use line_drawing::XiaolinWu;
use show_image::{ImageView, ImageInfo, create_window};
use rand::distributions::{WeightedIndex,Distribution};
use geo::{Line, coord, algorithm::line_intersection::line_intersection, LineIntersection, point};

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
    combo_scores : TriVec<Vec<Option<f32>>>,
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
            combo_scores: Default::default(),
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
            combo_scores : TriVec::new(pin_count, &vec![None; colors.len()]),
            colors,
            _background : background,
            path_length,
            strings_drawn,
            cur_step: 0,
            cur_idxs,
            cur_scores,
            edge_weight
        };
        sp.assign_viable_pairs();
        sp.find_start_positions();
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
        self.path.push(step);
        self.update();

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
    fn get_best_step(&mut self, color_idx : usize) -> PathStep
    {
        let from_idx = self.cur_idxs[color_idx];
        let mut best_step = PathStep {from_idx, color_idx, to_idx : 0, score : -1.};
        for to_idx in 0..self.pin_positions.len()
        {
            if to_idx != from_idx
            {
                let pin_combo = (std::cmp::min(from_idx, to_idx), std::cmp::max(from_idx,to_idx));
                let current_score_opt = self.calculate_whole_score(color_idx, &pin_combo);
                if current_score_opt.is_some() && current_score_opt.unwrap() > best_step.score
                {
                    best_step.score  = current_score_opt.unwrap();
                    best_step.to_idx = if pin_combo.0 == from_idx {pin_combo.1} else {pin_combo.0};
                    best_step.color_idx = color_idx;
                }            
            }
        }
        best_step
    }

    fn assign_viable_pairs(&mut self)
    {
        for x in 0..self.pin_positions.len()
        {
            for y in x+1..self.pin_positions.len()
            {
                for color_idx in 0..self.colors.len()
                {
                    self.combo_scores.at(x,y)[color_idx] = self.calculate_whole_score(color_idx, &(x,y));
                }   
            }
        }
    }

    fn find_start_positions(&mut self)
    {
        for color_idx in 0..self.colors.len()
        {
            let mut best_from = 0;
            let mut best_score = -1_f32;
            for x in 0..self.pin_positions.len()
            {
                for y in x+1..self.pin_positions.len()
                { 
                    let score_opt = &mut self.combo_scores.at(x,y)[color_idx];
                    if score_opt.unwrap_or(-1.) > best_score
                    {
                        best_from = x;
                        best_score = score_opt.unwrap();
                    }
                     
                }
            }
            self.cur_idxs[color_idx] = best_from;
        }
    }

    fn update(&mut self)
    {
        let last_step = self.path.last().unwrap();
        let latest_a = self.pin_positions[last_step.from_idx];
        let latest_b = self.pin_positions[last_step.to_idx];
        draw_line_lab(latest_a,latest_b, &mut self.strings_drawn, &self.colors[last_step.color_idx]);
        for color_idx in 0..self.colors.len()
        {
            for x in 0..self.pin_positions.len()
            {
                for y in x+1..self.pin_positions.len()
                {
                    self.null_intersected(&(x, y), color_idx);
                }
            }
        }
    }

    fn null_intersected(&mut self, pin_combo: &(usize, usize), color_idx: usize) -> Option<f32>
    {
        let last_step = self.path.last()?;
        let latest_a = self.pin_positions[last_step.from_idx];
        let latest_b = self.pin_positions[last_step.to_idx];
        let latest_line = Line::new(coord!{x: latest_a.0,y: latest_a.1}, coord!{x: latest_b.0,y: latest_b.1});

        let calc_a = self.pin_positions[pin_combo.0];
        let calc_b = self.pin_positions[pin_combo.1];
        let calc_line = Line::new(coord!{x: calc_a.0,y: calc_a.1}, coord!{x: calc_b.0,y: calc_b.1});
        let calc_old_score = self.combo_scores.at(pin_combo.0, pin_combo.1)[color_idx]?;

        let latest_and_calc = line_intersection(latest_line, calc_line)?;
        
        if let LineIntersection::SinglePoint { intersection: _, is_proper } = latest_and_calc {

            if !is_proper {return Some(calc_old_score)};
            self.combo_scores.at(pin_combo.0, pin_combo.1)[color_idx] = None;
            None
            /*
                let mut partial_score_change = 0_f32;
                let mut total_weight_sum = 0_f32;
                let pin_a = self.pin_positions[pin_combo.0];
                let pin_b = self.pin_positions[pin_combo.1];
                let line = XiaolinWu::<f32, i32>::new(pin_a, pin_b);
                let partial_distance = self.overlap_width(pin_combo, &(last_step.from_idx, last_step.to_idx)) as i32 + 2;
                for (point, weight) in line
                {
                    total_weight_sum += weight;
                    if (point.0 - intersection.x as i32).abs() < partial_distance  || (point.1 - intersection.y as i32).abs() < partial_distance 
                    {
                        partial_score_change -= self.score_at_point(&point, &pin_combo.0, &pin_combo.1, &color_idx, Some(old_strings_drawn))   * weight;
                        partial_score_change += self.score_at_point(&point, &pin_combo.0, &pin_combo.1, &color_idx, None) * weight;
                    }
                }
                let total_score_sum = total_weight_sum * calc_old_score;
                let new_score = (total_score_sum + partial_score_change) / total_weight_sum;
                Some(new_score)
                */
        } else {
            self.combo_scores.at(pin_combo.0, pin_combo.1)[color_idx] = Some(0.);
            Some(0.)
        }
    }
    
    //Calculate the current score of the given line (its similarity to the image vs the similarity without it)
    fn calculate_whole_score(&mut self, color_idx: usize, pin_combo: &(usize, usize)) -> Option<f32>
    {

        let score_opt = self.combo_scores.at(pin_combo.0, pin_combo.1)[color_idx];
        match score_opt 
        {
            None => 
            {
                let pin_a = self.pin_positions[pin_combo.0];
                let pin_b = self.pin_positions[pin_combo.1];
                let line = XiaolinWu::<f32, i32>::new(pin_a, pin_b);
                let mut score_sum = 0_f32;
                let mut weight_sum = 0_f32;
                for (point, weight) in line
                {
                    score_sum += self.score_at_point(&point, &pin_combo.0, &pin_combo.1, &color_idx) * weight;
                    weight_sum += weight;
                }
                let score = Some(score_sum / weight_sum);

                self.combo_scores.at(pin_combo.0, pin_combo.1)[color_idx] = score;
                score
            },
            Some(score) => Some(score)
        }
    }

    fn score_at_point(&mut self, point: &(i32, i32), pin_a_idx : &usize, pin_b_idx : &usize, color_idx: &usize) -> f32
    {
        let pin_a = self.pin_positions[*pin_a_idx];
        let pin_b = self.pin_positions[*pin_b_idx];
        let line_color = self.colors[*color_idx];

        //Calculate the mixed color of input_image at the current point
        let center_input = self.input_image.get_pixel(point.0 as u32,point.1 as u32).as_lab(&ColorSpace::Lab);
        let lr_mix_input = StringPath::left_right_mix(&pin_a, &pin_b, point, &self.input_image);
        let mix_input = center_input.mix(&lr_mix_input, self.edge_weight);
        //Calculate the mixed color of strings_drawn, without the line drawn over it, at the current point
        let center_undrawn =  self.strings_drawn.get_pixel(point.0 as u32,point.1 as u32).as_lab(&ColorSpace::Lab);
        let lr_mix_undrawn = StringPath::left_right_mix(&pin_a, &pin_b, point, &self.strings_drawn);
        let mix_undrawn = center_undrawn.mix(&lr_mix_undrawn, self.edge_weight);
        //Calculate the mixed color of strings_drawn, with the line drawn over it, at the current point
        let mix_drawn = line_color.mix(&lr_mix_undrawn, self.edge_weight);
        //Score using the unmixed colors at (x,y). Meant to represent the score with the assumption that the current color will be dense in this area.
        let score_unmixed = line_color.similarity_to(&center_input) - center_undrawn.similarity_to(&center_input);
        //Score using the mixed colors at (x,y). Meant to represent the score with the assumption that that the current color will be sparse in this area.
        let score_mixed = mix_drawn.similarity_to(&mix_input) - mix_undrawn.similarity_to(&mix_input);
        //Choose the best of the two scores 
        score_unmixed.max(score_mixed)
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
    pub fn overlap_width(&self, combo_a: &(usize, usize), combo_b: &(usize, usize)) -> f32
    {
        let diff_a = point!{x: self.pin_positions[combo_a.0].0 - self.pin_positions[combo_a.1].0,
                                        y: self.pin_positions[combo_a.0].1 - self.pin_positions[combo_a.1].1};
        let diff_b = point!{x: self.pin_positions[combo_b.0].0 - self.pin_positions[combo_b.1].0,
                                        y: self.pin_positions[combo_b.0].1 - self.pin_positions[combo_b.1].1};
        let mag_a = (diff_a.x() * diff_a.x() + diff_a.y() * diff_a.y()).sqrt();
        let mag_b = (diff_b.x() * diff_b.x() + diff_b.y() * diff_b.y()).sqrt();

        let dot = diff_a.dot(diff_b);
        let angle = (dot / (mag_a * mag_b)).acos();
        let overlap_width = 1. / angle.sin();
        if overlap_width > mag_a || overlap_width > mag_b {return mag_a.min(mag_b)};
        overlap_width
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