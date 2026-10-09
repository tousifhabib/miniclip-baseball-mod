//! What a full match writes on the art's boards: between innings, what the
//! other side has just done and how both sides stand, and when the match is
//! over, pages of what the book has to say of it.
//!
//! The board between innings is here, with the colours and the measures
//! the pages share. The pages and the turning of them are in `pages`, and
//! each kind of page has a file: `batting`, `figures`, `spray`, `timing`
//! and `turns`.

mod batting;
mod figures;
mod pages;
mod spray;
mod timing;
mod turns;

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use crate::art;
use crate::look::{self, Rgb};
use crate::play::full::{Cell, FullMatch};
use crate::sheet::Sheet;
pub use pages::Pages;

/// The colours of the board's lettering: as the art has it, for the side
/// that is the player's, and for headings. The rest are for what is drawn.
const CREAM: Rgb = look::CREAM;
const GOLD: Rgb = [0xff, 0xd2, 0x4a];
const PALE: Rgb = [0xa9, 0xdc, 0xf0];
const WHITE: Rgb = look::WHITE;
const RED: Rgb = [0xff, 0x6e, 0x5c];
const GREEN: Rgb = [0x86, 0xf0, 0x8c];
const GREY: Rgb = [0xb4, 0xc2, 0xcc];
const BACKING: Rgb = [0x05, 0x1c, 0x30];

/// How wide the columns of the innings are, with the lettering at its own
/// size: the one for the sides' names, one for each innings, and one each
/// for the runs, hits and errors in all.
const NAME_WIDTH: f32 = 70.0;
const INNINGS_WIDTH: f32 = 30.0;
const ALL_WIDTH: f32 = 34.0;

/// Where things go on the board between innings, from its middle: how far
/// down the heading is and its size, the lettering's own being 1, then the
/// line that says what the other side made, the innings, and the lines
/// after them.
const HEADING: (f32, f32) = (-126.0, 1.6);
const FIRST_LINE: f32 = -82.0;
const INNINGS: (f32, f32) = (-44.0, 0.9);
const LATER_LINES: f32 = 40.0;
const LINE_PITCH: f32 = 25.0;
/// How far apart the rows of the innings are, the lettering at its own
/// size.
const ROW_PITCH: f32 = 24.0;

/// Where things go on the boards a match ends on, from the corner of the
/// stage: the middle across, how far down the line that says who won is,
/// how far down the innings are and their size.
const MIDDLE: f32 = 295.0;
const VERDICT_TOP: f32 = 123.0;
const RESULT_INNINGS: (f32, f32) = (243.0, 0.75);
const VERDICT_SIZE: f32 = 0.8;

/// How many turns a column of an innings' page has room for.
const TURN_ROWS: usize = 14;

/// Writes what both sides made in each innings: the numbers of the innings,
/// then a row for each side, with its runs, hits and errors in all at the
/// end. `middle` is the middle of the rows across and `top` the top of the
/// first.
fn innings(
    full: &FullMatch,
    sheet: &mut Sheet,
    middle: f32,
    top: f32,
    size: f32,
    stage: &mut Stage,
) {
    let (first, count) = full.shown();
    let wide = NAME_WIDTH + INNINGS_WIDTH * count as f32 + ALL_WIDTH * 3.0;
    let left = middle - wide * size / 2.0;
    let name_at = left + NAME_WIDTH * size / 2.0;
    let innings_at =
        |column: u32| left + (NAME_WIDTH + INNINGS_WIDTH * (column as f32 + 0.5)) * size;
    let all_at = |column: f32| left + (wide - ALL_WIDTH * (2.5 - column)) * size;
    for column in 0..count {
        let number = (first + column).to_string();
        let at = (innings_at(column), top);
        sheet.write(stage, "boardInnings", &number, at, size, PALE);
    }
    for (column, letter) in ["R", "H", "E"].into_iter().enumerate() {
        let at = (all_at(column as f32), top);
        sheet.write(stage, "boardInnings", letter, at, size, PALE);
    }
    for (row, line) in full.lines().iter().enumerate() {
        let down = top + ROW_PITCH * size * (row + 1) as f32;
        let colour = if line.ours { GOLD } else { CREAM };
        sheet.write(stage, "boardSide", line.name, (name_at, down), size, colour);
        for (column, cell) in line.cells.iter().enumerate() {
            let says = match cell {
                Cell::Blank => continue,
                Cell::Runs(runs) => runs.to_string(),
                Cell::NotNeeded => "X".to_owned(),
            };
            let at = (innings_at(column as u32), down);
            sheet.write(stage, "boardCell", &says, at, size, colour);
        }
        let all = [
            ("boardRuns", line.runs),
            ("boardHits", line.hits),
            ("boardErrors", line.errors),
        ];
        for (column, (name, number)) in all.into_iter().enumerate() {
            let at = (all_at(column as f32), down);
            sheet.write(stage, name, &number.to_string(), at, size, colour);
        }
    }
}

/// Writes what the board between innings says, on the art's board at
/// `board`. Returns the clip it is all in.
pub fn interval(full: &FullMatch, board: &[u16], stage: &mut Stage) -> Option<Path> {
    let at = Stage::RULES_DEPTH + 1;
    let holder = stage.attach(board, art::HOLDER, at, "intervalBoard")?;
    let mut sheet = Sheet::on(holder.clone(), 1);
    let report = full.report();
    let (down, size) = HEADING;
    sheet.write(
        stage,
        "boardHeading",
        &report.heading,
        (0.0, down),
        size,
        CREAM,
    );
    let mut lines = report.lines.iter();
    if let Some(line) = lines.next() {
        sheet.write(stage, "boardLine", line, (0.0, FIRST_LINE), 1.0, CREAM);
    }
    let (down, size) = INNINGS;
    innings(full, &mut sheet, 0.0, down, size, stage);
    for (index, line) in lines.enumerate() {
        let top = (0.0, LATER_LINES + LINE_PITCH * index as f32);
        // The last line of more than two is the one not to miss.
        let colour = if index >= 2 { GOLD } else { CREAM };
        sheet.write(stage, "boardLine", line, top, 1.0, colour);
    }
    Some(holder)
}

#[cfg(test)]
mod tests {}
