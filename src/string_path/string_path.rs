use crate::{
    tri_vec::TriVec,
    image_module::lab::{LabImageBuffer, LabaImageBuffer, LabBuf, LabDifference, get_color_name},
};
use super::string_setting::StringSettings;

use std::path::Path;
use rand::distributions::{WeightedIndex,Distribution};
use geo::{Line, coord, algorithm::line_intersection::line_intersection, LineIntersection};
use image:: {RgbaImage, Rgba, Rgba32FImage, ImageResult, EncodableLayout, DynamicImage};
use palette::{Lab, Laba, Mix};
use line_drawing::XiaolinWu;
use show_image::{ImageView, ImageInfo, create_window};


#[derive(Clone, Copy)]
pub struct PathStep
{
    pub from_idx : usize,
    pub to_idx : usize,
    pub color_idx : usize,
    pub score : f32
}

#[derive(Default, Clone, PartialEq)]
pub enum StringCombo
{
    AllowedScored(f32),
    AllowedUnscored,
    #[default]
    Banned
}

#[derive(Default)]
pub struct StringPath
{
    pub path : Vec<PathStep>, //Each element of vector is (fron_index, to_index, color_index)
    pub pin_positions : Vec<(f32, f32)>, //Pin positions in unit space
    pin_radius : f32,
    input_image_path : String,
    input_image : LabImageBuffer, //Input image in Lab color space
    output_path : String,
    colors : Vec<Lab>,
    _background : Lab,
    path_length : usize,
    //Internally generated
    combo_scores : TriVec<Vec<StringCombo>>,
    pub strings_drawn : LabImageBuffer,
    pub cur_step : usize,
    pub cur_idxs : Vec<usize>,
    pub cur_scores : Vec<f32>,
    edge_weight : f32
}

impl StringPath
{
    pub fn new(settings: StringSettings) -> Result<StringPath, String>
    {
        let background = *settings.get::<Lab>("bg_color")?;
        let colors = settings.get::<Vec<Lab>>("str_colors")?.clone();

        let pin_count = *settings.get::<usize>("pin_count")?;
        let path_length = *settings.get::<usize>("line_count")?;

        let input_image_path = settings.get::<String>("in_image_path")?.clone();
        let binder = LabImageBuffer::from_file(&input_image_path);
        if binder.is_err()  {return Err(binder.is_err().to_string())}

        let input_image = binder.unwrap();

        let output_path = settings.get::<String>("out_image_path")?.clone();
        let dimensions = input_image.dimensions();
        //Make pins
        let pin_radius = *settings.get::<f32>("pin_radius")?;
        let pin_positions = pin_circle(pin_count,pin_radius, input_image.dimensions());
        let strings_drawn = LabImageBuffer::from_lab(
            dimensions.0,
            dimensions.1, 
            &Laba::new(
                background.l, 
                background.a,
                background.b,
                1.
            )
        );
        //Make combo scores iterator

        let cur_idxs = vec![0;colors.len()];
        let cur_scores = vec![0.;colors.len()];
        let edge_weight = *settings.get::<f32>("edge_weight")?;
        let mut sp = StringPath
        {
            path: Vec::new(),
            pin_positions,
            pin_radius,
            input_image_path,
            input_image,
            output_path,
            combo_scores : TriVec::new(pin_count, &vec![StringCombo::Banned; colors.len()]),
            colors,
            _background : background,
            path_length,
            strings_drawn,
            cur_step: 0,
            cur_idxs,
            cur_scores,
            edge_weight
        };
        sp.populate_allowed_combos();

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
        self.strings_drawn.save(&path)
    } 

    //Add a step to the path
    pub fn step(&mut self) -> bool
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
        self.strings_drawn.draw_line(from_coord, to_coord, &self.colors[step.color_idx], false);
        self.path.push(step);
        self.unscore_intersected(&step);
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
    fn populate_allowed_combos(&mut self)
    {
        for x in 0..self.pin_positions.len()
        {
            for y in x+1..self.pin_positions.len()
            {
                for c in self.combo_scores.at(x,y)
                {
                    *c = StringCombo::AllowedUnscored;
                }
            }
        }
    }
    
    //Calculate the current score of the given line (its similarity to the image vs the similarity without it)
    fn calculate_current_score(&mut self, color_idx: usize, pin_combo: &(usize, usize)) -> Option<f32>
    {
        match self.combo_scores.at(pin_combo.0, pin_combo.1)[color_idx]
        {
            StringCombo::AllowedScored(s) => return Some(s),
            StringCombo::Banned => return None,
            StringCombo::AllowedUnscored =>
            {
                let pin_a = self.pin_positions[pin_combo.0];
                let pin_b = self.pin_positions[pin_combo.1];
                let line = XiaolinWu::<f32, i32>::new(pin_a, pin_b);
                let mut score_sum = 0_f32;
                let mut weight_sum = 0_f32;
                let line_color = self.colors[color_idx];
                for ((x,y), weight) in line
                {
                    let score = self.score_at_point(&(x,y), &pin_a, &pin_b, &line_color) * weight;
                    //println!("Score 1: {score}\nScore 2: {score_2}\n");
                    score_sum += score;
                    weight_sum += weight;
                }
                let score = score_sum / weight_sum;
                self.combo_scores.at(pin_combo.0, pin_combo.1)[color_idx] = StringCombo::AllowedScored(score);
                return Some(score)
            }
        }        
    }
    
    fn score_at_point(&self, point: &(i32,i32), line_start: &(f32, f32), line_end: &(f32, f32), line_color: &Lab) -> f32
    {
        let mixed_input = StringPath::mix_at_point(line_start, line_end, point, &self.input_image, self.edge_weight).unwrap();
        let unmixed_input = StringPath::mix_at_point(line_start, line_end, point, &self.input_image, 0.).unwrap();
        let mixed_undrawn = StringPath::mix_at_point(line_start, line_end, point, &self.strings_drawn, self.edge_weight);
        let unmixed_undrawn = StringPath::mix_at_point(line_start, line_end, point, &self.strings_drawn, 0.);
        let mixed_drawn = StringPath::mix_line_at_point(line_start, line_end, point, &self.strings_drawn, self.edge_weight, line_color);
        let unmixed_drawn = line_color;

        let score_mixed = match mixed_undrawn
        {
            Some(mixed_undrawn_color) => 
            {
                mixed_drawn.similarity_to(&mixed_input) - mixed_undrawn_color.similarity_to(&mixed_input)
            },
            None => mixed_drawn.similarity_to(&mixed_input)
        };
        let score_unmixed = match unmixed_undrawn
        {
            Some(unmixed_undrawn_color) =>
            {
                unmixed_drawn.similarity_to(&unmixed_input) - unmixed_undrawn_color.similarity_to(&unmixed_input)
            }
            None => unmixed_drawn.similarity_to(&unmixed_input)
        };
        score_mixed.max(score_unmixed)
        //let score_unmixed = line_color.similarity_to(&center_input) - center_undrawn.similarity_to(&center_input);
        //let score_mixed = mix_drawn.similarity_to(&mix_input) - mix_undrawn.similarity_to(&mix_input);
        ////Choose the best of the two scores 
        //score_unmixed.max(score_mixed)
    }

    fn unscore_intersected(&mut self, step: &PathStep)
    {
        let inter_a = self.pin_positions[step.from_idx];
        let inter_b = self.pin_positions[step.to_idx];
        for color_idx in 0..self.colors.len()
        {
            for x in 0..self.pin_positions.len()
            {
                for y in x+1..self.pin_positions.len()
                {
                    if self.do_intersect(&(step.from_idx,step.to_idx), &(x,y), color_idx)
                    {
                        self.combo_scores.at(x,y)[color_idx] = StringCombo::AllowedUnscored;
                    }
                }
            }
        }
    }
    
    fn do_intersect(&mut self, combo_a: &(usize, usize), combo_b: &(usize, usize), color_idx : usize) -> bool
    {
        if StringCombo::Banned == self.combo_scores.at(combo_a.0,combo_a.1)[color_idx] ||
           StringCombo::Banned == self.combo_scores.at(combo_b.0,combo_b.1)[color_idx]
        {
            return false;
        }

        let from_a = self.pin_positions[combo_a.0];
        let to_a   = self.pin_positions[combo_a.1];
        let line_a = Line::new(coord!{x: from_a.0,y: from_a.1}, coord!{x: to_a.0,y: to_a.1});

        let from_b = self.pin_positions[combo_b.0];
        let to_b   = self.pin_positions[combo_b.1];
        let line_b = Line::new(coord!{x: from_b.0,y: from_b.1}, coord!{x: to_b.0,y: to_b.1});

        let a_b_intersection = line_intersection(line_a, line_b);
        if a_b_intersection.is_none() {return false};
        let a_b_intersection = a_b_intersection.unwrap();
        match a_b_intersection
        {
            LineIntersection::SinglePoint { intersection: _, is_proper} => {return is_proper},
            LineIntersection::Collinear { intersection: _ } => return true,
        }
    }

    fn mix_at_point(start : &(f32,f32), end : &(f32, f32), center: &(i32, i32), image : &LabImageBuffer, edge_weight: f32) -> Option<Lab>
    {
        let center_color = image.get_pixel(center.0 as u32, center.1 as u32);
        if edge_weight == 0.
            {return Some(center_color)};
        match StringPath::left_right_mix(start, end, center, image)
        {
            Some(lr_color) =>
            {
                return Some(center_color.mix(&lr_color, edge_weight));
            },
            None => 
            {
                return Some(center_color);
            }
        }
    }

    fn mix_line_at_point(start : &(f32,f32), end : &(f32, f32), center: &(i32, i32), image : &LabImageBuffer, edge_weight: f32, line_color: &Lab) -> Lab
    {
        let line_weight = 1. - edge_weight;
        match StringPath::left_right_mix(start, end, center, image)
        {
            Some(lr_color) =>
            {
                let line_lr_ratio = (edge_weight) / (edge_weight + line_weight);
                return line_color.mix(&lr_color, line_lr_ratio);
            },
            None => 
            {
                return *line_color;
            }
        }
    }

    fn left_right_mix(start : &(f32,f32), end : &(f32, f32), center: &(i32, i32), image : &LabImageBuffer) -> Option<Lab>
    {
        fn left_right_colors(start : &(f32,f32), end : &(f32, f32), center: &(i32, i32), image : &LabImageBuffer) -> (Option<Lab>, Option<Lab>)
        {
            let diff = (end.0 - start.0, end.1 - start.1);
            let len = (diff.0*diff.0 + diff.1*diff.1).sqrt();
            let offset = ((diff.0 / len).round() as i32,  (diff.1 / len).round() as i32);
            let l_coord = ((center.0 + offset.1) as u32, (center.1 - offset.0) as u32);
            let r_coord  = ((center.0 - offset.1) as u32, (center.1 + offset.0) as u32);
            let l_pix = image.get_pixel(l_coord.0, l_coord.1);
            let r_pix = image.get_pixel(r_coord.0, r_coord.1);
            let l_opt = Some(l_pix);
            let r_opt = Some(r_pix);
            (l_opt, r_opt)
        }
        let (l_opt,r_opt) = left_right_colors(start, end, center, image);
        if l_opt.is_some() && r_opt.is_some()
        {
            let l_color = l_opt.unwrap();
            let r_color = r_opt.unwrap();
            let lr_mix = l_color.mix(&r_color, 0.5);
            let lr_weight = 1.;//(l_weight + r_weight) / 2.;
            return Some(lr_mix);
        }
        else if l_opt.is_some()
        {
            return Some(l_opt.unwrap());
        }
        else if r_opt.is_some()
        {
            return Some(r_opt.unwrap());
        }
        return None;
    }

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