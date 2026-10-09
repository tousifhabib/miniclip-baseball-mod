//! Tired arm: the pitcher slows and misses the zone more as his pitches
//! mount up, until a fresh one comes in.
//!
//! His first pitches are as they always were. From there each takes a
//! little longer than the last, and he aims at a wider and wider area about
//! the same middle. How many he has thrown is written in the corner of the
//! batting view, from white through yellow to red, and he grows flushed.
//! In a full match the count goes on from one innings to the next. The
//! arcade game is over before any arm tires, so the mod has no place there.

use bb_engine::math::ColorTransform;

use super::Line;
use crate::look::Rgb;
use crate::play::overlay::Says;
use crate::play::snapshot::ArmSeen;
use crate::rules::{PitchRules, TiredArmRules};

/// How much of the green and the blue of a pitcher goes when he is spent:
/// he is flushed.
const FLUSH: f32 = 0.22;
/// The colour of the news that a new pitcher has come in.
const NEWS_COLOUR: Rgb = [0xc8, 0xf0, 0xff];

pub(crate) struct TiredArm {
    rules: TiredArmRules,
    /// Pitches the pitcher on the mound has thrown.
    thrown: u32,
    /// How many pitchers have been taken off.
    relieved: u32,
    /// How tired he was for the last pitch got ready, from 0, as good as
    /// ever, to 1, spent. `None` before the first.
    tired: Option<f32>,
}

impl TiredArm {
    pub fn new(rules: &TiredArmRules) -> TiredArm {
        TiredArm {
            rules: rules.clone(),
            thrown: 0,
            relieved: 0,
            tired: None,
        }
    }

    /// A pitch has left his hand.
    pub fn threw(&mut self) {
        self.thrown += 1;
    }

    /// Brings a fresh pitcher in if this one has thrown his last. Returns
    /// whether it did.
    pub fn relieve(&mut self) -> bool {
        let spent = self.thrown >= self.rules.relief.max(1);
        if spent {
            self.thrown = 0;
            self.relieved += 1;
        }
        spent
    }

    /// The news that a new pitcher has come in, to be put where it goes.
    pub fn news(&self) -> Says<'static> {
        Says::news(
            "newPitcher",
            "NEW PITCHER",
            NEWS_COLOUR,
            self.rules.told_time,
        )
    }

    /// Works out how tired the pitcher is and makes the coming pitch as
    /// much slower and wilder as that says. Returns how tired, from 0 to 1.
    pub fn tire(&mut self, table: &mut PitchRules) -> f32 {
        let tired = self.rules.tired(self.thrown);
        self.tired = Some(tired);
        *table = self.rules.pitch(table, tired);
        tired
    }

    /// What a pitcher this tired is tinted: the more spent, the more
    /// flushed.
    pub fn flush(tired: f32) -> ColorTransform {
        let left = 1.0 - FLUSH * tired;
        ColorTransform {
            mult: [1.0, left, left, 1.0],
            add: [0.0; 4],
        }
    }

    /// What the corner of the batting view says of a pitcher this tired:
    /// how many he has thrown, from white, through yellow, to red.
    pub fn line(&self, tired: f32) -> Line {
        Line {
            name: "pitches",
            words: format!("PITCHES {}", self.thrown),
            colour: [
                0xff,
                (0xff as f32 - 0x90 as f32 * tired) as u8,
                (0xff as f32 - 0xc0 as f32 * tired.min(0.5) * 2.0) as u8,
            ],
        }
    }

    /// How the arm is holding up, once a pitch has been got ready.
    pub fn seen(&self) -> Option<ArmSeen> {
        self.tired.map(|tired| ArmSeen {
            thrown: self.thrown,
            tired,
            relieved: self.relieved,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;
    use crate::settings::Difficulty;

    fn arm_and_table() -> (TiredArm, PitchRules) {
        let rules = Rules::default();
        let table = rules.pitch.at(Difficulty::Medium).clone();
        (TiredArm::new(&rules.tired_arm), table)
    }

    #[test]
    fn nothing_is_told_of_the_arm_before_a_pitch_is_got_ready() {
        let (mut arm, usual) = arm_and_table();
        assert_eq!(arm.seen(), None);
        arm.tire(&mut usual.clone());
        let seen = arm.seen().expect("an arm that has been looked at");
        assert_eq!((seen.thrown, seen.tired, seen.relieved), (0, 0.0, 0));
    }

    #[test]
    fn a_fresh_arm_pitches_as_ever_and_a_spent_one_slower_and_wider() {
        let (mut arm, usual) = arm_and_table();
        let mut fresh = usual.clone();
        assert_eq!(arm.tire(&mut fresh), 0.0);
        assert_eq!(fresh, usual);
        for _ in 0..Rules::default().tired_arm.spent {
            arm.threw();
        }
        let mut spent = usual.clone();
        assert_eq!(arm.tire(&mut spent), 1.0);
        assert!(spent.speed.high > usual.speed.high, "{:?}", spent.speed);
        assert!(spent.target.width > usual.target.width);
    }

    #[test]
    fn a_new_pitcher_comes_in_when_this_one_has_thrown_his_last() {
        let (mut arm, usual) = arm_and_table();
        let relief = Rules::default().tired_arm.relief;
        for _ in 0..relief - 1 {
            arm.threw();
        }
        assert!(!arm.relieve());
        arm.threw();
        assert!(arm.relieve());
        arm.tire(&mut usual.clone());
        let seen = arm.seen().expect("an arm that has been looked at");
        assert_eq!((seen.thrown, seen.tired, seen.relieved), (0, 0.0, 1));
    }

    #[test]
    fn the_line_goes_from_white_to_red_and_the_pitcher_grows_flushed() {
        let (arm, _) = arm_and_table();
        assert_eq!(arm.line(0.0).colour, [0xff, 0xff, 0xff]);
        assert_eq!(arm.line(1.0).colour, [0xff, 0x6f, 0x3f]);
        assert_eq!(TiredArm::flush(0.0).mult, [1.0; 4]);
        assert_eq!(TiredArm::flush(1.0).mult, [1.0, 0.78, 0.78, 1.0]);
    }
}
