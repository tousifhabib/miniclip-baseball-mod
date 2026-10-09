//! The arcade game: a fixed number of pitches, and a target on the outfield
//! to drop the ball on.
//!
//! Pitching and batting are the match's. What differs is here: there is no
//! count and nobody fields, and a ball that is hit scores by the ring of the
//! target it comes down in.

mod play;

use super::field::distance;
use super::pitch::Point;
use crate::rules::ArcadeRules;

pub(crate) struct Arcade {
    /// Pitches still to come.
    pub left: u32,
    pub points: u32,
    /// The middle of the target, in the overhead field's pixels.
    target: Point,
    /// Which rings have scored on this pitch, from the centre out.
    lit: Vec<bool>,
    /// How far from the target the ball was a frame ago, as the rings
    /// measure it.
    last_distance: f32,
    /// The ball has gone over the wall, and is taken away when it lands.
    cleared: bool,
    /// The overhead view is up and the ball is being followed.
    flying: bool,
    /// The feet a zinger that is still in the air will score when it comes
    /// down: with that mod on, a hit scores by how far it goes.
    pub owed: Option<u32>,
}

impl Arcade {
    pub fn new(pitches: u32) -> Arcade {
        Arcade {
            left: pitches,
            points: 0,
            target: (0.0, 0.0),
            lit: Vec::new(),
            last_distance: f32::MAX,
            cleared: false,
            flying: false,
            owed: None,
        }
    }

    /// How far a point on the grass is from the target as the rings count
    /// it. The target is drawn lying flat, so a miss up or down the field
    /// counts for more than one to the side.
    fn off_target(&self, ball: Point, rules: &ArcadeRules) -> f32 {
        distance(ball, self.target) + rules.depth_weight * (ball.1 - self.target.1).abs()
    }

    /// Scores a ball that has just touched the ground `off` from the target.
    /// Returns the ring it lit, counting from the centre, and the points.
    fn touch(&mut self, off: f32, rules: &ArcadeRules) -> Option<(usize, u32)> {
        let ring = rules
            .rings
            .iter()
            .position(|ring| off > ring.over && off <= ring.within)?;
        if self.lit.get(ring).copied().unwrap_or(true) {
            return None;
        }
        // The first ring scored on a pitch counts extra.
        let first = !self.lit.iter().any(|&lit| lit);
        self.lit[ring] = true;
        let points = rules.rings[ring].points * if first { rules.first_bonus } else { 1 };
        self.points += points;
        Some((ring, points))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    fn arcade() -> (Arcade, Rules) {
        let rules = Rules::default();
        let mut arcade = Arcade::new(rules.arcade.pitches);
        arcade.target = (200.0, 180.0);
        arcade.lit = vec![false; rules.arcade.rings.len()];
        (arcade, rules)
    }

    #[test]
    fn the_first_ring_scored_on_a_pitch_counts_double() {
        let (mut arcade, rules) = arcade();
        // Dead centre first: the middle ring, doubled.
        assert_eq!(arcade.touch(5.0, &rules.arcade), Some((0, 200)));
        // A later hop into the next ring out scores once.
        assert_eq!(arcade.touch(30.0, &rules.arcade), Some((1, 75)));
        assert_eq!(arcade.points, 275);
    }

    #[test]
    fn a_ring_scores_only_once_a_pitch() {
        let (mut arcade, rules) = arcade();
        assert!(arcade.touch(50.0, &rules.arcade).is_some());
        assert_eq!(arcade.touch(50.0, &rules.arcade), None);
    }

    #[test]
    fn a_ball_in_the_gap_between_two_rings_scores_nothing() {
        let (mut arcade, rules) = arcade();
        assert_eq!(arcade.touch(62.5, &rules.arcade), None);
        assert_eq!(arcade.touch(10.5, &rules.arcade), None);
        assert_eq!(arcade.points, 0);
    }

    #[test]
    fn a_ball_that_comes_down_wide_scores_nothing() {
        let (mut arcade, rules) = arcade();
        assert_eq!(arcade.touch(86.0, &rules.arcade), None);
        assert_eq!(arcade.points, 0);
    }

    #[test]
    fn a_miss_up_the_field_counts_for_more_than_one_to_the_side() {
        let (arcade, rules) = arcade();
        let aside = arcade.off_target((230.0, 180.0), &rules.arcade);
        let beyond = arcade.off_target((200.0, 150.0), &rules.arcade);
        assert_eq!(aside, 30.0);
        assert!(beyond > aside * 2.0);
    }

    #[test]
    fn the_best_a_pitch_can_score_is_every_ring_with_the_centre_first() {
        let (mut arcade, rules) = arcade();
        for off in [5.0, 30.0, 50.0, 80.0] {
            arcade.touch(off, &rules.arcade);
        }
        assert_eq!(arcade.points, 350);
    }
}
