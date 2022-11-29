use crate::{
    tri_vec::TriVec,
    image_module::lab::{LabaImageBuffer, LabBuf, LabDifference, get_color_name},
};
use super::string_setting::StringSettings;

use std::path::Path;
use rand::distributions::{WeightedIndex,Distribution};
use geo::{Line, coord, algorithm::line_intersection::line_intersection, LineIntersection};
use image:: {ImageBuffer, ImageResult, GrayImage};
use palette::{Lab, Laba, Mix};
use line_drawing::XiaolinWu;


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
    score_radius: f32,
    input_image_path : String,
    input_image : LabaImageBuffer, //Input image in Lab color space
    output_path : String,
    colors : Vec<Laba>,
    _background : Lab,
    path_length : usize,
    //Internally generated
    combo_scores : TriVec<Vec<StringCombo>>,
    pub strings_drawn : LabaImageBuffer,
    coverage_map: CoverageBuffer,
    string_layers: Vec<GrayImage>,
    pub cur_step : usize,
    pub cur_idxs : Vec<usize>,
    pub cur_scores : Vec<Option<f32>>,
    edge_weight : f32
}
type CoverageBuffer = ImageBuffer<image::Luma<u16>, Vec<u16>>;

impl StringPath
{
    pub fn new(settings: StringSettings) -> Result<StringPath, String>
    {
        let background = *settings.get::<Lab>("bg_color")?;
        let colors_lab = settings.get::<Vec<Lab>>("str_colors")?.clone();
        let colors: Vec<Laba> = colors_lab.iter().map(|c| Laba::new(c.l, c.a, c.b, 1.)).collect();
        let pin_count = *settings.get::<usize>("pin_count")?;
        let path_length = *settings.get::<usize>("line_count")?;

        let input_image_path = settings.get::<String>("in_image_path")?.clone();
        let binder = LabaImageBuffer::from_file(&input_image_path);
        if binder.is_err()  {return Err(binder.is_err().to_string())}

        let input_image = binder.unwrap();

        let output_path = settings.get::<String>("out_image_path")?.clone();
        let dimensions = input_image.dimensions();
        //Make pins
        let pin_radius = *settings.get::<f32>("pin_radius")?;
        let score_radius = *settings.get::<f32>("score_radius")?;
        let pin_positions = pin_circle(pin_count,pin_radius, input_image.dimensions());
        let strings_drawn = LabaImageBuffer::from_lab(
            dimensions.0,
            dimensions.1, 
            &Laba::new(
                background.l, 
                background.a,
                background.b,
                0.
            )
        );
        let string_layers = vec![GrayImage::new(dimensions.0,dimensions.1); colors.len()];
        //Make combo scores iterator

        let cur_idxs = vec![0;colors.len()];
        let cur_scores = vec![None;colors.len()];
        let edge_weight = *settings.get::<f32>("edge_weight")?;
        let combo_scores = TriVec::new(pin_count, &vec![StringCombo::Banned; colors.len()]);
        let mut sp = StringPath
        {
            path: Vec::new(),
            pin_positions,
            pin_radius,
            score_radius,
            input_image_path,
            input_image,
            output_path,
            combo_scores,
            colors,
            _background : background,
            path_length,
            strings_drawn,
            coverage_map : Default::default(),
            string_layers,
            cur_step: 0,
            cur_idxs,
            cur_scores,
            edge_weight
        };
        sp.populate_allowed_combos();
        sp.coverage_map = Self::generate_coverage_map(&sp.input_image.dimensions(), &sp.combo_scores, &sp.pin_positions, sp.score_radius);
        {
            let mut coverage_disp = image::GrayImage::new(sp.coverage_map.width(), sp.coverage_map.height());
            for (disp, coverage) in coverage_disp.iter_mut().zip(sp.coverage_map.iter())
            {
                *disp = *coverage as u8;
            }
            coverage_disp.save("/home/danny/Programming/Stringwind_Rust/stringwind/src/tests/images/output/coverage.png").unwrap();
        }
        Ok(sp)
    }

    //Save a visual representation of the current path
    pub fn save_visual(&self, verbose: bool) -> ImageResult<()>
    {
        let prefix = Path::new(&self.input_image_path).file_prefix().unwrap().to_str().unwrap();
        let color_names : Vec<String> =  self.colors.iter()
            .map(|c| get_color_name(c))
            .collect();
        let name_string = color_names.iter()
            .fold("".to_string(),|a,b| format!("{a},{b}"));
        let path = format!("{output_path}{prefix}_edgeweight:{edge_weight}_lines:{cur_step}{name_string}.png"
            ,output_path = self.output_path
            ,edge_weight = self.edge_weight
            ,cur_step = self.cur_step);
        println!("Saving to {path}");
        if verbose
        {
            let mut data_img = image::RgbImage::new(self.strings_drawn.width(), self.strings_drawn.height());    
            for x in 0..self.strings_drawn.width() as i32
            {
                for y in 0..self.strings_drawn.height() as i32
                {
                    let cur_pix = data_img.get_pixel_mut(x as u32, y as u32);
                    let cur_coverage = self.coverage_map.get_pixel(x as u32,y as u32)[0];
                    match cur_coverage.cmp(&2)
                    {
                        std::cmp::Ordering::Equal => 
                        {
                            cur_pix[0] = 255;
                            cur_pix[1] = 0;
                            cur_pix[2] = 0;
                        }
                        std::cmp::Ordering::Greater =>
                        {
                            cur_pix[0] = 255;
                            cur_pix[1] = 255;
                            cur_pix[2] = 255;
                        }
                        std::cmp::Ordering::Less =>
                        {
                            cur_pix[0] = 0;
                            cur_pix[1] = 0;
                            cur_pix[2] = 0;
                        }
                    }
                    
                }
            }
            let verbose_path = format!("{output_path}{prefix}_edgeweight:{edge_weight}_lines:{cur_step}{name_string}_verbose.png"
            ,output_path = self.output_path
            ,edge_weight = self.edge_weight
            ,cur_step = self.cur_step);
            self.strings_drawn.save(&path).and(data_img.save(&verbose_path))
        }
        else
        {
            self.strings_drawn.save(&path)
        }
    } 

    //Add a step to the path
    pub fn step(&mut self) -> bool
    {
        self.cur_step+= 1;
        if self.cur_step == self.path_length {return false};
        let next_steps = self.get_best_steps();
        let mut index_weights = vec![0_f32;self.colors.len()];
        for (color_idx, step_opt) in next_steps.iter().enumerate()
        {
            if step_opt.is_some() && step_opt.unwrap().score > 0.
            {
                let step = step_opt.unwrap();
                self.cur_scores[step.color_idx] = Some(step.score);
                index_weights[step.color_idx] = step.score.clamp(0.,1.);
                println!("\tColor {} is good.", step.color_idx);
            }
            else
            {
                let none_step = PathStep
                {
                    from_idx: self.cur_idxs[color_idx],
                    to_idx: (self.cur_idxs[color_idx] + 1) % self.pin_positions.len(),
                    color_idx,
                    score: -1.
                };
                self.cur_idxs[color_idx] = none_step.to_idx;
                self.cur_scores[color_idx] = if step_opt.is_some() {Some(step_opt.unwrap().score)} else {None};
                self.path.push(none_step);
                println!("\tColor {color_idx} is bad.");
            }
        }
        //if at least one choice is viable, make a weighted random choice for the next step
        if index_weights.iter().any(|w| *w != 0.)
        {
            let dist = WeightedIndex::new(index_weights).unwrap();
            let next_color_idx = dist.sample(&mut rand::thread_rng());
            let step = next_steps[next_color_idx].unwrap();

            self.cur_idxs[next_color_idx] = step.to_idx;
            let from_coord = self.pin_positions[step.from_idx];
            let to_coord = self.pin_positions[step.to_idx];
            self.strings_drawn.draw_line(from_coord, to_coord, &self.colors[next_color_idx], false);
            self.increment_string_layer(step.from_idx, step.to_idx, step.color_idx);
            self.path.push(step);
            self.combo_scores.at_mut(step.from_idx, step.to_idx)[next_color_idx] = StringCombo::Banned;
            self.unscore_intersected(&step);
            self.decrease_coverage(&step);
            println!("\tPicked Color {}. Moving to {}.", next_color_idx, self.cur_idxs[step.color_idx]);
        }

        true
    }

    //Calculate tehe color / pin combination with the best score
    pub fn get_best_steps(&mut self) -> Vec<Option<PathStep>>
    {
        let mut best_steps = Vec::<Option<PathStep>>::new();
        for color_idx in 0..self.colors.len()
        {
            let best_step = self.get_best_step(color_idx);
            best_steps.push(best_step);
        }   
        best_steps
    }

    //Calculate the best pin to move to for the given color
    fn get_best_step(&mut self, color_idx : usize) -> Option<PathStep>
    {
        let from_idx = self.cur_idxs[color_idx];
        let mut found_step = false;
        let mut best_step = PathStep {from_idx, color_idx, to_idx : 0, score : -1.};
        
        for to_idx in 0..self.pin_positions.len()
        {
            let pin_combo = (std::cmp::min(from_idx, to_idx), std::cmp::max(from_idx,to_idx));
            let current_score_opt = self.calculate_current_score(color_idx, &pin_combo);
            let mut best_color_for_step: usize = self.colors.len() + 1;
            let mut best_score_for_step: f32 = -1.;
            for cur_color_idx in 0..self.colors.len()
            {
                let current_score_opt = self.calculate_current_score(cur_color_idx, &pin_combo);
                if current_score_opt.is_some() && current_score_opt.unwrap() > best_score_for_step
                {
                    best_color_for_step = cur_color_idx;
                    best_score_for_step = current_score_opt.unwrap();
                }
            }
            if best_color_for_step == color_idx && best_score_for_step > best_step.score
            {
                best_step.score  = current_score_opt.unwrap();
                best_step.to_idx = if pin_combo.0 == from_idx {pin_combo.1} else {pin_combo.0};
                found_step = true;
            }           
        }
        if found_step
        {
            Some(best_step)
        }
        else
        {
            None
        }
    }
    
    //Calculate the initial score of every possible line
    fn populate_allowed_combos(&mut self)
    {
        let pin_count = self.pin_positions.len();
        for x in 0..self.pin_positions.len()
        {
            for y in x+1..self.pin_positions.len()
            {
                let d_left = y-x;
                let d_right = (pin_count + x) - y;

                if d_left.min(d_right) > 20
                {
                    for c in self.combo_scores.at_mut(x,y)
                    {
                        *c = StringCombo::AllowedUnscored;
                    }
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
                    if self.coverage_map.get_pixel(x as u32,y as u32)[0] == 2
                    {
                        let score = self.score_at_point(&(x,y), &pin_a, &pin_b, color_idx) * weight;
                        score_sum += score;
                        weight_sum += weight;
                    }
                }
                if weight_sum != 0.
                {
                    let score = score_sum / weight_sum;
                    self.combo_scores.at_mut(pin_combo.0, pin_combo.1)[color_idx] = StringCombo::AllowedScored(score);
                    return Some(score)
                }
                else
                {
                    return None
                }
            }
        }        
    }
    
    fn score_at_point(&self, point: &(i32,i32), line_start: &(f32, f32), line_end: &(f32, f32), color_idx: usize) -> f32
    {
        let line_color = &self.colors[color_idx];
        let mixed_input = StringPath::mix_at_point(line_start, line_end, point, &self.input_image, self.edge_weight).unwrap();
        let unmixed_input = StringPath::mix_at_point(line_start, line_end, point, &self.input_image, 0.).unwrap();
        //let mixed_undrawn = StringPath::mix_at_point(line_start, line_end, point, &self.strings_drawn, self.edge_weight);
        //let unmixed_undrawn = StringPath::mix_at_point(line_start, line_end, point, &self.strings_drawn, 0.);
        let mixed_drawn = StringPath::mix_line_at_point(line_start, line_end, point, &self.strings_drawn, self.edge_weight, line_color);
        let unmixed_drawn = line_color;
        /*
        let score_unmixed = match unmixed_undrawn
        {
            Some(unmixed_undrawn) => unmixed_drawn.similarity_to(&unmixed_input) - unmixed_undrawn.similarity_to(&unmixed_input),
            None => unmixed_drawn.similarity_to(&unmixed_input)
        };
        let score_mixed = match mixed_undrawn
        {
            Some(mixed_undrawn) => mixed_drawn.similarity_to(&mixed_input) - mixed_undrawn.similarity_to(&mixed_input),
            None => mixed_drawn.similarity_to(&mixed_input)
        };
        */
        let score_unmixed = unmixed_drawn.similarity_to(&unmixed_input);
        let score_mixed = mixed_drawn.similarity_to(&mixed_input);
        let mut score = score_mixed.max(score_unmixed);

        let layer_count = self.string_layers[color_idx].get_pixel(point.0 as u32, point.1 as u32)[0];
        score *= 0.75_f32.powf(layer_count as f32);
        score
    }

    fn unscore_intersected(&mut self, step: &PathStep)
    {
        for color_idx in 0..self.colors.len()
        {
            for x in 0..self.pin_positions.len()
            {
                for y in x+1..self.pin_positions.len()
                {
                    if self.do_intersect(&(step.from_idx,step.to_idx), &(x,y), color_idx)
                    {
                        self.combo_scores.at_mut(x,y)[color_idx] = StringCombo::AllowedUnscored;
                    }
                }
            }
        }
    }

    fn decrease_coverage(&mut self, step: &PathStep)
    {
        let from_pin = self.pin_positions[step.from_idx];
        let to_pin = self.pin_positions[step.to_idx];
        let line = XiaolinWu::<f32, i32>::new(from_pin, to_pin);
        for ((x,y), _weight) in line
        {
            let pix = self.coverage_map.get_pixel_mut(x as u32, y as u32);
            if pix[0] > 0 
            {
                pix[0] -= 1;
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

    fn mix_at_point(start : &(f32,f32), end : &(f32, f32), center: &(i32, i32), image : &LabaImageBuffer, edge_weight: f32) -> Option<Laba>
    {
        let center_color = image.get_pixel(center.0 as u32, center.1 as u32);
        if center_color.alpha == 0. {return None}
        let lr_opt = StringPath::left_right_mix(start, end, center, image);
        match lr_opt
        {
            Some(lr_color) => return Some(center_color.mix(&lr_color, edge_weight)),
            None => return Some(center_color)
        }
    }

    fn mix_line_at_point(start : &(f32,f32), end : &(f32, f32), center: &(i32, i32), image : &LabaImageBuffer, edge_weight: f32, line_color: &Laba) -> Laba
    {
        let lr_opt = StringPath::left_right_mix(start, end, center, image);
        match lr_opt
        {
            Some(lr_color) => line_color.mix(&lr_color, edge_weight),
            None => *line_color
        }
    }

    fn left_right_mix(start : &(f32,f32), end : &(f32, f32), center: &(i32, i32), image : &LabaImageBuffer) -> Option<Laba>
    {
        fn left_right_colors(start : &(f32,f32), end : &(f32, f32), center: &(i32, i32), image : &LabaImageBuffer) -> (Option<Laba>,Option<Laba>)
        {
            let diff = (end.0 - start.0, end.1 - start.1);
            let len = (diff.0*diff.0 + diff.1*diff.1).sqrt();
            let offset = ((diff.0 / len).round() as i32,  (diff.1 / len).round() as i32);
            let l_coord = ((center.0 + offset.1) as u32, (center.1 - offset.0) as u32);
            let r_coord  = ((center.0 - offset.1) as u32, (center.1 + offset.0) as u32);
            let l_pix = image.get_pixel(l_coord.0, l_coord.1);
            let r_pix = image.get_pixel(r_coord.0, r_coord.1);
            (if l_pix.alpha != 0. {Some(l_pix)} else {None},
             if r_pix.alpha != 0. {Some(l_pix)} else {None})
        }
        let (l_pix,r_pix) = left_right_colors(start, end, center, image);
        if l_pix.is_none() && r_pix.is_none() {return None}
        if l_pix.is_some() {return l_pix}
        if r_pix.is_some() {return r_pix}
        return Some(l_pix?.mix(&r_pix?, 0.5));
    }

    fn generate_coverage_map(image_dimensions: &(u32, u32), lines: &TriVec<Vec<StringCombo>>, pin_positions: &Vec<(f32, f32)>, score_radius: f32) -> CoverageBuffer
    {
        let mut coverage_map = CoverageBuffer::new(image_dimensions.0, image_dimensions.1);
        let center = (image_dimensions.0 as i32/ 2,image_dimensions.1 as i32/ 2);
        let d_edge_to_center = ((image_dimensions.0.min(image_dimensions.1)) / 2) as f32;


        for pin_a_idx in 0..lines.size
        {
            for pin_b_idx in pin_a_idx+1..lines.size
            {
                if lines.at(pin_a_idx,pin_b_idx)[0] != StringCombo::Banned
                {
                    let pin_a = pin_positions[pin_a_idx];
                    let pin_b = pin_positions[pin_b_idx];
                    let line = XiaolinWu::<f32, i32>::new(pin_a,pin_b);
                    for ((x,y), _weight) in line
                    {
                        let d_from_center = ((((x - center.0).pow(2) + (y - center.1).pow(2))) as f32).sqrt();
                        let p_from_center = d_from_center / d_edge_to_center;
                        let coverage_pix = &mut coverage_map.get_pixel_mut(x as u32, y as u32)[0];
                        if p_from_center <= score_radius && *coverage_pix < 10
                        {
                            *coverage_pix += 1;
                        }
                    }
                }
            }
        }
        coverage_map
    }

    fn increment_string_layer(&mut self, pin_a_idx: usize, pin_b_idx: usize, color_idx: usize)
    {
        let pin_a = self.pin_positions[pin_a_idx];
        let pin_b = self.pin_positions[pin_b_idx];

        for (idx, layers) in self.string_layers.iter_mut().enumerate()
        {
            let line = XiaolinWu::<f32, i32>::new(pin_a,pin_b);
            if idx != color_idx
            {
                for ((x,y), _weight) in line
                {
                    layers.get_pixel_mut(x as u32, y as u32)[0] = 0;
                }
            }
            else
            {
                for ((x,y), _weight) in line.into_iter()
                {
                    let pix = &mut layers.get_pixel_mut(x as u32, y as u32)[0];
                    if *pix < u8::MAX
                    {
                        *pix += 1;
                    }
                }
            }
        }
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