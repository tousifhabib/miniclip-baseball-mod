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

use crate::art;
use crate::look::{self, Rgb};
use crate::mods::About;
use crate::play::Parts;
use crate::play::overlay::{self, DARK, Words};
use crate::play::pitch::Point;
use crate::rules::BulletTimeRules;

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "bullet_time",
    name: "BULLET TIME",
    does: "HOLD SPACE TO SLOW THE PITCH: HITS REFILL THE METER",
    setting: None,
};

/// The mod, in play: the meter, and how the ball is being held back.
pub(crate) struct BulletTime {
    rules: BulletTimeRules,
    /// How many frames of holding the ball back are left in the meter.
    /// `None` until the first view is got ready, when it is filled.
    meter: Option<u32>,
    /// How many frames the ball has been held back for, in all. It moves
    /// on one of them in every so many.
    beat: u32,
    /// Whether it was held back on the frame just played.
    slowed: bool,
    /// A click made on a frame the ball was held back on, which the next
    /// frame that moves it takes.
    late_press: Option<Point>,
}

impl BulletTime {
    pub fn new(rules: &BulletTimeRules) -> BulletTime {
        BulletTime {
            rules: rules.clone(),
            meter: None,
            beat: 0,
            slowed: false,
            late_press: None,
        }
    }

    /// Fills the meter, if this is the first view of the game.
    pub fn fill_at_the_start(&mut self) {
        self.meter.get_or_insert(self.rules.full);
    }

    /// What is left in the meter, once it has been filled.
    pub fn left(&self) -> Option<u32> {
        self.meter
    }

    /// The same as a share of all it holds, from 0 to 1.
    pub fn share_left(&self) -> Option<f32> {
        let full = self.rules.full.max(1) as f32;
        self.meter.map(|left| left as f32 / full)
    }

    pub fn slowed(&self) -> bool {
        self.slowed
    }

    /// Whether the ball was held back on the frame just played. It is asked
    /// once a frame, and forgotten.
    pub fn was_slowed(&mut self) -> bool {
        std::mem::take(&mut self.slowed)
    }

    /// The click that was kept from a frame the ball was held back on, if
    /// there was one.
    pub fn late_press(&mut self) -> Option<Point> {
        self.late_press.take()
    }

    /// Keeps a click made while the ball is held back for the frame that
    /// moves it.
    pub fn keep_press(&mut self, pressed: Option<Point>) {
        self.late_press = pressed;
    }

    /// Whether a pitch that has come to this step of its flight, of so
    /// many, is near enough the plate to be slowed.
    pub fn is_near(&self, step: usize, steps: usize) -> bool {
        step + self.rules.near as usize >= steps
    }

    /// Whether the frame in hand is one the ball is held back for. Each
    /// frame the key is used on takes one off the meter, held back or not:
    /// the ball still moves on one frame in every so many.
    pub fn holds_back(&mut self, near: bool, swung: bool, key_down: bool) -> bool {
        let Some(left) = &mut self.meter else {
            return false;
        };
        if !key_down || !near || swung || *left == 0 {
            return false;
        }
        *left -= 1;
        self.slowed = true;
        self.beat += 1;
        !self.beat.is_multiple_of(self.rules.slow.max(1))
    }

    /// A hit puts some of the meter back: all of it if the batter got home
    /// on it, and otherwise the share a hit is worth.
    pub fn refill(&mut self, got_home: bool) {
        let share = if got_home { 1.0 } else { self.rules.hit };
        if let Some(left) = &mut self.meter {
            let more = (self.rules.full as f32 * share).round() as u32;
            *left = (*left + more).min(self.rules.full);
        }
    }
}

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
