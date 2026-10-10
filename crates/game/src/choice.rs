//! A row of boxes to choose one of several by: each has its word beside
//! it, and the box of the one chosen is filled in.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use crate::art;
use crate::look::{self, Rgb};
use crate::sheet::Sheet;

/// What fills the box of the one chosen: how far into the box it sits, and
/// its size, the art's block being 1.
const FILL_IN: f32 = 2.0;
const FILL_SIZE: f32 = 0.58;

/// How a row of boxes is laid out, and what its things are called.
pub(crate) struct Row {
    /// The corner of the first box, and how far apart the boxes are.
    pub first: (f32, f32),
    pub pitch: f32,
    /// Where each box's word is from its box, whether that is where the
    /// word begins and not where its middle is, the word's size, the
    /// lettering's own being 1, and its colour.
    pub word: (f32, f32),
    pub begins: bool,
    pub size: f32,
    pub colour: Rgb,
    /// The colour the box of the one chosen is filled with.
    pub fill: Rgb,
    /// What the words, the boxes and what fills them are named on the
    /// stage.
    pub names: [&'static str; 3],
}

/// One of the things to choose: its box, and what fills the box when it is
/// the one chosen.
struct Boxed<T> {
    which: T,
    button: Path,
    fill: Path,
}

/// A row of boxes on the stage, one for each thing there is to choose.
pub(crate) struct Choice<T> {
    boxes: Vec<Boxed<T>>,
}

impl<T: Copy + PartialEq> Choice<T> {
    /// Puts a box and its word on the sheet for each of `all`. Each one's
    /// things have ten depths to themselves, from `depth` up.
    pub fn put(
        all: &[(T, &str)],
        row: &Row,
        depth: u16,
        sheet: &mut Sheet,
        stage: &mut Stage,
    ) -> Choice<T> {
        let [word, button, fill] = row.names;
        let mut boxes = Vec::new();
        for (index, &(which, says)) in all.iter().enumerate() {
            sheet.depth = depth + index as u16 * 10;
            let at = (row.first.0 + row.pitch * index as f32, row.first.1);
            let beside = (at.0 + row.word.0, at.1 + row.word.1);
            if row.begins {
                sheet.write_left(stage, word, says, beside, row.size, row.colour);
            } else {
                sheet.write(stage, word, says, beside, row.size, row.colour);
            }
            // The box goes on after its word, so that a click on the word
            // is a click on the box.
            let button = sheet.add(stage, art::CHOICE, button, at, (1.0, 1.0));
            let inside = (at.0 + FILL_IN, at.1 + FILL_IN);
            let size = (FILL_SIZE, FILL_SIZE);
            let fill = sheet.add(stage, art::BLOCK, fill, inside, size);
            if let (Some(button), Some(fill)) = (button, fill) {
                if let Some(fill) = stage.child_mut(&fill) {
                    fill.set_color(look::tint(row.fill));
                }
                boxes.push(Boxed {
                    which,
                    button,
                    fill,
                });
            }
        }
        Choice { boxes }
    }

    /// Keeps the box of the one chosen filled, and the others empty.
    pub fn show(&self, chosen: T, stage: &mut Stage) {
        for each in &self.boxes {
            if let Some(fill) = stage.child_mut(&each.fill) {
                fill.set_visible(each.which == chosen);
            }
        }
    }

    /// Which of them the button at `path` is the box of, if it is one of
    /// these.
    pub fn clicked(&self, path: &[u16]) -> Option<T> {
        self.boxes
            .iter()
            .find(|each| each.button == path)
            .map(|each| each.which)
    }
}
