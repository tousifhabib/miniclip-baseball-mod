//! The page of a side's batting: each batter's line.

use bb_engine::stage::Stage;

use super::figures::innings_pitched;
use super::{CREAM, GOLD, MIDDLE, PALE};
use crate::play::book::{ORDER, Side, average, percent};
use crate::sheet::Sheet;

/// The page of what each batter of a side did. `fielding` is the side that
/// was in the field, whose errors and whose pitcher's figures these are
/// too. `whose` is whether the side batting is the player's, and how many
/// outs it had in an innings.
pub(super) fn batting(
    side: &Side,
    fielding: &Side,
    whose: (bool, u32),
    sheet: &mut Sheet<'_>,
    stage: &mut Stage,
) {
    let (ours, outs_an_innings) = whose;
    const ACROSS: [f32; 11] = [
        62.0, 108.0, 150.0, 192.0, 234.0, 276.0, 318.0, 364.0, 408.0, 450.0, 506.0,
    ];
    const HEADS: [&str; 11] = [
        "BAT", "AB", "R", "H", "2B", "3B", "HR", "RBI", "BB", "SO", "AVG",
    ];
    const SIZE: f32 = 0.7;
    const TOP: f32 = 90.0;
    const PITCH: f32 = 15.5;
    for (across, head) in ACROSS.into_iter().zip(HEADS) {
        sheet.write(stage, "battingHead", head, (across, TOP), SIZE, PALE);
    }
    let all = side.figures();
    let rows = (0..ORDER)
        .map(|order| ((order + 1).to_string(), side.figures_of(order)))
        .chain([("ALL".to_owned(), all)]);
    for (row, (who, figures)) in rows.enumerate() {
        // The row for the whole side stands a little apart.
        let last = row == ORDER;
        let down = TOP + 17.0 + PITCH * row as f32 + if last { 6.0 } else { 0.0 };
        let colour = if last { GOLD } else { CREAM };
        let cells = [
            who,
            figures.at_bats.to_string(),
            figures.runs.to_string(),
            figures.hits.to_string(),
            figures.doubles.to_string(),
            figures.triples.to_string(),
            figures.home_runs.to_string(),
            figures.runs_in.to_string(),
            figures.walks.to_string(),
            figures.strikeouts.to_string(),
            average(figures.average()),
        ];
        let name = if last { "battingAll" } else { "battingCell" };
        for (across, cell) in ACROSS.into_iter().zip(cells) {
            sheet.write(stage, name, &cell, (across, down), SIZE, colour);
        }
    }
    let left = format!(
        "LEFT ON BASE {}   DOUBLE PLAYS {}   SACRIFICE FLIES {}   TWO-OUT RUNS {}   ERRORS {}",
        all.left, all.double_plays, all.sacrifices, all.two_out_runs, fielding.errors
    );
    sheet.write(stage, "battingLine", &left, (MIDDLE, 276.0), 0.6, PALE);
    let whose = if ours { "THEIR" } else { "YOUR" };
    let pitcher = format!(
        "{whose} PITCHER: {} INNINGS, {} PITCHES, {} STRIKES, {} STRIKEOUTS, {} WALKS",
        innings_pitched(side.outs(), outs_an_innings),
        all.pitches,
        percent(all.strike_rate()),
        all.strikeouts,
        all.walks
    );
    sheet.write(stage, "pitcherLine", &pitcher, (MIDDLE, 292.0), 0.6, PALE);
}
