//! The pages of the sides: the list of them, each with an arrow to open
//! it by, and what a side that has been opened has to show: its results,
//! its batting and its figures.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use crate::art;
use crate::board::{CREAM, GOLD, PALE, batting, figures};
use crate::sheet::Sheet;
use crate::tournament::Tournament;
use crate::tournament::stats::a_side;

/// How far down the first line of a list is, how far apart its lines are,
/// and the size of its lettering.
const LIST: (f32, f32, f32) = (96.0, 26.0, 0.75);
/// Where things are across the list of sides: the arrow that opens a
/// side, the dot of its colour, where its name begins, and the middles of
/// what it has won, what it has lost and its runs.
const ARROW: f32 = 48.0;
const DOT: f32 = 76.0;
const NAME: f32 = 88.0;
const DONE: [f32; 3] = [340.0, 410.0, 494.0];
/// Where the columns of a side's results begin.
const RESULTS: [f32; 4] = [60.0, 196.0, 290.0, 452.0];

/// The colour a side's lines are written in: the player's own stands out.
fn colour(tournament: &Tournament, side: usize) -> [u8; 3] {
    if side == tournament.player() {
        GOLD
    } else {
        CREAM
    }
}

/// Writes the list of the sides, in the order they were drawn. Returns
/// the arrows that open them, each with its side.
pub(super) fn list(
    tournament: &Tournament,
    sheet: &mut Sheet,
    stage: &mut Stage,
) -> Vec<(Path, usize)> {
    let (top, pitch, size) = LIST;
    // Eight sides want their lines closer together than six.
    let pitch = pitch.min(204.0 / tournament.sides().len().max(1) as f32);
    for (across, head) in DONE.into_iter().zip(["WON", "LOST", "RUNS"]) {
        sheet.write(stage, "sidesHead", head, (across, top - 4.0), 0.65, PALE);
    }
    let mut opens = Vec::new();
    for (side, entrant) in tournament.sides().iter().enumerate() {
        let down = top + pitch * (side + 1) as f32 - 8.0;
        let colour = colour(tournament, side);
        let summed = tournament.summed(side);
        sheet.dot(
            stage,
            "sideDot",
            (DOT, down + 9.0 * size),
            1.6 * size,
            entrant.colour,
        );
        sheet.write_left(stage, "sideName", &entrant.name, (NAME, down), size, colour);
        let cells = [
            summed.won.to_string(),
            (summed.matches - summed.won).to_string(),
            format!("{} - {}", summed.own.runs, summed.against.runs),
        ];
        for (across, cell) in DONE.into_iter().zip(cells) {
            sheet.write(stage, "sideDone", &cell, (across, down), size, colour);
        }
        let at = (ARROW, down + 3.0);
        if let Some(arrow) = sheet.add(stage, art::BOARD_TURN, "openSide", at, (0.8, 0.8)) {
            opens.push((arrow, side));
        }
    }
    opens
}

/// What the first page of a side is headed: how it has done.
pub(super) fn heading(tournament: &Tournament, side: usize) -> String {
    a_side::heading(tournament, side)
}

/// What the page of a side's batting is headed.
pub(super) fn batting_heading(tournament: &Tournament, side: usize) -> String {
    format!("{}: BATTING IN EVERY MATCH", tournament.name_of(side))
}

/// What the page of a side's figures is headed.
pub(super) fn figures_heading(tournament: &Tournament, side: usize) -> String {
    format!("{} AND THE SIDES AGAINST IT", tournament.name_of(side))
}

/// The page of a side's fixtures in turn, and how each came out.
pub(super) fn results(tournament: &Tournament, side: usize, sheet: &mut Sheet, stage: &mut Stage) {
    let (top, pitch, size) = LIST;
    for (across, head) in RESULTS.into_iter().zip(a_side::RESULT_HEADS) {
        sheet.write_left(stage, "resultsHead", head, (across, top - 4.0), 0.65, PALE);
    }
    for (row, result) in a_side::results(tournament, side).iter().enumerate() {
        let down = top + pitch * (row + 1) as f32 - 8.0;
        // A match against the player's own side stands out.
        let colour = if result.ours { GOLD } else { CREAM };
        for (across, cell) in RESULTS.into_iter().zip(&result.cells) {
            sheet.write_left(stage, "resultsCell", cell, (across, down), size, colour);
        }
    }
}

/// The page of what each of a side's batters has done in every match it
/// has played.
pub(super) fn batting(tournament: &Tournament, side: usize, sheet: &mut Sheet, stage: &mut Stage) {
    let (rows, lines) = a_side::batting(tournament, side);
    batting::table(&rows, &lines, sheet, stage);
}

/// The page of a side's figures beside those of the sides it has played.
pub(super) fn figures(tournament: &Tournament, side: usize, sheet: &mut Sheet, stage: &mut Stage) {
    let sides = [
        (tournament.short_of(side), colour(tournament, side)),
        ("AGST", PALE),
    ];
    let (hitting, pitching) = a_side::figures(tournament, side);
    figures::table(sides, hitting, pitching, sheet, stage);
}
