//! The other side's innings, played out on paper.
//!
//! How many runs an innings of theirs comes to is settled first, by the
//! skill level. Then the innings is played here a pitch at a time, over and
//! over, until it comes to just that many, and that playing of it is the
//! one that goes in the book. Nothing is made up to fit afterwards, so
//! everything the book says of them adds up.

mod in_play;
mod pitches;
mod play;
#[cfg(test)]
mod properties;

use super::book::{End, Hit, ORDER, Pitch, Steal, Thrown, Turn};
use super::field::Ground;
use crate::rng::Rng;
use crate::rules::{StealRules, TheirBattingRules};
use pitches::Miss;
use play::Play;

/// A half of an innings as it was played on paper.
#[derive(Clone, Debug, PartialEq)]
pub struct Half {
    pub turns: Vec<Turn>,
    /// The tries at stealing a base there were in it. Each is told by how
    /// many of this half's turns were over when it was made.
    pub steals: Vec<Steal>,
    /// How many runners were left on base.
    pub left: u32,
    /// The runs each place in the order made in it.
    pub runs: [u32; ORDER],
    /// Whose turn it is next.
    pub next: usize,
}

/// How many times an innings is played before it is given up on, and how
/// many batters one playing of it may have.
const TRIES: u32 = 6000;
const MOST_TURNS: usize = 45;
/// How much likelier the hits are made, or rarer, after a playing that came
/// to too few runs or too many, and how far that can go.
const LEAN: f32 = 1.04;
const MOST_LEAN: f32 = 40.0;

/// The outs the other side has in an innings. On paper they play the game
/// by its own old rules, three outs, three strikes and four balls, whatever
/// the numbers the player's side is played by.
pub const OUTS: u32 = 3;

/// The half of an innings that is to be played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wanted {
    /// The runs it is to come to.
    pub made: u32,
    /// Whether those runs win the match, which then ends the moment the
    /// last of them is in.
    pub winning: bool,
    /// Which innings it is, the first being 1.
    pub innings: u32,
    /// Whose turn it is, by his place in the order.
    pub first_up: usize,
}

/// Plays the half of an innings that is wanted. With `steals` their
/// runners try for a base now and then, as those rules say.
pub fn half(
    wanted: Wanted,
    rules: &TheirBattingRules,
    steals: Option<&StealRules>,
    ground: &Ground,
    rng: &mut Rng,
) -> Half {
    // An innings of many runs is one in which the hits came easily, so
    // each playing that falls short makes them likelier for the next.
    let mut lean = 1.0_f32;
    for _ in 0..TRIES {
        let mut play = Play::new(wanted, lean, rules, ground);
        play.steals = steals;
        match play.out(rng) {
            Ok(()) => return play.half(),
            Err(Miss::TooFew) => lean = (lean * LEAN).min(MOST_LEAN),
            Err(Miss::TooMany) => lean = (lean / LEAN).max(1.0 / MOST_LEAN),
        }
    }
    plainly(wanted, ground)
}

/// An innings of just so many runs, for when none would come out that way
/// by itself: a home run for each, and then three strikeouts.
fn plainly(wanted: Wanted, ground: &Ground) -> Half {
    let Wanted {
        made,
        winning,
        innings,
        first_up,
    } = wanted;
    let mut half = Half {
        turns: Vec::new(),
        steals: Vec::new(),
        left: 0,
        runs: [0; ORDER],
        next: first_up,
    };
    let strike = |thrown| Pitch {
        in_zone: true,
        thrown,
        off: None,
        quality: None,
    };
    let turn = |half: &mut Half, end: End, outs: u32| {
        let order = half.next % ORDER;
        half.next = (order + 1) % ORDER;
        let home_run = end == End::HomeRun;
        if home_run {
            half.runs[order] += 1;
        }
        half.turns.push(Turn {
            innings,
            order,
            outs,
            on: [false; 3],
            pitches: if home_run {
                vec![strike(Thrown::InPlay)]
            } else {
                vec![
                    strike(Thrown::Called),
                    strike(Thrown::Swinging),
                    strike(Thrown::Swinging),
                ]
            },
            end,
            ball: home_run
                .then(|| Hit::at(ground, ground.point(0.5, ground.wall + 60.0), true, None)),
            runs_in: u32::from(home_run),
            outs_made: u32::from(!home_run),
        });
    };
    for _ in 0..made {
        turn(&mut half, End::HomeRun, 0);
    }
    if !winning {
        for outs in 0..OUTS {
            turn(&mut half, End::Strikeout, outs);
        }
    }
    half
}

#[cfg(test)]
mod tests;
