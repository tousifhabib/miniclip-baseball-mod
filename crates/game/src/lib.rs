//! The baseball game: the engine playing the extracted art, with the rules
//! in `baseball`.

// Outside the tests nothing is taken for granted: what may be missing is
// dealt with, or the reason it cannot be is given.
#![warn(clippy::unwrap_used)]

pub mod art;
pub mod baseball;
pub mod board;
mod choice;
pub mod game;
pub mod kept;
pub mod locate;
pub mod look;
pub mod menu;
pub mod mods;
pub mod play;
pub mod rng;
pub mod rules;
pub mod scores;
pub mod script;
pub mod settings;
mod sheet;
pub mod tournament;
