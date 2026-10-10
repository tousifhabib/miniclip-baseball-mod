//! A tournament: full matches, one after another, against sides with
//! names, until one of them has won it.
//!
//! Everything here is sums. None of it touches what the game is drawn on,
//! so a tournament can be drawn, played out and added up with no game
//! running at all.
//!
//! The shapes a tournament comes in are in `format`, its sides and the
//! drawing of them in `sides`, who is to meet whom in `schedule`, what
//! each thing that draws numbers draws them from in `seeds`, and what a
//! side's strength does to its runs in `strength`.

pub mod format;
pub mod schedule;
pub mod seeds;
pub mod sides;
pub mod strength;

pub use format::{Format, Round};
pub use sides::Entrant;
