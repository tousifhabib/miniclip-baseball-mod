//! The shift: a mod that has the fielders stand where the batting side has
//! been hitting the ball.
//!
//! Every fair ball is remembered by how far across the field it came down.
//! Before each pitch the middle of the field is taken to be where the last
//! few went on the whole, and the fielders who roam stand as far to either
//! side of that as they stood of the real middle. They are moved wherever
//! they are drawn: on the field, behind the pitcher in the batting view,
//! and on the little field in its corner.

mod standing;

use super::Line;
use crate::mods::About;
use crate::rules::ShiftRules;

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "the_shift",
    name: "THE SHIFT",
    does: "FIELDERS STAND WHERE YOU HIT IT: GO THE OTHER WAY",
    setting: None,
};

/// The mod, in play: where the balls have been going, and where that has
/// the fielders standing.
pub(crate) struct TheShift {
    rules: ShiftRules,
    /// How far across the field each fair ball of this game came down,
    /// from 0 at one foul line to 1 at the other, in the order they were
    /// hit.
    spray: Vec<f32>,
    /// How far the middle has moved for the pitch in hand: to the left
    /// below nought, to the right above it.
    by: f32,
}

impl TheShift {
    pub fn new(rules: &ShiftRules) -> TheShift {
        TheShift {
            rules: rules.clone(),
            spray: Vec::new(),
            by: 0.0,
        }
    }

    /// A fair ball has come down this far across the field. It is
    /// remembered, caught or not.
    pub fn remember(&mut self, across: f32) {
        self.spray.push(across);
    }

    pub fn by(&self) -> f32 {
        self.by
    }

    /// Works out where the fielders stand for the coming pitch, by where
    /// the last few balls went.
    pub fn stand(&mut self) -> Shift {
        let shift = Shift::of(&self.spray, &self.rules);
        self.by = shift.by();
        shift
    }

    /// What the corner of the batting view says of where they stand, when
    /// they have moved.
    pub fn line(&self, shift: Shift) -> Option<Line> {
        shift.words(&self.rules).map(|words| Line {
            name: "shift",
            words: words.to_owned(),
            colour: [0xc8, 0xf0, 0xff],
        })
    }
}

/// How near a foul line a fielder may be moved, the width of the field
/// between the lines being 1.
const MARGIN: f32 = 0.04;

/// Where the fielders take the middle of the field to be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shift {
    /// How far across the field, from 0 on the left foul line to 1 on the
    /// right. With no shift on it is a half.
    pub middle: f32,
}

impl Shift {
    /// The shift for a side whose fair balls came down this far across the
    /// field, the latest last.
    pub fn of(spray: &[f32], rules: &ShiftRules) -> Shift {
        let from = spray.len().saturating_sub(rules.memory as usize);
        let last = &spray[from..];
        if last.is_empty() || last.len() < rules.least as usize {
            return Shift { middle: 0.5 };
        }
        let usual = last
            .iter()
            .map(|across| across.clamp(0.0, 1.0))
            .sum::<f32>()
            / last.len() as f32;
        let by = ((usual - 0.5) * rules.follow).clamp(-rules.most, rules.most);
        Shift { middle: 0.5 + by }
    }

    /// How far the middle has moved: to the left if less than nought.
    pub fn by(self) -> f32 {
        self.middle - 0.5
    }

    /// Where a fielder who stands this far across the field stands now.
    /// The foul lines stay where they are, and the field between them is
    /// squeezed on the side the middle has moved to and stretched on the
    /// other.
    pub fn across(self, across: f32) -> f32 {
        let middle = self.middle.clamp(MARGIN, 1.0 - MARGIN);
        let moved = if across <= 0.5 {
            across * middle / 0.5
        } else {
            1.0 - (1.0 - across) * (1.0 - middle) / 0.5
        };
        moved.clamp(MARGIN, 1.0 - MARGIN)
    }

    /// What the player is told of it, if it is enough to tell.
    pub fn words(self, rules: &ShiftRules) -> Option<&'static str> {
        match self.by() {
            by if by <= -rules.told => Some("SHIFT LEFT"),
            by if by >= rules.told => Some("SHIFT RIGHT"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
