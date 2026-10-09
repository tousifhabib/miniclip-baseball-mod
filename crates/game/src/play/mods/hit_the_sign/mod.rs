//! Hit the sign: a mod that puts signs along the outfield wall and pays in
//! runs for a ball that strikes one.
//!
//! The signs stand side by side on the wall, and one of them is lit, a
//! different one each innings. A ball that comes to the wall at a sign, no
//! higher than the sign is tall, has struck it: the lit one is worth the
//! most. The signs are drawn on the wall in both views. In the batting
//! view they are where the art's pointer shows a hit going, so the pointer
//! on a sign is a hit at it.

mod board;

use crate::look::Rgb;
use crate::mods::About;
use crate::play::overlay::Says;
use crate::rng::{Rng, mixed_with};
use crate::rules::SignRules;
pub(crate) use board::Board;

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "hit_the_sign",
    name: "HIT THE SIGN",
    does: "SIGNS ON THE WALL PAY RUNS, THE LIT ONE MOST OF ALL",
    setting: None,
};

/// The mod, in play: which sign is lit, and what a ball that struck one
/// was worth.
pub(crate) struct HitTheSign {
    /// The innings a sign was last lit for, and which it was, counting
    /// from 0.
    lit: Option<(u32, usize)>,
    /// What the next sign to be lit is drawn by.
    rng: Rng,
    /// The sign a ball has just struck and the runs that was worth, until
    /// that has been told.
    pub news: Option<(usize, u32)>,
    /// The same for the pitch in hand, once it has been told.
    pub struck: Option<(usize, u32)>,
}

impl HitTheSign {
    /// `seed` is the game's own, from which the signs' is made.
    pub fn new(seed: u64) -> HitTheSign {
        HitTheSign {
            lit: None,
            rng: Rng::new(seed ^ mixed_with::THE_SIGNS),
            news: None,
            struck: None,
        }
    }

    /// Which sign is lit, counting from 0, once one has been.
    pub fn lit(&self) -> Option<usize> {
        self.lit.map(|(_, lit)| lit)
    }

    /// The sign a ball has struck on the pitch in hand and what it paid,
    /// told yet or not.
    pub fn struck(&self) -> Option<(usize, u32)> {
        self.struck.or(self.news)
    }

    /// The sign that is lit for this innings, counting from 0. A new
    /// innings lights another.
    pub fn light(&mut self, innings: u32, signs: &Signs) -> usize {
        let count = signs.count() as u32;
        match self.lit {
            Some((lit_in, lit)) if lit_in == innings && lit < signs.count() => lit,
            // Never the one that was lit the innings before.
            Some((_, was)) if count > 1 => {
                let pick = self.rng.below(count - 1) as usize;
                let lit = if pick >= was { pick + 1 } else { pick };
                self.lit = Some((innings, lit));
                lit
            }
            _ => {
                let lit = self.rng.below(count) as usize;
                self.lit = Some((innings, lit));
                lit
            }
        }
    }
}

/// The colours of a sign and of what is written on it: unlit, lit, and lit
/// on the beat.
const UNLIT: (Rgb, Rgb) = ([0x16, 0x34, 0x58], [0x9c, 0xc4, 0xe4]);
const LIT: (Rgb, Rgb) = ([0xff, 0xd2, 0x2a], [0x7a, 0x1a, 0x08]);
const BEAT: (Rgb, Rgb) = ([0xff, 0xf4, 0xb0], [0x7a, 0x1a, 0x08]);
/// Where the view of the field says that a sign was struck: how far down,
/// and in what colour.
const NEWS_TOP: f32 = 250.0;
const NEWS_COLOUR: Rgb = [0xff, 0xe2, 0x4a];

/// Where the signs are along the wall.
#[derive(Clone, Debug, PartialEq)]
pub struct Signs {
    /// How far across the field the middle of each is, from 0 on the left
    /// foul line to 1 on the right, and how wide each is, the same way.
    middles: Vec<f32>,
    width: f32,
}

impl Signs {
    pub fn of(rules: &SignRules) -> Signs {
        let count = rules.count.max(1);
        let apart = match count {
            1 => 0.0,
            count => (rules.last - rules.first) / (count - 1) as f32,
        };
        Signs {
            middles: (0..count)
                .map(|sign| rules.first + apart * sign as f32)
                .collect(),
            width: rules.width,
        }
    }

    pub fn count(&self) -> usize {
        self.middles.len()
    }

    /// Where a sign begins and ends, across the field.
    pub fn span(&self, sign: usize) -> (f32, f32) {
        let middle = self.middles[sign];
        (middle - self.width / 2.0, middle + self.width / 2.0)
    }

    /// The sign at this place across the field, counting from 0, if there
    /// is one there.
    pub fn at(&self, across: f32) -> Option<usize> {
        (0..self.count()).find(|&sign| {
            let (from, to) = self.span(sign);
            (from..=to).contains(&across)
        })
    }
}

/// What the view of the field says when a sign has been struck for this
/// many runs.
pub(crate) fn news_words(runs: u32) -> String {
    format!("OFF THE SIGN! +{runs}")
}

/// How it says them, for this many frames. `centre_x` is the middle of
/// that view.
pub(crate) fn news(words: &str, frames: u32, centre_x: f32) -> Says<'_> {
    Says::news("signNews", words, NEWS_COLOUR, frames)
        .at((centre_x, NEWS_TOP))
        .sized(1.2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    #[test]
    fn the_signs_stand_side_by_side_along_the_wall() {
        let rules = Rules::default().sign;
        let signs = Signs::of(&rules);
        assert_eq!(signs.count(), 5);
        // The first and the last are where the rules put them, and none
        // overlaps the next.
        let spans: Vec<(f32, f32)> = (0..5).map(|sign| signs.span(sign)).collect();
        assert!(((spans[0].0 + spans[0].1) / 2.0 - rules.first).abs() < 1e-5);
        assert!(((spans[4].0 + spans[4].1) / 2.0 - rules.last).abs() < 1e-5);
        assert!(spans.windows(2).all(|pair| pair[0].1 < pair[1].0));
        assert!(spans.iter().all(|span| span.0 > 0.0 && span.1 < 1.0));
        // A ball at the middle of one strikes it, and one in the gap
        // between two strikes nothing.
        assert_eq!(signs.at(rules.first), Some(0));
        assert_eq!(signs.at(0.52), Some(2));
        assert_eq!(signs.at((spans[1].1 + spans[2].0) / 2.0), None);
        assert_eq!(signs.at(0.02), None);
        // One sign alone is where the first would be.
        let one = Signs::of(&SignRules { count: 1, ..rules });
        assert_eq!(one.count(), 1);
        assert_eq!(one.at(0.24), Some(0));
    }
}
