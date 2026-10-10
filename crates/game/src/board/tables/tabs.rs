//! The boxes along the top of the tables, which choose what there is to
//! read: a section for each.

use bb_engine::stage::Stage;

use crate::board::{CREAM, GOLD};
use crate::choice::{Choice, Row};
use crate::sheet::Sheet;

/// One of the sections the tables are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Section {
    /// How the sides stand, and the knockout rounds.
    Table,
    /// Every round's fixtures, and the matches that have been played.
    Matches,
    /// Every side, and what each has done.
    Sides,
    /// The best of the batters and of the sides.
    Leaders,
    /// The records, and everything added up.
    Records,
}

impl Section {
    const ALL: [Section; 5] = [
        Section::Table,
        Section::Matches,
        Section::Sides,
        Section::Leaders,
        Section::Records,
    ];

    /// The word beside its box.
    pub fn word(self) -> &'static str {
        match self {
            Section::Table => "TABLE",
            Section::Matches => "MATCHES",
            Section::Sides => "SIDES",
            Section::Leaders => "LEADERS",
            Section::Records => "RECORDS",
        }
    }
}

/// The row of boxes: where it is on the board, and what its things are
/// called.
const BOXES: Row = Row {
    first: (40.0, 56.0),
    pitch: 104.0,
    word: (19.0, -2.0),
    begins: true,
    size: 0.62,
    colour: CREAM,
    fill: GOLD,
    names: ["sectionWord", "section", "sectionFill"],
};

/// Puts a box for each section in the clip at `holder`.
pub(super) fn put(holder: &[u16], stage: &mut Stage) -> Choice<Section> {
    let mut sheet = Sheet::on(holder.to_vec(), 600);
    let all = Section::ALL.map(|section| (section, section.word()));
    Choice::put(&all, &BOXES, 600, &mut sheet, stage)
}
