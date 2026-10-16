//! The page of the knockout rounds: each round's ties in a column, the
//! first round on the left and the final on the right.

use bb_engine::stage::Stage;

use crate::board::{CREAM, GOLD, GREY, PALE, PANEL};
use crate::sheet::Sheet;
use crate::tournament::Tournament;
use crate::tournament::stats::rounds::{self, Listed, Named};

/// How far down the names of the rounds are, how far down the ties begin
/// and how much room they have, and how far apart the two sides of a tie
/// are.
const ROUNDS: f32 = 92.0;
const TIES: (f32, f32) = (114.0, 192.0);
const SIDES: f32 = 15.0;
/// How far in from the edges of its column a tie's names begin and its
/// runs are.
const INSET: (f32, f32) = (12.0, 22.0);

/// Writes every knockout round there is.
pub(super) fn knockout(tournament: &Tournament, sheet: &mut Sheet, stage: &mut Stage) {
    let format = tournament.setup().format;
    let rounds: Vec<usize> = (0..format.rounds())
        .filter(|&round| format.round(round).is_knockout())
        .collect();
    let [left, _, wide, _] = PANEL;
    let wide = wide / rounds.len().max(1) as f32;
    // Three columns have less room for a name than two.
    let size = if rounds.len() > 2 { 0.55 } else { 0.68 };
    for (column, &round) in rounds.iter().enumerate() {
        let left = left + wide * column as f32;
        let name = format.round(round).words();
        let middle = left + wide / 2.0;
        sheet.write(stage, "knockoutRound", &name, (middle, ROUNDS), 0.7, PALE);
        let ties = rounds::round(tournament, round);
        // The ties of a round are spread down its column, so that each
        // lies between the two it follows from.
        let (top, room) = TIES;
        let each = room / ties.len().max(1) as f32;
        for (row, tie) in ties.iter().enumerate() {
            let down = top + each * row as f32 + (each - SIDES * 2.0) / 2.0;
            let at = (left + INSET.0, left + wide - INSET.1, down);
            one_tie(tournament, tie, at, size, sheet, stage);
        }
    }
}

/// Writes a tie: the side at home over the side away, each with its runs
/// if they have played. `at` is where the names begin, where the runs are,
/// and how far down the first of them is.
fn one_tie(
    tournament: &Tournament,
    tie: &Listed,
    at: (f32, f32, f32),
    size: f32,
    sheet: &mut Sheet,
    stage: &mut Stage,
) {
    let (names, runs, down) = at;
    let (home, away) = tie.score.unzip();
    let sides: [(&Named, Option<u32>, Option<u32>); 2] =
        [(&tie.home, home, away), (&tie.away, away, home)];
    for (line, (side, made, against)) in sides.into_iter().enumerate() {
        let down = down + SIDES * line as f32;
        // The player's own side stands out, and a side that has lost
        // stands back.
        let colour = if side.side == Some(tournament.player()) {
            GOLD
        } else if made < against {
            GREY
        } else {
            CREAM
        };
        sheet.write_left(
            stage,
            "knockoutSide",
            &side.name,
            (names, down),
            size,
            colour,
        );
        if let Some(made) = made {
            let made = made.to_string();
            sheet.write(stage, "knockoutRuns", &made, (runs, down), size, colour);
        }
    }
}
