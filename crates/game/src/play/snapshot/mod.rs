//! How a game stands, set down as plain facts and written out as one line.
//!
//! A script, a test or an inspector asks the game where it is and gets this.
//! The line it prints as is read by the tests, a word at a time, so the
//! words and their order are fixed here and nowhere else: what is said, in
//! what order, and to how many places.

mod words;

use std::fmt::{self, Formatter};

use super::Phase;
use super::pitch::{Kind, Point};

/// Where a game is and how it stands.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Snapshot {
    pub phase: Phase,
    pub standing: Standing,
    /// The pitch in hand, while there is a batting view.
    pub pitch: Option<PitchSeen>,
    pub mods: ModsSeen,
}

/// The score, in whichever kind of game it is.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Standing {
    /// The arcade game has points, and pitches still to come.
    Arcade { points: u32, pitches_left: u32 },
    Match {
        score: Score,
        outs: u32,
        balls: u32,
        strikes: u32,
        /// Whether a runner stands on first, second and third.
        bases: [bool; 3],
        pitched: u32,
        /// In a full match, what each side made in every innings so far,
        /// as the match itself tells it.
        innings: Option<String>,
    },
}

/// What the runs are counted against.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Score {
    /// The last innings has a number of runs to reach.
    Of { score: u32, target: u32 },
    /// A full match has the other side's runs, and says which half of
    /// which innings is being played.
    Against {
        batting_in: String,
        score: u32,
        theirs: u32,
    },
}

/// What a script needs to know of a pitch to time a swing at it, and what
/// has come of it so far.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PitchSeen {
    /// Where it crosses the plate.
    pub crosses: Point,
    /// How many frames it takes to get there.
    pub frames: usize,
    pub in_zone: bool,
    /// With the timing bar up, the first and last of the steps it shows as
    /// the best to swing on.
    pub best: Option<(usize, usize)>,
    /// How far the ball went, if it was hit for a zinger.
    pub zinger_feet: Option<u32>,
    pub mystery: Option<Kind>,
    pub golden: bool,
    pub rebounds: u32,
    /// Where on the outfield the shot was called.
    pub called: Option<Point>,
    /// Where the ball first came down.
    pub came_down: Option<Point>,
}

/// What the mods that keep a tally have to say for themselves. Each is as
/// it would be with its mod off unless the mod is on and has something to
/// tell.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ModsSeen {
    /// How often the fielders have let the ball go.
    pub let_go: u32,
    pub heat: u32,
    pub hits_in_a_row: u32,
    pub rally: u32,
    pub clutch: bool,
    pub southpaw: bool,
    /// What is left on the bullet-time meter, and whether the ball is
    /// being held back this frame.
    pub bullet_time: Option<(u32, bool)>,
    /// Which sign is lit, counting from 0.
    pub sign_lit: Option<usize>,
    /// The sign the ball struck, counting from 0, and the runs it paid.
    pub sign_struck: Option<(usize, u32)>,
    /// The base each runner who is stealing is going to.
    pub stealing: Vec<u8>,
    pub stolen: u32,
    pub caught: u32,
    pub arm: Option<ArmSeen>,
    /// How far the fielders have shifted: to the left below nought, to the
    /// right above it.
    pub shifted: f32,
}

/// How the pitcher's arm is holding up.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ArmSeen {
    /// Pitches this pitcher has thrown.
    pub thrown: u32,
    /// How tired he is, from 0 to 1.
    pub tired: f32,
    /// How many pitchers have been taken off before him.
    pub relieved: u32,
}

impl ModsSeen {
    fn bullet_time(&self, out: &mut Formatter<'_>) -> fmt::Result {
        if let Some((left, slowed)) = self.bullet_time {
            write!(out, ", bullet time {left}")?;
            if slowed {
                write!(out, " slowed")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
