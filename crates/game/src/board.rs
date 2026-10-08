//! What a full match writes on the art's boards: between innings, what the
//! other side has just done and how both sides stand, and when the match is
//! over, what each made in every innings.

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::stage::Stage;

use crate::art;
use crate::look::Rgb;
use crate::play::full::{Cell, FullMatch};
use crate::play::overlay::Words;

/// The colours of the board's lettering: as the art has it, for the side
/// that is the player's, and for the numbers of the innings.
const CREAM: Rgb = [0xfd, 0xf6, 0xc0];
const GOLD: Rgb = [0xff, 0xd2, 0x4a];
const PALE: Rgb = [0xa9, 0xdc, 0xf0];

/// How wide the columns of the innings are, with the lettering at its own
/// size: the one for the sides' names, one for each innings, and the one
/// for the runs in all.
const NAME_WIDTH: f32 = 70.0;
const INNINGS_WIDTH: f32 = 30.0;
const RUNS_WIDTH: f32 = 50.0;

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
const RESULT_MIDDLE: f32 = 295.0;
const VERDICT_TOP: f32 = 123.0;
const RESULT_INNINGS: (f32, f32) = (243.0, 0.75);
const VERDICT_SIZE: f32 = 0.8;

/// One line of words in the board's lettering.
#[allow(clippy::too_many_arguments)]
fn write(
    holder: &[u16],
    depth: &mut u16,
    name: &str,
    text: &str,
    top: (f32, f32),
    size: f32,
    colour: Rgb,
    stage: &mut Stage,
    library: &Library,
) {
    if let Some(words) = Words::new(holder, *depth, name, top, size, stage, library) {
        words.say(text, colour, stage);
    }
    // Words are written twice, the second their shadow.
    *depth += 2;
}

/// Writes what both sides made in each innings: the numbers of the innings,
/// then a row for each side, with its runs in all at the end. `middle` is
/// the middle of the rows across and `top` the top of the first.
#[allow(clippy::too_many_arguments)]
fn innings(
    full: &FullMatch,
    holder: &[u16],
    depth: &mut u16,
    middle: f32,
    top: f32,
    size: f32,
    stage: &mut Stage,
    library: &Library,
) {
    let (first, count) = full.shown();
    let wide = NAME_WIDTH + INNINGS_WIDTH * count as f32 + RUNS_WIDTH;
    let left = middle - wide * size / 2.0;
    let name_at = left + NAME_WIDTH * size / 2.0;
    let innings_at =
        |column: u32| left + (NAME_WIDTH + INNINGS_WIDTH * (column as f32 + 0.5)) * size;
    let runs_at = left + (wide - RUNS_WIDTH / 2.0) * size;
    for column in 0..count {
        let number = (first + column).to_string();
        let at = (innings_at(column), top);
        write(
            holder,
            depth,
            "boardInnings",
            &number,
            at,
            size,
            PALE,
            stage,
            library,
        );
    }
    write(
        holder,
        depth,
        "boardInnings",
        "R",
        (runs_at, top),
        size,
        PALE,
        stage,
        library,
    );
    for (row, line) in full.lines().iter().enumerate() {
        let down = top + ROW_PITCH * size * (row + 1) as f32;
        let colour = if line.ours { GOLD } else { CREAM };
        write(
            holder,
            depth,
            "boardSide",
            line.name,
            (name_at, down),
            size,
            colour,
            stage,
            library,
        );
        for (column, cell) in line.cells.iter().enumerate() {
            let says = match cell {
                Cell::Blank => continue,
                Cell::Runs(runs) => runs.to_string(),
                Cell::NotNeeded => "X".to_owned(),
            };
            let at = (innings_at(column as u32), down);
            write(
                holder,
                depth,
                "boardCell",
                &says,
                at,
                size,
                colour,
                stage,
                library,
            );
        }
        let runs = line.runs.to_string();
        write(
            holder,
            depth,
            "boardRuns",
            &runs,
            (runs_at, down),
            size,
            colour,
            stage,
            library,
        );
    }
}

/// Writes what the board between innings says, on the art's board at
/// `board`. Returns the clip it is all in.
pub fn interval(
    full: &FullMatch,
    board: &[u16],
    stage: &mut Stage,
    library: &Library,
) -> Option<Path> {
    let at = Stage::RULES_DEPTH + 1;
    let holder = stage.attach(board, art::HOLDER, at, "intervalBoard", library)?;
    let report = full.report();
    let mut depth = 1;
    let (down, size) = HEADING;
    let heading = &report.heading;
    write(
        &holder,
        &mut depth,
        "boardHeading",
        heading,
        (0.0, down),
        size,
        CREAM,
        stage,
        library,
    );
    let mut lines = report.lines.iter();
    if let Some(line) = lines.next() {
        let top = (0.0, FIRST_LINE);
        write(
            &holder,
            &mut depth,
            "boardLine",
            line,
            top,
            1.0,
            CREAM,
            stage,
            library,
        );
    }
    let (down, size) = INNINGS;
    innings(full, &holder, &mut depth, 0.0, down, size, stage, library);
    for (index, line) in lines.enumerate() {
        let top = (0.0, LATER_LINES + LINE_PITCH * index as f32);
        // The last line of more than two is the one not to miss.
        let colour = if index >= 2 { GOLD } else { CREAM };
        write(
            &holder,
            &mut depth,
            "boardLine",
            line,
            top,
            1.0,
            colour,
            stage,
            library,
        );
    }
    Some(holder)
}

/// Writes a finished match on the board the game ends on, in the clip at
/// `holder`, which lies over the whole stage: who won, and every innings.
pub fn result(full: &FullMatch, holder: &[u16], stage: &mut Stage, library: &Library) {
    let mut depth = 100;
    let verdict = full.verdict();
    let top = (RESULT_MIDDLE, VERDICT_TOP);
    let size = VERDICT_SIZE;
    write(
        holder,
        &mut depth,
        "boardVerdict",
        &verdict,
        top,
        size,
        CREAM,
        stage,
        library,
    );
    let (down, size) = RESULT_INNINGS;
    innings(
        full,
        holder,
        &mut depth,
        RESULT_MIDDLE,
        down,
        size,
        stage,
        library,
    );
}
