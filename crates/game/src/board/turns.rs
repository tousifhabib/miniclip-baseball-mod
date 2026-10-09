//! The pages that tell an innings a turn at a time.

use bb_engine::stage::Stage;

use super::{CREAM, GOLD, TURN_ROWS};
use crate::play::full::{FullMatch, hits_words, runs_words};
use crate::sheet::Sheet;

/// A page of an innings: the turns of the visitors' half down the left,
/// and of the home side's down the right.
pub(super) fn turns(
    full: &FullMatch,
    innings: u32,
    part: usize,
    sheet: &mut Sheet,
    stage: &mut Stage,
) {
    const SIZE: f32 = 0.62;
    const TOP: f32 = 90.0;
    const PITCH: f32 = 13.6;
    let home = full.at_home();
    // The visitors bat in the top of the innings.
    let halves = [(!home, "TOP", 38.0), (home, "BOTTOM", 302.0)];
    for (ours, half, left) in halves {
        let side = if ours {
            &full.book.ours
        } else {
            &full.book.theirs
        };
        let runs: u32 = side.innings(innings).map(|turn| turn.runs_in).sum();
        let hits = side.hits_in(innings);
        // Every turn, and every try at stealing a base, in the order they
        // came.
        let all = side.told(innings);
        let who = if ours { "YOU" } else { "THEM" };
        let head = if all.is_empty() {
            format!("{half}: {who}, NOT BATTED")
        } else {
            format!(
                "{half}: {who}, {} ON {}",
                runs_words(runs),
                hits_words(hits)
            )
        };
        let colour = if ours { GOLD } else { CREAM };
        sheet.write_left(stage, "turnsHead", &head, (left, TOP), 0.66, colour);
        let shown = all.iter().skip(part * TURN_ROWS).take(TURN_ROWS);
        for (row, (line, scored)) in shown.enumerate() {
            let down = TOP + 18.0 + PITCH * row as f32;
            // A turn that brought a run in stands out.
            let colour = if *scored { GOLD } else { CREAM };
            sheet.write_left(stage, "turnLine", line, (left, down), SIZE, colour);
        }
    }
}
