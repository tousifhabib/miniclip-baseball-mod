//! The pages of the best: six lists to a page, three across and two down.

use bb_engine::stage::Stage;

use crate::board::{CREAM, GOLD, PALE, PANEL};
use crate::sheet::Sheet;
use crate::tournament::stats::leaders::List;

/// How many lists go across a page.
const ACROSS: usize = 3;
/// How far down the first row of lists is and how far below it the second,
/// how far apart the lines of a list are, and the size of its lettering.
const TOPS: (f32, f32) = (92.0, 108.0);
const PITCH: f32 = 15.5;
const SIZE: f32 = 0.6;
/// How far in from the left of its column a list's names begin, and from
/// the right of it the middle of what each has.
const INSET: (f32, f32) = (12.0, 30.0);

/// Writes the lists, each under what it is of, the best first.
pub(super) fn lists(lists: &[List], sheet: &mut Sheet, stage: &mut Stage) {
    let [left, _, wide, _] = PANEL;
    let wide = wide / ACROSS as f32;
    for (index, list) in lists.iter().enumerate() {
        let left = left + wide * (index % ACROSS) as f32;
        let top = TOPS.0 + TOPS.1 * (index / ACROSS) as f32;
        let middle = left + wide / 2.0;
        sheet.write(stage, "bestOf", list.of, (middle, top), 0.65, PALE);
        if list.best.is_empty() {
            let down = top + PITCH * 1.4;
            sheet.write(
                stage,
                "bestNobody",
                "NOBODY YET",
                (middle, down),
                SIZE,
                PALE,
            );
        }
        for (row, leader) in list.best.iter().enumerate() {
            let down = top + PITCH * (row as f32 + 1.4);
            // The player's own side, or one of its batters, stands out.
            let colour = if leader.ours { GOLD } else { CREAM };
            let who = (left + INSET.0, down);
            sheet.write_left(stage, "bestWho", &leader.who, who, SIZE, colour);
            let has = (left + wide - INSET.1, down);
            sheet.write(stage, "bestHas", &leader.has, has, SIZE, colour);
        }
    }
}
