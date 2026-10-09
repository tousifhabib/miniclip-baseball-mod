//! Who plays the games that are written down: a batter who reads the pitch
//! and swings, a monkey who reads nothing, and written steps for the menu.
//!
//! Each plays one frame at a time, so that every frame can be taken in.
//! Their chances come from dice of their own, not the game's, so nothing
//! done to the game's own generator can change what they do.

mod batter;
mod monkey;
mod written;

use bb_engine::display::{Children, Content, child_bounds};
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_format::SymbolId;
use bb_game::art::ButtonLabels;
use bb_game::script::Script;

pub use batter::{Batter, Tricks};
pub use monkey::Monkey;
pub use written::{Then, Written};

/// The stage's size, in its own pixels.
const STAGE: (u32, u32) = (590, 400);

pub trait Player {
    /// Plays one frame. Returns false, having played nothing, once there
    /// is nothing left to play.
    fn frame(&mut self, script: &mut Script) -> bool;
}

/// Numbers by chance that are the same every time for the same seed.
pub struct Dice(u64);

impl Dice {
    pub fn new(seed: u64) -> Dice {
        Dice(seed)
    }

    fn roll(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^ (mixed >> 31)
    }

    /// A whole number from nought up to but not including `count`.
    pub fn below(&mut self, count: u32) -> u32 {
        (((self.roll() >> 32) * u64::from(count)) >> 32) as u32
    }

    /// True this many times in a hundred.
    pub fn chance(&mut self, in_a_hundred: u32) -> bool {
        self.below(100) < in_a_hundred
    }

    pub fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[self.below(of.len() as u32) as usize]
    }
}

fn run(script: &mut Script, steps: &str) {
    script.run(steps).expect("the steps to run");
}

/// Every button that can be seen on the stage: which it is, and its middle.
pub fn buttons(script: &Script) -> Vec<(SymbolId, f32, f32)> {
    fn look(
        children: &Children,
        matrix: Matrix,
        library: &Library,
        found: &mut Vec<(SymbolId, f32, f32)>,
    ) {
        for child in children.values() {
            if !child.visible || child.clip_depth.is_some() {
                continue;
            }
            match &child.content {
                Content::Button(_) => {
                    if let Some([left, top, right, bottom]) = child_bounds(child, matrix, library) {
                        found.push((child.symbol, (left + right) / 2.0, (top + bottom) / 2.0));
                    }
                }
                Content::Clip(clip) => look(
                    &clip.children,
                    matrix.then_inner(child.matrix),
                    library,
                    found,
                ),
                Content::Graphic => {}
            }
        }
    }
    let mut found = Vec::new();
    let runner = &script.runner;
    look(
        &runner.stage.root.children,
        Matrix::IDENTITY,
        &runner.library,
        &mut found,
    );
    found.retain(|&(_, x, y)| {
        (0.0..STAGE.0 as f32).contains(&x) && (0.0..STAGE.1 as f32).contains(&y)
    });
    found
}

/// The words on the button that gives a game up.
const QUIT: &str = "QUIT";

/// Picks out buttons to click, by chance, and knows what each one says.
struct Hand {
    dice: Dice,
    labels: Option<ButtonLabels>,
}

impl Hand {
    fn new(seed: u64) -> Hand {
        Hand {
            dice: Dice::new(seed),
            labels: None,
        }
    }

    /// Clicks a button that can be seen, any of them, or does `otherwise`
    /// if there is none. The button that gives the game up is passed over
    /// unless `may_quit`: a game given up at once shows very little.
    fn click_a_button(&mut self, script: &mut Script, otherwise: &str, may_quit: bool) {
        let labels = self
            .labels
            .get_or_insert_with(|| ButtonLabels::read(&script.runner.library));
        let mut seen = buttons(script);
        if !may_quit {
            seen.retain(|&(button, _, _)| labels.get(button) != Some(QUIT));
        }
        if seen.is_empty() {
            run(script, otherwise);
        } else {
            let (_, x, y) = self.dice.pick(&seen);
            run(script, &format!("click {x} {y}"));
        }
    }
}
