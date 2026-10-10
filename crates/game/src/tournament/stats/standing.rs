//! A table as it is written: a row for each side, from the top.

use super::Cells;
use crate::tournament::{Format, Tournament};

/// What stands at the head of each column of a table: a side's place and
/// its name have nothing over them.
pub const HEADS: [&str; 8] = ["", "", "P", "W", "L", "FOR", "AGST", "DIFF"];

/// What a group's table is called. A league's is the only one it has.
pub fn title(format: Format, group: usize) -> String {
    match format {
        Format::Groups => format!("GROUP {}", letter(group)),
        Format::League | Format::Cup => "THE TABLE".to_owned(),
    }
}

/// The letter a group goes by: A for the first.
pub fn letter(group: usize) -> char {
    char::from(b'A' + (group % 26) as u8)
}

/// A number with its sign before it, as a difference is written.
fn signed(number: i64) -> String {
    if number > 0 {
        format!("+{number}")
    } else {
        number.to_string()
    }
}

/// The rows of a group's table under those heads, the side at the top
/// first.
pub fn standing(tournament: &Tournament, group: usize) -> Vec<Cells> {
    let table = tournament.table(group);
    table
        .iter()
        .enumerate()
        .map(|(place, row)| Cells {
            cells: vec![
                (place + 1).to_string(),
                tournament.name_of(row.side).to_owned(),
                row.played.to_string(),
                row.won.to_string(),
                row.lost.to_string(),
                row.runs_for.to_string(),
                row.runs_against.to_string(),
                signed(row.difference()),
            ],
            ours: row.side == tournament.player(),
        })
        .collect()
}
