//! One side of a tournament, over every match it has played: how it has
//! done, its results, its batting and its figures against those of the
//! sides it played.

use super::Cells;
use super::a_match::batting_rows;
use crate::play::book::{Row, besides, hitting_rows, pitcher, pitching_rows};
use crate::tournament::Tournament;

/// A side in a line: its name, the matches it has won and lost, and its
/// runs against the runs against it.
pub fn heading(tournament: &Tournament, side: usize) -> String {
    let summed = tournament.summed(side);
    format!(
        "{}: WON {}, LOST {}, RUNS {} - {}",
        tournament.name_of(side),
        summed.won,
        summed.matches - summed.won,
        summed.own.runs,
        summed.against.runs
    )
}

/// What stands at the head of each column of a side's results.
pub const RESULT_HEADS: [&str; 4] = ["ROUND", "", "AGAINST", ""];

/// A side's fixtures in turn, under those heads: the round, whether it
/// was at home or away, who it played, and how it came out if it has been
/// played. A knockout tie it has not reached is not among them, and the
/// ones against the player's own side are marked.
pub fn results(tournament: &Tournament, side: usize) -> Vec<Cells> {
    let fixtures = tournament.fixtures();
    let its_own = fixtures.iter().filter(|fixture| fixture.has(side));
    its_own
        .map(|fixture| {
            let against = fixture.against(side);
            let at_home = fixture.home == Some(side);
            let came_out = match tournament.card(fixture.number).and_then(|card| {
                let (own, other) = (card.of_side(side)?, card.of_side(against?)?);
                Some((card.winner() == side, own.total(), other.total()))
            }) {
                Some((true, own, other)) => format!("WON {own} - {other}"),
                Some((false, own, other)) => format!("LOST {own} - {other}"),
                None => "TO PLAY".to_owned(),
            };
            Cells {
                cells: vec![
                    fixture.kind.words(),
                    if at_home { "AT HOME" } else { "AWAY" }.to_owned(),
                    against
                        .map_or("", |other| tournament.name_of(other))
                        .to_owned(),
                    came_out,
                ],
                ours: against == Some(tournament.player()),
            }
        })
        .collect()
}

/// A side's batting over every match it has played: the rows of the
/// table, and the two lines under it.
pub fn batting(tournament: &Tournament, side: usize) -> (Vec<[String; 11]>, [String; 2]) {
    let summed = tournament.summed(side);
    let own = &summed.own;
    let lines = [
        besides(&own.all, summed.against.errors),
        pitcher("PITCHED TO", own.outs, own.outs_an_innings, &own.all),
    ];
    (batting_rows(&own.places, &own.all), lines)
}

/// A side's figures beside those of the sides it has played, its own
/// first: the rows of the hitting, and of the pitches thrown.
pub fn figures(tournament: &Tournament, side: usize) -> (Vec<Row>, Vec<Row>) {
    let summed = tournament.summed(side);
    let (own, against) = (&summed.own.all, &summed.against.all);
    (hitting_rows(own, against), pitching_rows(own, against))
}
