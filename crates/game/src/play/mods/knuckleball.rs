//! Knuckleball: pitches sway from side to side, and the crossing marker is
//! only roughly right.
//!
//! The marker shows where the pitch was going before it began to sway, so
//! the ball crosses somewhere near it and not always on it. The hit goes by
//! where the ball really was, and a pitch that sways out of the strike zone
//! is a ball.

use crate::play::pitch::{Mound, Pitch};
use crate::rules::KnuckleballRules;

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
