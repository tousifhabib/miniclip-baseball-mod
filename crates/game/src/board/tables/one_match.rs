//! The pages of a match that has been played, from its card: who won and
//! every innings, the two sides' figures, and each side's batting.

use bb_engine::stage::Stage;

use crate::board::{CREAM, GOLD, MIDDLE, batting, figures, innings};
use crate::sheet::Sheet;
use crate::tournament::stats::a_match;
use crate::tournament::{Card, Tournament};

/// How far down the line that says who won is and its size, and how far
/// down the innings are and theirs.
const VERDICT: (f32, f32) = (112.0, 0.85);
const INNINGS: (f32, f32) = (160.0, 0.9);

/// The card of a fixture, with the names of the side away and the side at
/// home, in short.
fn sides(tournament: &Tournament, fixture: usize) -> Option<(&Card, &str, &str)> {
    let card = tournament.card(fixture)?;
    let short = |side: usize| tournament.short_of(side);
    Some((card, short(card.away.side), short(card.home.side)))
}

/// What the pages of a match are headed: the visitors at the home side,
/// and the round.
pub(super) fn heading(tournament: &Tournament, fixture: usize) -> String {
    let Some(card) = tournament.card(fixture) else {
        return String::new();
    };
    let name = |side: usize| tournament.name_of(side);
    let fixtures = tournament.fixtures();
    let round = fixtures.get(fixture).map(|fixture| fixture.kind.words());
    format!(
        "{} AT {}, {}",
        name(card.away.side),
        name(card.home.side),
        round.unwrap_or_default()
    )
}

/// What the page of one side's batting in it is headed.
pub(super) fn batting_heading(tournament: &Tournament, fixture: usize, home: bool) -> String {
    let Some(card) = tournament.card(fixture) else {
        return String::new();
    };
    let (side, other) = if home {
        (&card.home, &card.away)
    } else {
        (&card.away, &card.home)
    };
    // A side that goes by "you" bats as "your".
    let whose = match tournament.name_of(side.side) {
        "YOU" => "YOUR",
        name => name,
    };
    format!("{whose} BATTING AGAINST {}", tournament.name_of(other.side))
}

/// The page that says who won, with every innings of both sides.
pub(super) fn score(tournament: &Tournament, fixture: usize, sheet: &mut Sheet, stage: &mut Stage) {
    let Some(card) = tournament.card(fixture) else {
        return;
    };
    let verdict = a_match::verdict(card, tournament);
    let (down, size) = VERDICT;
    sheet.write(stage, "matchVerdict", &verdict, (MIDDLE, down), size, CREAM);
    let (shown, lines) = a_match::line_score(card, tournament);
    let (down, size) = INNINGS;
    innings(shown, &lines, sheet, MIDDLE, down, size, stage);
}

/// The page of the two sides' figures, the visitors' first.
pub(super) fn figures(
    tournament: &Tournament,
    fixture: usize,
    sheet: &mut Sheet,
    stage: &mut Stage,
) {
    let Some((card, away, home)) = sides(tournament, fixture) else {
        return;
    };
    // The player's own side stands out, whichever of them it is.
    let colour = |side: usize| {
        if side == tournament.player() {
            GOLD
        } else {
            CREAM
        }
    };
    let sides = [
        (away, colour(card.away.side)),
        (home, colour(card.home.side)),
    ];
    let (hitting, pitching) = a_match::figures(card);
    figures::table(sides, hitting, pitching, sheet, stage);
}

/// The page of one side's batting.
pub(super) fn batting(
    tournament: &Tournament,
    fixture: usize,
    home: bool,
    sheet: &mut Sheet,
    stage: &mut Stage,
) {
    let Some(card) = tournament.card(fixture) else {
        return;
    };
    let (rows, lines) = a_match::batting(card, home, tournament);
    batting::table(&rows, &lines, sheet, stage);
}
