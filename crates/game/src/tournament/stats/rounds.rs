//! The rounds of a tournament as they are listed: who meets whom in each,
//! and how it came out if it has been played.

use crate::tournament::format::Round;
use crate::tournament::schedule::{self, Slot};
use crate::tournament::stats::standing::letter;
use crate::tournament::{Fixture, Tournament};

/// One of the two sides of a fixture as it is listed: what it is called,
/// in full and in short, and which side it is once that is known. Until
/// then it is called by where it is to come from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    pub name: String,
    pub short: String,
    pub side: Option<usize>,
}

/// A fixture as it is listed: the side at home and the side away, their
/// runs if it has been played, and whether it is one of the player's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    pub number: usize,
    pub home: Named,
    pub away: Named,
    pub score: Option<(u32, u32)>,
    pub ours: bool,
}

impl Listed {
    /// The fixture in a line: the two sides with their runs between them,
    /// or a "v" if they have yet to play.
    pub fn words(&self) -> String {
        match self.score {
            Some((home, away)) => {
                format!("{} {home} - {away} {}", self.home.name, self.away.name)
            }
            None => format!("{} v {}", self.home.name, self.away.name),
        }
    }
}

/// What a side that is not known yet is called: by where it is to finish
/// in its group, or by the tie it is to win.
fn awaited(tournament: &Tournament, slot: Slot) -> String {
    let format = tournament.setup().format;
    match slot {
        Slot::Place(place) => tournament.name_of(place).to_owned(),
        Slot::Finisher { group, place } => {
            let place = if place == 0 { "WINNER" } else { "SECOND" };
            format!("{place} OF GROUP {}", letter(group))
        }
        Slot::WinnerOf(number) => {
            // Which tie of its round that one is, the first being 1.
            let ties = schedule::ties(format);
            let round = ties.get(number).map_or(0, |tie| tie.round);
            let before = ties[..number.min(ties.len())]
                .iter()
                .filter(|tie| tie.round == round);
            let kind = match format.round(round) {
                Round::QuarterFinals => "QUARTER-FINAL",
                Round::SemiFinals => "SEMI-FINAL",
                Round::Final | Round::Of(_) => "TIE",
            };
            format!("WINNER OF {kind} {}", before.count() + 1)
        }
    }
}

/// The fixtures of a round, the first being nought, in the order of their
/// numbers.
pub fn round(tournament: &Tournament, round: usize) -> Vec<Listed> {
    let named = |side: Option<usize>, from: Slot| match side {
        Some(side) => Named {
            name: tournament.name_of(side).to_owned(),
            short: tournament.short_of(side).to_owned(),
            side: Some(side),
        },
        // Not known yet, and called by where it is to come from.
        None => {
            let name = awaited(tournament, from);
            Named {
                short: name.clone(),
                name,
                side: None,
            }
        }
    };
    let listed = |fixture: &Fixture| {
        let card = tournament.card(fixture.number);
        Listed {
            number: fixture.number,
            home: named(fixture.home, fixture.home_from),
            away: named(fixture.away, fixture.away_from),
            score: card.map(|card| (card.home.total(), card.away.total())),
            ours: fixture.has(tournament.player()),
        }
    };
    let fixtures = tournament.fixtures();
    let of_the_round = fixtures.iter().filter(|fixture| fixture.round == round);
    of_the_round.map(listed).collect()
}
