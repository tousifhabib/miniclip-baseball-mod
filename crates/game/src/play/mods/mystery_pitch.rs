//! Mystery pitch: each pitch is a fastball, a change-up or a curve, and you
//! find out as it is thrown.
//!
//! Nothing gives it away beforehand: the pitcher stands as long before one
//! as before another, and the marker of where the pitch will cross is not
//! shown until the ball has left his hand. Then the pitch is named over
//! him.

use crate::play::overlay::Says;
use crate::play::pitch::Kind;
use crate::rng::Rng;
use crate::rules::{MysteryRules, PitchRules};

pub(crate) struct MysteryPitch {
    rules: MysteryRules,
}

impl MysteryPitch {
    pub fn new(rules: &MysteryRules) -> MysteryPitch {
        MysteryPitch {
            rules: rules.clone(),
        }
    }

    /// Picks which kind the coming pitch is and changes the numbers it is
    /// picked by to suit. The marker is kept back until `release_frame`,
    /// when the ball leaves the pitcher's hand.
    ///
    /// Two numbers are drawn whatever comes up: the kind, and which way a
    /// curve would break.
    pub fn pick(&self, table: &mut PitchRules, release_frame: u16, rng: &mut Rng) -> Kind {
        let which = Kind::ALL[rng.below(Kind::ALL.len() as u32) as usize];
        which.shape(table, &self.rules, rng.below(2) == 0);
        table.marker_frame = release_frame;
        which
    }

    /// The name of the pitch, to be put up over the pitcher once it can be
    /// told.
    pub fn news(&self, kind: Kind) -> Says<'static> {
        Says::news(
            "mysteryPitch",
            kind.words(),
            [0xff, 0xf2, 0x8a],
            self.rules.told_time,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;
    use crate::settings::Difficulty;

    #[test]
    fn every_kind_comes_up_and_the_marker_waits_for_the_ball_to_be_thrown() {
        let rules = Rules::default();
        let mystery = MysteryPitch::new(&rules.mystery);
        let usual = rules.pitch.at(Difficulty::Medium).clone();
        let mut rng = Rng::new(3);
        let mut seen = Vec::new();
        for _ in 0..60 {
            let mut table = usual.clone();
            let kind = mystery.pick(&mut table, rules.throw.release_frame, &mut rng);
            assert_eq!(table.marker_frame, rules.throw.release_frame);
            if !seen.contains(&kind) {
                seen.push(kind);
            }
        }
        assert_eq!(seen.len(), Kind::ALL.len(), "{seen:?}");
    }

    #[test]
    fn picking_a_pitch_always_draws_two_numbers() {
        // The pitches that follow must come out the same whichever kind
        // this one was, so the draws cannot depend on it.
        let rules = Rules::default();
        let mystery = MysteryPitch::new(&rules.mystery);
        let usual = rules.pitch.at(Difficulty::Medium).clone();
        for seed in 0..20 {
            let (mut picked, mut drawn) = (Rng::new(seed), Rng::new(seed));
            mystery.pick(&mut usual.clone(), 78, &mut picked);
            drawn.below(3);
            drawn.below(2);
            assert_eq!(picked.below(1000), drawn.below(1000));
        }
    }
}
