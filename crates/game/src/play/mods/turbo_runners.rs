//! Turbo runners: runners are several times as fast, as you set, and can be
//! sent on with the ball in the air.
//!
//! A runner's run is a clip that plays a frame at a time. Turbo runners are
//! hurried on through it by more frames than that. A runner on a base can
//! be sent on at any time the ball is in play, where as the game was he had
//! to wait for it to come down or be caught.

use crate::mods::{About, Setting};
use crate::rules::TurboRules;

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "turbo_runners",
    name: "TURBO RUNNERS",
    does: "RUNNERS ARE FAST, AND CAN GO ON WITH THE BALL IN THE AIR",
    setting: Some(Setting {
        name: "SPEED",
        usual: 2,
        levels: |rules| rules.turbo.speed.count(),
        words: |level, rules| format!("{}X", rules.turbo.speed.at(level).unwrap_or(1.0)),
    }),
};

pub(crate) struct TurboRunners {
    /// How many times as fast as usual the runners go.
    speed: f32,
    /// The part of a frame the runners are owed, on top of the whole frames
    /// they have been hurried on by.
    owed: f32,
}

impl TurboRunners {
    /// `level` is the setting the mod is at, the first being 1.
    pub fn new(rules: &TurboRules, level: u8) -> TurboRunners {
        TurboRunners {
            speed: rules.speed.at(level).unwrap_or(1.0),
            owed: 0.0,
        }
    }

    /// How many frames more than the usual one the runners are moved on by
    /// this frame. A speed that is not a whole number is made up over
    /// several frames.
    pub fn hurry(&mut self) -> u16 {
        self.owed += (self.speed - 1.0).max(0.0);
        let hurried = self.owed.floor() as u16;
        self.owed -= f32::from(hurried);
        hurried
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    #[test]
    fn twice_as_fast_is_one_frame_more_every_frame() {
        let mut turbo = TurboRunners::new(&Rules::default().turbo, 2);
        let hurried: Vec<u16> = (0..4).map(|_| turbo.hurry()).collect();
        assert_eq!(hurried, [1, 1, 1, 1]);
    }

    #[test]
    fn half_as_fast_again_is_one_frame_more_every_other_frame() {
        let mut turbo = TurboRunners::new(&Rules::default().turbo, 1);
        let hurried: Vec<u16> = (0..6).map(|_| turbo.hurry()).collect();
        assert_eq!(hurried, [0, 1, 0, 1, 0, 1]);
    }
}
