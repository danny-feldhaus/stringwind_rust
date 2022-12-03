use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct PegGroup {
    pegs: Vec<(f32, f32)>,
}

impl PegGroup {
    pub fn new_circle_at_center(peg_count: usize, dimensions: (u32, u32), radius: f32) -> Self {
        assert!(radius > 0. && radius < 1.);
        let mut pegs = vec![(0., 0.); peg_count];
        let center = ((dimensions.0 / 2) as f32, (dimensions.1 / 2) as f32);
        for i in 0..peg_count {
            let angle = std::f32::consts::PI * 2. * (i as f32) / peg_count as f32;
            pegs[i].0 = center.0 + angle.cos() * center.0 * radius;
            pegs[i].1 = center.1 + angle.sin() * center.1 * radius;
        }
        PegGroup { pegs }
    }
    pub fn at(&self, index: usize) -> (f32, f32) {
        assert!(index < self.len());
        self.pegs[index]
    }
    pub fn len(&self) -> usize {
        self.pegs.len()
    }
    pub fn clockwise_steps_between(&self, start_peg_idx: usize, end_peg_idx: usize) -> usize {
        assert!(start_peg_idx < self.len() && end_peg_idx < self.len());
        if start_peg_idx <= end_peg_idx {
            start_peg_idx + (self.len() - end_peg_idx)
        } else {
            start_peg_idx - end_peg_idx
        }
    }
    pub fn cclockwise_steps_between(&self, start_peg_idx: usize, end_peg_idx: usize) -> usize {
        assert!(start_peg_idx < self.len() && end_peg_idx < self.len());
        if start_peg_idx <= end_peg_idx {
            end_peg_idx - start_peg_idx
        } else {
            start_peg_idx + (self.len() - end_peg_idx)
        }
    }
    pub fn steps_between(&self, peg_a_idx: usize, peg_b_idx: usize) -> usize {
        self.clockwise_steps_between(peg_a_idx, peg_b_idx)
            .min(self.cclockwise_steps_between(peg_a_idx, peg_b_idx))
    }
}

#[cfg(test)]
mod test {
    use crate::string_path::peg::PegGroup;

    #[test]
    fn test_distance() {
        let pg = PegGroup::new_circle_at_center(10, (10, 10), 0.9);
        assert_eq!(
            pg.steps_between(0, 5),
            pg.steps_between(5, 0)
        );
        assert_eq!(pg.steps_between(0, 1), 1);
        assert_eq!(pg.steps_between(0, 9), 1);
        assert_eq!(pg.steps_between(5, 5), 0);
        assert_eq!(pg.clockwise_steps_between(0, 1), 9);
        assert_eq!(pg.cclockwise_steps_between(0, 1), 1);
        assert_eq!(
            pg.clockwise_steps_between(5, 6),
            pg.clockwise_steps_between(8, 9)
        );
        assert_eq!(
            pg.cclockwise_steps_between(5,6),
            pg.cclockwise_steps_between(8,9)
        );
    }
}
