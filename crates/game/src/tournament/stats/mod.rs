//! What a tournament has to say for itself, set out in rows for a page to
//! write: its tables in `standing`, its rounds in `rounds`, one match in
//! `a_match` and one side in `a_side`.
//!
//! All of it is added up from the cards, by the sums a full match's own
//! pages are added up by, so that what is said of a side in one place
//! agrees with what is said of it in another.

pub mod a_match;
pub mod a_side;
pub mod rounds;
pub mod standing;

use super::{Card, SideCard, Tournament};
use crate::play::book::{Figures, ORDER};

/// A row of a table as it is to be written: what goes in each of its
/// columns, and whether it is the player's own side's, which a page marks
/// out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cells {
    pub cells: Vec<String>,
    pub ours: bool,
}

/// One side of several matches, added together.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Part {
    /// What each place in the order did, and the whole side.
    pub places: [Figures; ORDER],
    pub all: Figures,
    /// The runs made, which are the scores added up.
    pub runs: u32,
    /// The halves batted in, how many were put out in them, and how many
    /// outs there were to an innings.
    pub halves: u32,
    pub outs: u32,
    pub outs_an_innings: u32,
    /// The errors made in the field.
    pub errors: u32,
}

impl Part {
    fn add(&mut self, card: &SideCard) {
        for (all, more) in self.places.iter_mut().zip(card.places) {
            *all += more;
        }
        self.all += card.figures();
        self.runs += card.total();
        self.halves += card.runs.len() as u32;
        self.outs += card.outs;
        self.outs_an_innings = card.outs_an_innings;
        self.errors += card.errors;
    }
}

/// What a side has done over every match it has played, and what the
/// sides it played did against it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Summed {
    pub matches: u32,
    pub won: u32,
    pub own: Part,
    pub against: Part,
}

impl Tournament {
    /// What a side is called, by its place in the draw. A place there is
    /// no side at has no name.
    pub fn name_of(&self, side: usize) -> &str {
        self.sides.get(side).map_or("", |side| &side.name)
    }

    /// The same in the few letters a scoreboard has room for.
    pub fn short_of(&self, side: usize) -> &str {
        self.sides.get(side).map_or("", |side| &side.short)
    }

    /// The cards of the matches a side has played, in the order they were
    /// played: each with the side's own part of it, and the other side's.
    pub fn cards_of(&self, side: usize) -> impl Iterator<Item = (&Card, &SideCard, &SideCard)> {
        self.played.iter().filter_map(move |card| {
            if card.home.side == side {
                Some((card, &card.home, &card.away))
            } else if card.away.side == side {
                Some((card, &card.away, &card.home))
            } else {
                None
            }
        })
    }

    /// Everything a side has done, and had done against it, added up.
    pub fn summed(&self, side: usize) -> Summed {
        let mut summed = Summed::default();
        for (card, own, against) in self.cards_of(side) {
            summed.matches += 1;
            summed.won += u32::from(card.winner() == side);
            summed.own.add(own);
            summed.against.add(against);
        }
        summed
    }
}

#[cfg(test)]
mod tests;
