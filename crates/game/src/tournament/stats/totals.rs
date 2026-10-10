//! A tournament added up: how much of everything there has been in all
//! its matches so far.

use crate::play::book::{Figures, tenths};
use crate::tournament::{Card, Tournament, schedule};

/// What there has been of each thing in all, in the order it is written:
/// what it is, and how much.
pub fn totals(tournament: &Tournament) -> Vec<(&'static str, String)> {
    let cards = tournament.cards();
    let all = schedule::ties(tournament.setup().format).len();
    let figures: Figures = cards
        .iter()
        .flat_map(|card| [card.home.figures(), card.away.figures()])
        .sum();
    let runs: u32 = cards
        .iter()
        .map(|card| card.home.total() + card.away.total())
        .sum();
    let count = |which: &dyn Fn(&&Card) -> bool| cards.iter().filter(which).count().to_string();
    let innings = tournament.setup().innings;
    let a_match = (!cards.is_empty()).then(|| runs as f32 / cards.len() as f32);
    vec![
        ("MATCHES PLAYED", format!("{} OF {all}", cards.len())),
        ("RUNS", runs.to_string()),
        ("RUNS A MATCH", tenths(a_match)),
        ("HITS", figures.hits.to_string()),
        ("HOME RUNS", figures.home_runs.to_string()),
        ("WALKS", figures.walks.to_string()),
        ("STRIKEOUTS", figures.strikeouts.to_string()),
        ("PITCHES", figures.pitches.to_string()),
        (
            "WON AT HOME",
            count(&|card| card.winner() == card.home.side),
        ),
        ("WON AWAY", count(&|card| card.winner() == card.away.side)),
        // The side at home won in a half it had need of, which is its
        // last.
        (
            "WON IN THE LAST HALF",
            count(&|card| card.winner() == card.home.side && !card.unneeded),
        ),
        (
            "WENT TO EXTRA INNINGS",
            count(&|card| card.innings() > innings),
        ),
        (
            "SHUT-OUTS",
            count(&|card| card.home.total().min(card.away.total()) == 0),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;
    use crate::tournament::Format;
    use crate::tournament::testing::{drawn, played_out};

    #[test]
    fn the_totals_are_of_every_match_there_has_been() {
        let fresh = drawn(Format::Cup, 3, 9);
        let before = totals(&fresh);
        assert_eq!(before[0], ("MATCHES PLAYED", "0 OF 7".to_owned()));
        assert_eq!(before[2], ("RUNS A MATCH", "-".to_owned()));
        let done = played_out(fresh, &Rules::default(), |_| {});
        let after = totals(&done);
        let of = |what: &str| -> u32 {
            let found = after.iter().find(|(name, _)| *name == what);
            found.expect("a total").1.parse().expect("a number")
        };
        assert_eq!(after[0].1, "7 OF 7");
        // Every match was won at home or away.
        assert_eq!(of("WON AT HOME") + of("WON AWAY"), 7);
        assert!(of("WON IN THE LAST HALF") <= of("WON AT HOME"));
        let runs: u32 = done
            .cards()
            .iter()
            .map(|card| card.home.total() + card.away.total())
            .sum();
        assert_eq!(of("RUNS"), runs);
        assert!(of("HITS") > 0 && of("PITCHES") > of("STRIKEOUTS"));
    }
}
