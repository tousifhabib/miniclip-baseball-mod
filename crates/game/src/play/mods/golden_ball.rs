//! Golden ball: every fifth pitch of a match is gold. Every run scored off
//! it counts for three, and a strike on it puts the batter out whatever the
//! count.
//!
//! A foul is still a foul and a ball is still a ball. The arcade game has
//! no runs or outs for a golden ball to change, so the mod has no place
//! there.

use bb_engine::math::ColorTransform;
use bb_engine::stage::Stage;

use super::Line;
use crate::play::Parts;
use crate::rules::GoldenRules;

/// What turns the white of the ball to gold.
const GOLD: ColorTransform = ColorTransform {
    mult: [1.0, 0.8, 0.22, 1.0],
    add: [0.0, 0.0, 0.0, 0.0],
};

pub(crate) struct GoldenBall {
    rules: GoldenRules,
}

impl GoldenBall {
    pub fn new(rules: &GoldenRules) -> GoldenBall {
        GoldenBall {
            rules: rules.clone(),
        }
    }

    /// Whether the pitch with this number, the first being 1, is a golden
    /// one.
    pub fn is_gold(&self, pitch: u32) -> bool {
        self.rules.is_gold(pitch)
    }

    /// How many times over a run scored off a golden ball counts.
    pub fn runs(&self) -> u32 {
        self.rules.runs
    }

    /// Colours the ball gold wherever it is drawn: coming in, in the air,
    /// and on the field.
    pub fn gild(parts: &Parts, stage: &mut Stage) {
        for ball in [&parts.ball, &parts.fly_ball, &parts.field_ball] {
            if let Some(ball) = stage.child_mut(ball) {
                ball.set_color(GOLD);
            }
        }
    }

    /// What the corner of the batting view says of a golden ball.
    pub fn line() -> Line {
        Line {
            name: "goldenBall",
            words: "GOLDEN BALL".to_owned(),
            colour: [0xff, 0xd2, 0x40],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    #[test]
    fn every_fifth_pitch_is_gold_and_worth_three() {
        let golden = GoldenBall::new(&Rules::default().golden);
        let gold: Vec<u32> = (1..=12).filter(|&pitch| golden.is_gold(pitch)).collect();
        assert_eq!(gold, [5, 10]);
        assert_eq!(golden.runs(), 3);
    }
}
