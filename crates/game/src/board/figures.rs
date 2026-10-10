//! The page of figures: two sides' hitting and pitching, row by row.

use bb_engine::stage::Stage;

use super::{CREAM, GOLD, PALE};
use crate::look::Rgb;
use crate::play::book::{Row, Side, hitting_rows, pitching_rows};
use crate::sheet::Sheet;

/// The page of the two sides' figures, side by side. `called` is what the
/// other side goes by.
pub(super) fn figures(
    ours: &Side,
    theirs: &Side,
    called: &str,
    sheet: &mut Sheet,
    stage: &mut Stage,
) {
    let (us, them) = (ours.figures(), theirs.figures());
    let sides = [("YOU", GOLD), (called, CREAM)];
    let (hitting, pitching) = (hitting_rows(&us, &them), pitching_rows(&us, &them));
    table(sides, hitting, pitching, sheet, stage);
}

/// Writes a page of figures: the hitting down the left and the pitching
/// down the right, each with a column for either of the `sides`, which are
/// what each is called and the colour it is written in.
pub(super) fn table(
    sides: [(&str, Rgb); 2],
    hitting: Vec<Row>,
    pitching: Vec<Row>,
    sheet: &mut Sheet,
    stage: &mut Stage,
) {
    const SIZE: f32 = 0.68;
    const TOP: f32 = 90.0;
    const PITCH: f32 = 16.2;
    let [(_, first), (_, second)] = sides;
    // Each half of the page: where its words begin, and the middles of the
    // two sides' columns.
    for (rows, left, columns) in [
        (hitting, 38.0, [208.0, 258.0]),
        (pitching, 300.0, [478.0, 530.0]),
    ] {
        for (across, (side, colour)) in columns.into_iter().zip(sides) {
            sheet.write(stage, "figuresHead", side, (across, TOP), SIZE, colour);
        }
        for (row, (name, one, other)) in rows.into_iter().enumerate() {
            let down = TOP + PITCH * (row + 1) as f32;
            sheet.write_left(stage, "figuresName", name, (left, down), SIZE, PALE);
            sheet.write(stage, "figuresOurs", &one, (columns[0], down), SIZE, first);
            sheet.write(
                stage,
                "figuresTheirs",
                &other,
                (columns[1], down),
                SIZE,
                second,
            );
        }
    }
}
