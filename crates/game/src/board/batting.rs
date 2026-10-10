//! The page of a side's batting: each batter's line.

use bb_engine::stage::Stage;

use super::{CREAM, GOLD, MIDDLE, PALE};
use crate::play::book::{BATTING_HEADS, ORDER, Side, batting_cells, besides, pitcher};
use crate::sheet::Sheet;

/// The page of what each batter of a side did. `fielding` is the side that
/// was in the field, whose errors and whose pitcher's figures these are
/// too. `whose` is whether the side batting is the player's, and how many
/// outs it had in an innings.
pub(super) fn batting(
    side: &Side,
    fielding: &Side,
    whose: (bool, u32),
    sheet: &mut Sheet,
    stage: &mut Stage,
) {
    let (ours, outs_an_innings) = whose;
    let all = side.figures();
    let rows: Vec<[String; 11]> = (0..ORDER)
        .map(|order| batting_cells((order + 1).to_string(), &side.figures_of(order)))
        .chain([batting_cells("ALL".to_owned(), &all)])
        .collect();
    let whose = if ours { "THEIR" } else { "YOUR" };
    let lines = [
        besides(&all, fielding.errors),
        pitcher(whose, side.outs(), outs_an_innings, &all),
    ];
    table(&rows, &lines, sheet, stage);
}

/// Writes a table of batting: the heads, a row for each batter, the last
/// of which is the whole side's and stands a little apart, and the two
/// lines under them.
pub(super) fn table(
    rows: &[[String; 11]],
    lines: &[String; 2],
    sheet: &mut Sheet,
    stage: &mut Stage,
) {
    const ACROSS: [f32; 11] = [
        62.0, 108.0, 150.0, 192.0, 234.0, 276.0, 318.0, 364.0, 408.0, 450.0, 506.0,
    ];
    const SIZE: f32 = 0.7;
    const TOP: f32 = 90.0;
    const PITCH: f32 = 15.5;
    for (across, head) in ACROSS.into_iter().zip(BATTING_HEADS) {
        sheet.write(stage, "battingHead", head, (across, TOP), SIZE, PALE);
    }
    for (row, cells) in rows.iter().enumerate() {
        // The row for the whole side stands a little apart.
        let last = row + 1 == rows.len();
        let down = TOP + 17.0 + PITCH * row as f32 + if last { 6.0 } else { 0.0 };
        let colour = if last { GOLD } else { CREAM };
        let name = if last { "battingAll" } else { "battingCell" };
        for (across, cell) in ACROSS.into_iter().zip(cells) {
            sheet.write(stage, name, cell, (across, down), SIZE, colour);
        }
    }
    let [left, pitcher] = lines;
    sheet.write(stage, "battingLine", left, (MIDDLE, 276.0), 0.6, PALE);
    sheet.write(stage, "pitcherLine", pitcher, (MIDDLE, 292.0), 0.6, PALE);
}
