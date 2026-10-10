//! The pages of the records, and of everything added up.

use bb_engine::stage::Stage;

use crate::board::{CREAM, MIDDLE, PALE};
use crate::sheet::Sheet;
use crate::tournament::Tournament;
use crate::tournament::stats::{records, totals};

/// How far down the first record is, how far apart they are, and the size
/// of their lettering.
const TOP: f32 = 94.0;
const PITCH: f32 = 20.5;
const SIZE: f32 = 0.6;
/// Where a record's name begins, the middle of what it stands at, and
/// where who set it begins.
const RECORD: [f32; 3] = [38.0, 276.0, 318.0];
/// The totals are in two columns: where the names of each begin, and how
/// far along from there the middle of how much there is.
const TOTALS: ([f32; 2], f32) = ([70.0, 320.0], 190.0);
const TOTAL_PITCH: f32 = 26.0;
const TOTAL_SIZE: f32 = 0.72;

/// Writes the records there are so far.
pub(super) fn records(tournament: &Tournament, sheet: &mut Sheet, stage: &mut Stage) {
    let records = records::records(tournament);
    if records.is_empty() {
        let none = "NO MATCH HAS BEEN PLAYED YET";
        sheet.write(
            stage,
            "recordsNone",
            none,
            (MIDDLE, TOP + PITCH * 3.0),
            0.75,
            PALE,
        );
    }
    let [what, stands_at, by] = RECORD;
    for (row, record) in records.iter().enumerate() {
        let down = TOP + PITCH * row as f32;
        sheet.write_left(stage, "recordWhat", record.what, (what, down), SIZE, PALE);
        let at = (stands_at, down);
        sheet.write(stage, "recordStands", &record.stands_at, at, SIZE, CREAM);
        sheet.write_left(stage, "recordBy", &record.by, (by, down), SIZE, CREAM);
    }
}

/// Writes how much there has been of everything in all.
pub(super) fn totals(tournament: &Tournament, sheet: &mut Sheet, stage: &mut Stage) {
    let totals = totals::totals(tournament);
    let ([first, second], along) = TOTALS;
    // Half of them down the left, and the rest down the right.
    let down_each = totals.len().div_ceil(2);
    for (index, (what, how_much)) in totals.iter().enumerate() {
        let left = if index < down_each { first } else { second };
        let down = TOP + 6.0 + TOTAL_PITCH * (index % down_each.max(1)) as f32;
        sheet.write_left(stage, "totalWhat", what, (left, down), TOTAL_SIZE, PALE);
        let at = (left + along, down);
        sheet.write(stage, "totalHas", how_much, at, TOTAL_SIZE, CREAM);
    }
}
