//! What the tournament's tests are built from.

use super::card::{Card, SideCard};
use crate::play::book::{Figures, ORDER};

/// The card of a fixture that came out so: each side by its place in the
/// draw, with the runs it made, all of them in one innings. Nothing else
/// is on it.
pub fn result(fixture: usize, home: (usize, u32), away: (usize, u32)) -> Card {
    let side = |(side, runs): (usize, u32)| SideCard {
        side,
        runs: vec![runs],
        places: [Figures::default(); ORDER],
        left: 0,
        errors: 0,
        outs: 3,
        outs_an_innings: 3,
    };
    Card {
        fixture,
        home: side(home),
        away: side(away),
        unneeded: false,
    }
}
