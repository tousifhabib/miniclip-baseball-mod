//! Bullet time: a mod that slows the pitch as it comes to the plate for as
//! long as a key is held, and a meter lasts.
//!
//! Holding the key holds the ball back: of every few frames it moves on
//! only one. It does that only while the ball is near the plate, where the
//! swing has to be timed, and only until the swing has begun, since what
//! comes of a swing is settled when it is made. Every frame the ball is
//! held back takes one off the meter, and hits put some of it back.

use bb_engine::display::Path;
use bb_engine::input::Key;
use bb_engine::library::Library;
use bb_engine::math::{ColorTransform, Matrix};
use bb_engine::stage::Stage;

use super::overlay::{self, DARK, Words};
use super::pitch::Point;
use super::{AtBat, Match, Parts};
use crate::art;
use crate::look::{self, Rgb};
use crate::menu::Game;
use crate::mods::Mod;
use crate::rules::BulletTimeRules;

/// The key that is held.
pub const KEY: Key = Key::Char(' ');
/// How wide and how tall the meter is, and how much bigger all round the
/// dark bar it lies on.
const WIDE: f32 = 80.0;
const TALL: f32 = 5.0;
const EDGE: f32 = 1.0;
/// What is written over the meter, and the colours of that and of the
/// meter: with something in it, with nothing, and while it is in use.
const WORDS: &str = "SLOW: SPACE";
const READY: Rgb = [0x7c, 0xe4, 0xff];
const EMPTY: Rgb = [0x8a, 0x96, 0xa0];
const IN_USE: Rgb = [0xff, 0xff, 0xff];
/// How much is left of the red, the green and the blue of the stadium
/// while the ball is being held back.
const COOL: [f32; 3] = [0.55, 0.72, 1.0];

/// The meter as it is drawn in the corner of the batting view.
pub(crate) struct Meter {
    words: Words,
    fill: Path,
}

impl Meter {
    /// Draws the meter with its words at `top`, the middle of their top
    /// edge, and the meter itself at `under`, the same of it.
    pub fn put(
        parts: &Parts,
        top: Point,
        under: Point,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Meter> {
        let holder = overlay::holder(parts, "bulletTime", stage, library)?;
        let words = Words::new(&holder, 1, "bulletWords", top, 0.8, stage, library)?;
        let left = under.0 - WIDE / 2.0;
        let mut bar = |depth: u16, name: &str, at: [f32; 4], colour: Rgb| {
            let path = stage.attach(&holder, art::BLOCK, depth, name, library)?;
            let block = stage.child_mut(&path)?;
            block.set_matrix(Matrix {
                a: at[2] / art::BLOCK_SIDE,
                d: at[3] / art::BLOCK_SIDE,
                tx: at[0],
                ty: at[1],
                ..Matrix::IDENTITY
            });
            block.set_color(look::tint(colour));
            Some(path)
        };
        let edge = [
            left - EDGE,
            under.1 - EDGE,
            WIDE + EDGE * 2.0,
            TALL + EDGE * 2.0,
        ];
        bar(3, "bulletEdge", edge, DARK)?;
        let fill = bar(4, "bulletMeter", [left, under.1, WIDE, TALL], READY)?;
        Some(Meter { words, fill })
    }

    /// Shows how much is left in the meter, from 0 to 1, and whether it is
    /// being used.
    pub fn keep(&self, left: f32, slowed: bool, stage: &mut Stage) {
        let colour = match (slowed, left > 0.0) {
            (true, _) => IN_USE,
            (false, true) => READY,
            (false, false) => EMPTY,
        };
        self.words.say(WORDS, colour, stage);
        if let Some(fill) = stage.child_mut(&self.fill) {
            let mut matrix = fill.matrix;
            matrix.a = WIDE * left.clamp(0.0, 1.0) / art::BLOCK_SIDE;
            fill.set_matrix(matrix);
            fill.set_color(look::tint(colour));
        }
    }
}

/// The light of the stadium as it is while the ball is being held back,
/// given what it would be otherwise.
pub fn cool(lighting: ColorTransform) -> ColorTransform {
    let mut cooled = lighting;
    for (channel, share) in cooled.mult.iter_mut().zip(COOL) {
        *channel *= share;
    }
    cooled
}

impl Match {
    /// Whether the frame in hand is one that bullet time holds the ball
    /// back for, `step` being the step of its flight the pitch has come
    /// to. Each frame the key is used on takes one off the meter, held
    /// back or not.
    pub(crate) fn held_back(
        &mut self,
        at_bat: &AtBat,
        step: usize,
        game: &Game,
        stage: &Stage,
    ) -> bool {
        let rules = &game.rules.bullet_time;
        let Some(left) = &mut self.bullet else {
            return false;
        };
        let near = step + rules.near as usize >= at_bat.pitch.samples.len();
        let wanted = game.mods.is_on(Mod::BulletTime) && stage.key_down(KEY);
        if !wanted || !near || at_bat.swing.is_some() || *left == 0 {
            return false;
        }
        *left -= 1;
        self.slowed = true;
        self.slow_beat += 1;
        !self.slow_beat.is_multiple_of(rules.slow.max(1))
    }

    /// A hit puts some of the meter back: this share of all it holds.
    pub(crate) fn refill_bullet_time(&mut self, share: f32, rules: &BulletTimeRules) {
        if let Some(left) = &mut self.bullet {
            let more = (rules.full as f32 * share).round() as u32;
            *left = (*left + more).min(rules.full);
        }
    }
}
