//! One match of a tournament, from its card: who won, the innings of both
//! sides, their batting and their figures.

use crate::play::book::{
    Figures, ORDER, Row, batting_cells, besides, hitting_rows, pitcher, pitching_rows,
};
use crate::play::full::{Line, cells_of, shown_of};
use crate::tournament::{Card, SideCard, Tournament};

/// Who beat whom, by what, and in how many innings if it took more than
/// it had to.
pub fn verdict(card: &Card, tournament: &Tournament) -> String {
    let (won, lost) = if card.winner() == card.home.side {
        (&card.home, &card.away)
    } else {
        (&card.away, &card.home)
    };
    let more = if card.innings() > tournament.setup().innings {
        format!(" IN {} INNINGS", card.innings())
    } else {
        String::new()
    };
    format!(
        "{} BEAT {} {} - {}{more}",
        tournament.name_of(won.side),
        tournament.name_of(lost.side),
        won.total(),
        lost.total()
    )
}

/// The innings of both sides as a board has them: the first innings there
/// is room to show and how many, and the two sides' lines, the visitors'
/// first.
pub fn line_score(card: &Card, tournament: &Tournament) -> ((u32, u32), [Line; 2]) {
    let shown = shown_of(card.innings(), tournament.setup().innings);
    let line = |side: &SideCard, unneeded: bool| Line {
        name: tournament.short_of(side.side).to_owned(),
        ours: side.side == tournament.player(),
        cells: cells_of(&side.runs, shown, unneeded),
        runs: side.total(),
        hits: side.figures().hits,
        errors: side.errors,
    };
    (
        shown,
        [line(&card.away, false), line(&card.home, card.unneeded)],
    )
}

/// The two sides' figures, the visitors' first: the rows of their hitting,
/// and of the pitches they were thrown.
pub fn figures(card: &Card) -> (Vec<Row>, Vec<Row>) {
    let (away, home) = (card.away.figures(), card.home.figures());
    (hitting_rows(&away, &home), pitching_rows(&away, &home))
}

/// The rows of a table of batting: one for each place in the order, and
/// one for all of them.
pub fn batting_rows(places: &[Figures; ORDER], all: &Figures) -> Vec<[String; 11]> {
    (0..ORDER)
        .map(|order| batting_cells((order + 1).to_string(), &places[order]))
        .chain([batting_cells("ALL".to_owned(), all)])
        .collect()
}

/// One side's batting in the match: the rows of the table, and the two
/// lines under it, which tell of the errors and the pitching of the side
/// that was in the field.
pub fn batting(
    card: &Card,
    home: bool,
    tournament: &Tournament,
) -> (Vec<[String; 11]>, [String; 2]) {
    let (side, fielding) = if home {
        (&card.home, &card.away)
    } else {
        (&card.away, &card.home)
    };
    let all = side.figures();
    let who = format!("{} PITCHER", tournament.short_of(fielding.side));
    let lines = [
        besides(&all, fielding.errors),
        pitcher(&who, side.outs, side.outs_an_innings, &all),
    ];
    (batting_rows(&side.places, &all), lines)
}
