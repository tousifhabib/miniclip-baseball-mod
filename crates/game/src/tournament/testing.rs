//! What the tournament's tests are built from.

use super::card::{Card, SideCard};
use super::on_paper::{self, OnPaper, Paper};
use super::{Entrant, Format, Setup, Tournament};
use crate::look;
use crate::play::book::{Figures, ORDER};
use crate::play::field::Ground;
use crate::rules::Rules;
use crate::settings::Difficulty;

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

/// A tournament of this shape, drawn by the built-in rules, with matches
/// of this many innings on the middle skill level.
pub fn drawn(format: Format, innings: u32, seed: u64) -> Tournament {
    let setup = Setup {
        format,
        innings,
        skill: Difficulty::Medium,
    };
    let player = Entrant::the_players("", look::WHITE);
    Tournament::new(setup, player, &Rules::default().tournament, seed).expect("sides enough")
}

/// Plays a tournament to its end with nobody at the bat: the player's
/// fixtures are played on paper like any other. `seen` is shown the
/// tournament each time the player would have come to play.
pub fn played_out(
    mut tournament: Tournament,
    rules: &Rules,
    mut seen: impl FnMut(&Tournament),
) -> Tournament {
    let ground = Ground::default();
    let mods = OnPaper::default();
    while let Some(to_play) = tournament.to_play(rules) {
        seen(&tournament);
        let setup = tournament.setup();
        let by = Paper {
            rules,
            skill: setup.skill,
            innings: setup.innings,
            mods,
            ground: &ground,
        };
        let player = tournament.player();
        let (home, away) = if to_play.at_home {
            (player, to_play.against)
        } else {
            (to_play.against, player)
        };
        let strength = |side: usize| rules.tournament.strength_of(&tournament.sides()[side].key);
        let sides = ((home, strength(home)), (away, strength(away)));
        let card = on_paper::played(to_play.fixture, sides.0, sides.1, &by, to_play.seed);
        tournament.begin();
        tournament.take(card).expect("the card of the next fixture");
        tournament.play_on(rules, mods, &ground);
    }
    tournament
}
