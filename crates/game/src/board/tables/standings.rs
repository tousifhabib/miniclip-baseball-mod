//! The page of how the sides stand: a league's table, or both groups'.

use bb_engine::stage::Stage;

use crate::board::{CREAM, GOLD, PALE};
use crate::sheet::Sheet;
use crate::tournament::stats::standing::{self, HEADS};
use crate::tournament::{Format, Tournament};

/// Where the columns of a table are: the middle of each, but for the
/// second, a side's name, which is where it begins. Before the name is a
/// dot of the side's colour.
const ACROSS: [f32; 8] = [56.0, 88.0, 318.0, 360.0, 402.0, 448.0, 494.0, 538.0];
const DOT: f32 = 76.0;

/// How far down a table begins, the size of its lettering, and how far
/// apart its rows are: for the one table of a league, and for each of two
/// groups'.
const ONE: (f32, f32, f32) = (96.0, 0.8, 26.0);
const EACH_OF_TWO: (f32, f32, f32) = (90.0, 0.7, 18.5);
/// How far below one group's table the other's begins.
const BETWEEN: f32 = 108.0;

/// What the page is headed.
pub(super) fn heading(tournament: &Tournament) -> String {
    match tournament.setup().format {
        Format::Groups => "THE GROUPS".to_owned(),
        Format::League | Format::Cup => "THE TABLE".to_owned(),
    }
}

/// Writes every table the tournament has.
pub(super) fn standings(tournament: &Tournament, sheet: &mut Sheet, stage: &mut Stage) {
    let format = tournament.setup().format;
    let (top, size, pitch) = if format.groups() > 1 {
        EACH_OF_TWO
    } else {
        ONE
    };
    for group in 0..format.groups() {
        let top = top + BETWEEN * group as f32;
        // A group's table is called by its letter, where a side's name
        // would be. A league's is the only one there is.
        if format.groups() > 1 {
            let title = standing::title(format, group);
            sheet.write_left(stage, "tableTitle", &title, (ACROSS[1], top), size, PALE);
        }
        for (across, head) in ACROSS.into_iter().zip(HEADS) {
            sheet.write(stage, "tableHead", head, (across, top), size, PALE);
        }
        let table = tournament.table(group);
        let rows = standing::standing(tournament, group);
        for (row, (cells, stands)) in rows.iter().zip(&table).enumerate() {
            let down = top + pitch * (row + 1) as f32;
            let colour = if cells.ours { GOLD } else { CREAM };
            let name = if cells.ours { "tableOurs" } else { "tableCell" };
            let side = tournament.sides().get(stands.side);
            if let Some(side) = side {
                let middle = (DOT, down + 9.0 * size);
                sheet.dot(stage, "tableDot", middle, 1.6 * size, side.colour);
            }
            for (column, (across, cell)) in ACROSS.into_iter().zip(&cells.cells).enumerate() {
                if column == 1 {
                    sheet.write_left(stage, name, cell, (across, down), size, colour);
                } else {
                    sheet.write(stage, name, cell, (across, down), size, colour);
                }
            }
        }
    }
}
