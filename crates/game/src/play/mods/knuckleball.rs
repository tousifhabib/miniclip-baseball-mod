//! Knuckleball: pitches sway from side to side, and the crossing marker is
//! only roughly right.
//!
//! The marker shows where the pitch was going before it began to sway, so
//! the ball crosses somewhere near it and not always on it. The hit goes by
//! where the ball really was, and a pitch that sways out of the strike zone
//! is a ball.

use crate::mods::About;
use crate::play::pitch::{Mound, Pitch};
use crate::rules::KnuckleballRules;

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "knuckleball",
    name: "KNUCKLEBALL",
    does: "PITCHES SWAY, AND THE MARKER IS ONLY ROUGHLY RIGHT",
    setting: None,
};

pub(crate) struct Knuckleball {
    rules: KnuckleballRules,
}

impl Knuckleball {
    pub fn new(rules: &KnuckleballRules) -> Knuckleball {
        Knuckleball {
            rules: rules.clone(),
        }
    }

    /// Makes a pitch sway on its way in. `start` is where in a turn it sets
    /// off, from 0 to 1, which is left to chance.
    pub fn sway(&self, pitch: &mut Pitch, start: f32, mound: &Mound) {
        pitch.knuckle(self.rules.sway, self.rules.turns, start, mound);
    }
}
