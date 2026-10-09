//! Which kind of game is being played. There are three, and a match is
//! always exactly one of them.

use super::arcade::Arcade;
use super::full::FullMatch;

/// What is being played, with what only that kind of game keeps.
pub(crate) enum Mode {
    /// The last innings of a match, batting to overtake the other side.
    LastInnings,
    /// A fixed number of pitches, and a target on the field to drop the
    /// ball on. It has points, and no runs, outs or fielders.
    Arcade(Arcade),
    /// Every innings of a match against another side.
    Full(Box<FullMatch>),
}

impl Mode {
    pub fn is_arcade(&self) -> bool {
        matches!(self, Mode::Arcade(_))
    }

    /// The arcade game's own state, if that is what is being played.
    pub fn arcade(&self) -> Option<&Arcade> {
        match self {
            Mode::Arcade(arcade) => Some(arcade),
            _ => None,
        }
    }

    pub fn arcade_mut(&mut self) -> Option<&mut Arcade> {
        match self {
            Mode::Arcade(arcade) => Some(arcade),
            _ => None,
        }
    }

    /// The full match, if that is what is being played.
    pub fn full(&self) -> Option<&FullMatch> {
        match self {
            Mode::Full(full) => Some(full),
            _ => None,
        }
    }

    pub fn full_mut(&mut self) -> Option<&mut FullMatch> {
        match self {
            Mode::Full(full) => Some(full),
            _ => None,
        }
    }
}
