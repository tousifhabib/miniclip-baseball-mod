//! The page of a round: its fixtures, each with an arrow to open it by if
//! it has been played.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use crate::art;
use crate::board::{CREAM, GOLD, MIDDLE, PALE};
use crate::sheet::Sheet;
use crate::tournament::Tournament;
use crate::tournament::stats::rounds;

/// How far down the first fixture is, how far apart they are, and the size
/// of their lettering.
const TOP: f32 = 112.0;
const PITCH: f32 = 30.0;
const SIZE: f32 = 0.8;
/// Where the arrow that opens a fixture is from the left of the page and
/// from the top of its line, and its size, the art's own being 1.
const ARROW: (f32, f32, f32) = (48.0, 3.0, 0.8);
/// How far down the line is that says what the arrows are for.
const HINT: f32 = 286.0;

/// Writes the fixtures of a round. Returns the arrows that open the ones
/// that have been played, each with the number of its fixture.
pub(super) fn round(
    tournament: &Tournament,
    round: usize,
    sheet: &mut Sheet,
    stage: &mut Stage,
) -> Vec<(Path, usize)> {
    let mut opens = Vec::new();
    for (row, fixture) in rounds::round(tournament, round).iter().enumerate() {
        let down = TOP + PITCH * row as f32;
        let colour = if fixture.ours { GOLD } else { CREAM };
        let words = fixture.words();
        sheet.write(stage, "roundFixture", &words, (MIDDLE, down), SIZE, colour);
        if fixture.score.is_none() {
            continue;
        }
        let (across, below, size) = ARROW;
        let at = (across, down + below);
        let arrow = sheet.add(stage, art::BOARD_TURN, "openMatch", at, (size, size));
        if let Some(arrow) = arrow {
            opens.push((arrow, fixture.number));
        }
    }
    if !opens.is_empty() {
        let hint = "AN ARROW OPENS THE MATCH BESIDE IT";
        sheet.write(stage, "roundHint", hint, (MIDDLE, HINT), 0.6, PALE);
    }
    opens
}
