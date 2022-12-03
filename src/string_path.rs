pub mod io;
mod peg;
mod scoring;

use peg::PegGroup;

use crate::{
    image_module::lab::{get_color_name, LabBuf, LabDifference, LabaImageBuffer},
    tri_vec::TriVec,
};

use csv::Writer;
use geo::{algorithm::line_intersection::line_intersection, coord, Line, LineIntersection};
use image::{ImageBuffer};
use line_drawing::XiaolinWu;
use palette::{IntoColor, Laba, Mix, Srgb, Srgba};
use rand::distributions::{Distribution, WeightedIndex};
use serde::{Deserialize, Serialize};
use std::io::read_to_string;
use std::{error::Error};
use svg::node::element::{path::Data, Path as PathSVG};
use svg::Document;

#[derive(Clone, Copy, Serialize)]
pub struct PathStep {
    pub from_idx: usize,
    pub to_idx: usize,
    pub color_idx: usize,
    pub score: f32,
}

#[derive(Default, Clone, PartialEq)]
pub enum StringCombo {
    Allowed(Option<f32>),
    Filled,
    #[default]
    Banned,
}

#[derive(Default, Serialize, Deserialize)]
pub struct StringPath {
    input_image_path: String,
    output_path: String,
    path_length: usize,
    pin_count: usize,
    colors_raw: Vec<(f32, f32, f32)>,
    pin_circle_radius: f32,
    score_radius: f32,
    edge_weight: f32,

    #[serde(skip_deserializing)]
    pub path: Vec<PathStep>,
    #[serde(skip_deserializing)]
    pub pegs: PegGroup,
    #[serde(skip)]
    colors: Vec<Laba>,
    #[serde(skip)]
    combo_scores: TriVec<Vec<StringCombo>>,
    #[serde(skip)]
    input_image: LabaImageBuffer,
    #[serde(skip)]
    pub strings_drawn: LabaImageBuffer,
    #[serde(skip)]
    coverage_map: CoverageBuffer,
    #[serde(skip_deserializing)]
    pub cur_step: usize,
    #[serde(skip_deserializing)]
    pub cur_idxs: Vec<usize>,
    #[serde(skip_deserializing)]
    pub cur_scores: Vec<Option<f32>>,
}
type CoverageBuffer = ImageBuffer<image::Luma<u16>, Vec<u16>>;

impl StringPath {
    pub fn from_file(settings_path: &str) -> Result<StringPath, Box<dyn Error>> {
        println!("Loading settings from file {settings_path}");
        let f = std::fs::File::open(settings_path)?;
        let settings_str = read_to_string(f)?;
        let mut sp: StringPath = toml::from_str(&settings_str)?;
        //Populate derived elements
        println!("Loading input image from {}", sp.input_image_path);
        sp.input_image = LabaImageBuffer::from_file(&sp.input_image_path)?;
        sp.pegs = PegGroup::new_circle_at_center(sp.pin_count, sp.dimensions(), sp.pin_circle_radius);
        sp.strings_drawn =
            LabaImageBuffer::from_lab(sp.width(), sp.height(), &Laba::new(0., 0., 0., 0.));
        //convert the raw rgb values to LAB
        sp.colors = sp
            .colors_raw
            .iter()
            .map(|(r, g, b)| -> Laba { Srgba::new(r / 255., g / 255., b / 255., 1.).into_color() })
            .collect();
        sp.cur_idxs = vec![0; sp.color_count()];
        sp.cur_scores = vec![None; sp.color_count()];
        sp.combo_scores = TriVec::new(sp.pin_count, &vec![StringCombo::Banned; sp.color_count()]);
        println!("Populating allowed combos");
        sp.populate_allowed_combos();
        println!("Calculating coverage map");
        sp.coverage_map = Self::generate_coverage_map(
            &sp.dimensions(),
            &sp.combo_scores,
            &sp.pegs,
            sp.score_radius,
        );
        Ok(sp)
    }

    //Add a step to the path
    pub fn step(&mut self) -> bool {
        self.cur_step += 1;
        if self.cur_step == self.path_length {
            return false;
        };
        let next_steps = self.get_best_steps();
        let mut index_weights = vec![0_f32; self.color_count()];
        for (color_idx, step_opt) in next_steps.iter().enumerate() {
            if step_opt.is_some() && step_opt.unwrap().score > 0. {
                let step = step_opt.unwrap();
                self.cur_scores[step.color_idx] = Some(step.score);
                index_weights[step.color_idx] = step.score.clamp(0., 1.);
                println!("\tColor {} is good.", step.color_idx);
            } else {
                let none_step = PathStep {
                    from_idx: self.cur_idxs[color_idx],
                    to_idx: (self.cur_idxs[color_idx] + 1) % self.pin_count,
                    color_idx,
                    score: -1.,
                };
                self.cur_idxs[color_idx] = none_step.to_idx;
                self.cur_scores[color_idx] = if step_opt.is_some() {
                    Some(step_opt.unwrap().score)
                } else {
                    None
                };
                self.path.push(none_step);
                println!("\tColor {color_idx} is bad.");
            }
        }
        //if at least one choice is viable, make a weighted random choice for the next step
        if index_weights.iter().any(|w| *w != 0.) {
            let dist = WeightedIndex::new(index_weights).unwrap();
            let next_color_idx = dist.sample(&mut rand::thread_rng());
            let step = next_steps[next_color_idx].unwrap();

            self.cur_idxs[next_color_idx] = step.to_idx;
            let from_coord = self.pegs.at(step.from_idx);
            let to_coord = self.pegs.at(step.to_idx);
            self.strings_drawn
                .draw_line(from_coord, to_coord, &self.colors[next_color_idx], true);
            self.path.push(step);
            self.set_combo_to_filled(step.from_idx, step.to_idx);
            self.unscore_intersected(&step);
            self.decrease_coverage(&step);
            println!(
                "\tPicked Color {}. Moving to {}.",
                next_color_idx, self.cur_idxs[step.color_idx]
            );
        }

        true
    }

    //Set all colors to
    fn set_combo_to_filled(&mut self, pin_a_idx: usize, pin_b_idx: usize) {
        for c in self.combo_scores.at_mut(pin_a_idx, pin_b_idx) {
            *c = StringCombo::Filled;
        }
    }

    //Calculate the color / pin combination with the best score
    pub fn get_best_steps(&mut self) -> Vec<Option<PathStep>> {
        let mut best_steps = Vec::<Option<PathStep>>::new();
        for color_idx in 0..self.color_count() {
            let best_step = self.get_best_step(color_idx);
            best_steps.push(best_step);
        }
        best_steps
    }

    //Calculate the best pin to move to for the given color
    fn get_best_step(&mut self, color_idx: usize) -> Option<PathStep> {
        let from_idx = self.cur_idxs[color_idx];
        let mut found_step = false;
        let mut best_step = PathStep {
            from_idx,
            color_idx,
            to_idx: 0,
            score: -1.,
        };

        for to_idx in 0..self.pin_count {
            let pin_combo = (
                std::cmp::min(from_idx, to_idx),
                std::cmp::max(from_idx, to_idx),
            );
            let current_score_opt = self.calculate_current_score(color_idx, &pin_combo);
            let mut best_color_for_step: usize = self.color_count() + 1;
            let mut best_score_for_step: f32 = -1.;
            for cur_color_idx in 0..self.color_count() {
                let current_score_opt = self.calculate_current_score(cur_color_idx, &pin_combo);
                if current_score_opt.is_some() && current_score_opt.unwrap() > best_score_for_step {
                    best_color_for_step = cur_color_idx;
                    best_score_for_step = current_score_opt.unwrap();
                }
            }
            if best_color_for_step == color_idx && best_score_for_step > best_step.score {
                best_step.score = current_score_opt.unwrap();
                best_step.to_idx = if pin_combo.0 == from_idx {
                    pin_combo.1
                } else {
                    pin_combo.0
                };
                found_step = true;
            }
        }
        if found_step {
            Some(best_step)
        } else {
            None
        }
    }

    //Calculate the initial score of every possible line
    fn populate_allowed_combos(&mut self) {
        let pin_count = self.pin_count;
        for x in 0..self.pin_count {
            for y in x + 1..self.pin_count {
                let d_left = y - x;
                let d_right = (pin_count + x) - y;

                if d_left.min(d_right) > 20 {
                    for c in self.combo_scores.at_mut(x, y) {
                        *c = StringCombo::Allowed(None);
                    }
                }
            }
        }
    }

    //Calculate the current score of the given line (its similarity to the image vs the similarity without it)
    fn calculate_current_score(
        &mut self,
        color_idx: usize,
        pin_combo: &(usize, usize),
    ) -> Option<f32> {
        match self.combo_scores.at(pin_combo.0, pin_combo.1)[color_idx] {
            StringCombo::Allowed(s) => match s {
                Some(score) => return Some(score),
                None => {
                    let pin_a = self.pegs.at(pin_combo.0);
                    let pin_b = self.pegs.at(pin_combo.1);
                    let line = XiaolinWu::<f32, i32>::new(pin_a, pin_b);
                    let mut score_sum = 0_f32;
                    let mut weight_sum = 0_f32;
                    for ((x, y), weight) in line {
                        if self.coverage_map.get_pixel(x as u32, y as u32)[0] == 2 {
                            let score =
                                self.score_at_point(&(x, y), &pin_a, &pin_b, color_idx) * weight;
                            score_sum += score;
                            weight_sum += weight;
                        }
                    }
                    if weight_sum != 0. {
                        let score = score_sum / weight_sum;
                        self.combo_scores.at_mut(pin_combo.0, pin_combo.1)[color_idx] =
                            StringCombo::Allowed(Some(score));
                        return Some(score);
                    } else {
                        return None;
                    }
                }
            },
            _ => return None,
        }
    }

    //Calculate the score for the color at (color_idx) at (point)
    fn score_at_point(
        &self,
        point: &(i32, i32),
        line_start: &(f32, f32),
        line_end: &(f32, f32),
        color_idx: usize,
    ) -> f32 {
        let line_color = &self.colors[color_idx];
        let mixed_input = StringPath::mix_at_point(
            line_start,
            line_end,
            point,
            &self.input_image,
            self.edge_weight,
        )
        .unwrap();
        let unmixed_input =
            StringPath::mix_at_point(line_start, line_end, point, &self.input_image, 0.).unwrap();
        let mixed_undrawn = StringPath::mix_at_point(
            line_start,
            line_end,
            point,
            &self.strings_drawn,
            self.edge_weight,
        );
        let unmixed_undrawn =
            StringPath::mix_at_point(line_start, line_end, point, &self.strings_drawn, 0.);
        let mixed_drawn = StringPath::mix_line_at_point(
            line_start,
            line_end,
            point,
            &self.strings_drawn,
            self.edge_weight,
            line_color,
        );
        let unmixed_drawn = line_color;
        let score_unmixed = match unmixed_undrawn {
            Some(unmixed_undrawn) => {
                unmixed_drawn.color.similarity_to(&unmixed_input)
                    - unmixed_undrawn.color.similarity_to(&unmixed_input)
            }
            None => unmixed_drawn.color.similarity_to(&unmixed_input),
        };
        let score_mixed = match mixed_undrawn {
            Some(mixed_undrawn) => {
                mixed_drawn.color.similarity_to(&mixed_input)
                    - mixed_undrawn.color.similarity_to(&mixed_input)
            }
            None => mixed_drawn.color.similarity_to(&mixed_input),
        };
        score_mixed.max(score_unmixed)
        //let score_unmixed = line_color.similarity_to(&center_input) - center_undrawn.similarity_to(&center_input);
        //let score_mixed = mix_drawn.similarity_to(&mix_input) - mix_undrawn.similarity_to(&mix_input);
        ////Choose the best of the two scores
        //score_unmixed.max(score_mixed)
    }

    //Un-score all combos that intersect with the given step.
    //This indicates that the score needs to be re-calculated the next time the combo is checked.
    fn unscore_intersected(&mut self, step: &PathStep) {
        for color_idx in 0..self.color_count() {
            for x in 0..self.pin_count {
                for y in x + 1..self.pin_count {
                    if self.do_intersect(&(step.from_idx, step.to_idx), &(x, y), color_idx) {
                        self.combo_scores.at_mut(x, y)[color_idx] = StringCombo::Allowed(None);
                    }
                }
            }
        }
    }

    fn decrease_coverage(&mut self, step: &PathStep) {
        let from_pin = self.pegs.at(step.from_idx);
        let to_pin = self.pegs.at(step.to_idx);
        let line = XiaolinWu::<f32, i32>::new(from_pin, to_pin);
        for ((x, y), _weight) in line {
            let pix = self.coverage_map.get_pixel_mut(x as u32, y as u32);
            if pix[0] > 0 {
                pix[0] -= 1;
            }
        }
    }

    fn do_intersect(
        &mut self,
        combo_a: &(usize, usize),
        combo_b: &(usize, usize),
        color_idx: usize,
    ) -> bool {
        if matches!(
            self.combo_scores.at(combo_a.0, combo_a.1)[color_idx],
            StringCombo::Allowed(_)
        ) && matches!(
            self.combo_scores.at(combo_b.0, combo_b.1)[color_idx],
            StringCombo::Allowed(_)
        ) {
            return false;
        }

        let from_a = self.pegs.at(combo_a.0);
        let to_a = self.pegs.at(combo_a.1);
        let line_a = Line::new(
            coord! {x: from_a.0,y: from_a.1},
            coord! {x: to_a.0,y: to_a.1},
        );

        let from_b = self.pegs.at(combo_b.0);
        let to_b = self.pegs.at(combo_b.1);
        let line_b = Line::new(
            coord! {x: from_b.0,y: from_b.1},
            coord! {x: to_b.0,y: to_b.1},
        );

        let a_b_intersection = line_intersection(line_a, line_b);
        if a_b_intersection.is_none() {
            return false;
        };
        let a_b_intersection = a_b_intersection.unwrap();
        match a_b_intersection {
            LineIntersection::SinglePoint {
                intersection: _,
                is_proper,
            } => return is_proper,
            LineIntersection::Collinear { intersection: _ } => return true,
        }
    }

    fn mix_at_point(
        start: &(f32, f32),
        end: &(f32, f32),
        center: &(i32, i32),
        image: &LabaImageBuffer,
        edge_weight: f32,
    ) -> Option<Laba> {
        let center_color = image.get_pixel(center.0 as u32, center.1 as u32);
        if center_color.alpha == 0. {
            return None;
        }
        let lr_color = StringPath::left_right_mix(start, end, center, image);
        let lr_weight = edge_weight * lr_color.alpha / (lr_color.alpha + center_color.alpha);
        Some(center_color.mix(&lr_color, lr_weight))
    }

    fn mix_line_at_point(
        start: &(f32, f32),
        end: &(f32, f32),
        center: &(i32, i32),
        image: &LabaImageBuffer,
        edge_weight: f32,
        line_color: &Laba,
    ) -> Laba {
        let lr_color = StringPath::left_right_mix(start, end, center, image);
        let lr_weight = edge_weight * lr_color.alpha;
        line_color.mix(&lr_color, lr_weight)
    }

    fn left_right_mix(
        start: &(f32, f32),
        end: &(f32, f32),
        center: &(i32, i32),
        image: &LabaImageBuffer,
    ) -> Laba {
        fn left_right_colors(
            start: &(f32, f32),
            end: &(f32, f32),
            center: &(i32, i32),
            image: &LabaImageBuffer,
        ) -> (Laba, Laba) {
            let diff = (end.0 - start.0, end.1 - start.1);
            let len = (diff.0 * diff.0 + diff.1 * diff.1).sqrt();
            let offset = ((diff.0 / len).round() as i32, (diff.1 / len).round() as i32);
            let l_coord = ((center.0 + offset.1) as u32, (center.1 - offset.0) as u32);
            let r_coord = ((center.0 - offset.1) as u32, (center.1 + offset.0) as u32);
            let l_pix = image.get_pixel(l_coord.0, l_coord.1);
            let r_pix = image.get_pixel(r_coord.0, r_coord.1);
            (l_pix, r_pix)
        }
        let (l_pix, r_pix) = left_right_colors(start, end, center, image);
        let r_pix_weight = r_pix.alpha / (r_pix.alpha + l_pix.alpha);
        l_pix.mix(&r_pix, r_pix_weight)
    }

    fn generate_coverage_map(
        image_dimensions: &(u32, u32),
        lines: &TriVec<Vec<StringCombo>>,
        pegs: &PegGroup,
        score_radius: f32,
    ) -> CoverageBuffer {
        let mut coverage_map = CoverageBuffer::new(image_dimensions.0, image_dimensions.1);
        let center = (image_dimensions.0 as i32 / 2, image_dimensions.1 as i32 / 2);
        let d_edge_to_center = ((image_dimensions.0.min(image_dimensions.1)) / 2) as f32;

        for pin_a_idx in 0..lines.size {
            for pin_b_idx in pin_a_idx + 1..lines.size {
                if lines.at(pin_a_idx, pin_b_idx)[0] != StringCombo::Banned {
                    let pin_a = pegs.at(pin_a_idx);
                    let pin_b = pegs.at(pin_b_idx);
                    let line = XiaolinWu::<f32, i32>::new(pin_a, pin_b);
                    for ((x, y), _weight) in line {
                        let d_from_center =
                            (((x - center.0).pow(2) + (y - center.1).pow(2)) as f32).sqrt();
                        let p_from_center = d_from_center / d_edge_to_center;
                        let coverage_pix = &mut coverage_map.get_pixel_mut(x as u32, y as u32)[0];
                        if p_from_center <= score_radius && *coverage_pix < 10 {
                            *coverage_pix += 1;
                        }
                    }
                }
            }
        }
        coverage_map
    }

    pub fn cleaned_path(&self) -> Vec<PathStep> {
        let mut cur_idxs: Vec<Option<usize>> = vec![None; self.color_count()];
        let mut cpath = Vec::<PathStep>::new();
        for step in self.path.iter() {
            if cur_idxs[step.color_idx].is_none() {
                cur_idxs[step.color_idx] = Some(step.from_idx);
            }
            let cur_idx = &mut cur_idxs[step.color_idx].unwrap();
            let d_step = self.pegs.steps_between(step.from_idx, step.to_idx);
            //If this is an important step, add it to the cleaned path
            if d_step > 1 {
                cpath.push(*step);
                *cur_idx = step.to_idx;
            }
            //If this is an unimportant step, but it's far enough away from the last important step,
            //  jump from the last important step to this one
            else if !matches!(
                self.combo_scores.at(*cur_idx, step.to_idx)[step.color_idx],
                StringCombo::Banned
            ) {
                cpath.push(PathStep {
                    from_idx: *cur_idx,
                    to_idx: step.to_idx,
                    color_idx: step.color_idx,
                    score: 0.,
                });
                *cur_idx = step.to_idx;
            }
        }
        cpath
    }

    pub fn write_to_csv(&self, file_path: &str) -> Result<(), Box<dyn Error>> {
        #[derive(serde::Serialize)]
        struct Row {
            color_index: usize,
            pin_index: usize,
            pin_position_x: f32,
            pin_position_y: f32,
            color_r: f32,
            color_g: f32,
            color_b: f32,
            color_name: String,
        }
        let path = self.path.clone();
        let mut wtr = Writer::from_path(file_path)?;
        for step in path {
            let c_rgb: Srgb = self.colors[step.color_idx].color.into_color();
            wtr.serialize(Row {
                color_index: step.color_idx,
                pin_index: step.from_idx,
                pin_position_x: self.pegs.at(step.from_idx).0,
                pin_position_y: self.pegs.at(step.from_idx).1,
                color_r: c_rgb.red,
                color_g: c_rgb.green,
                color_b: c_rgb.blue,
                color_name: get_color_name(&self.colors[step.color_idx].color),
            })?;
        }
        wtr.flush()?;
        Ok(())
    }

    pub fn write_to_svg(&self, file_path: &str) -> std::io::Result<()> {
        let path = self.path.clone(); //cleaned_path();
        let mut document = Document::new()
            .set("viewBox", (0, 0, self.width(), self.height()))
            .set("style", "background-color:black");
        for step in path {
            let from = self.pegs.at(step.from_idx);
            let to = self.pegs.at(step.to_idx);
            let c_rgb: Srgb = self.colors[step.color_idx].color.into_color();
            let c_str = format!(
                "rgb({},{},{})",
                (c_rgb.red * 255.) as u8,
                (c_rgb.green * 255.) as u8,
                (c_rgb.blue * 255.) as u8
            );
            let data = Data::new().move_to(from).line_to(to);
            let path = PathSVG::new()
                .set("fill", "none")
                .set("stroke", c_str)
                .set("stroke-width", 1)
                .set("d", data);
            document = document.clone().add(path);
        }
        svg::save(file_path, &document)
    }

    fn color_count(&self) -> usize {
        self.colors.len()
    }
    fn dimensions(&self) -> (u32, u32) {
        self.input_image.dimensions()
    }
    fn width(&self) -> u32 {
        self.input_image.width()
    }
    fn height(&self) -> u32 {
        self.input_image.height()
    }
}
