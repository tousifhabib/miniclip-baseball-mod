//! The arrows that turn the pages of a board, and the words between them
//! that say which page is up.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::{CREAM, MIDDLE};
use crate::art;
use crate::play::overlay::Words;
use crate::sheet::Sheet;

/// The arrows and the words between them: how far down, how far either
/// side of the middle the arrows are, and the size of the words.
const PAGER_TOP: f32 = 317.0;
const PAGER_REACH: f32 = 62.0;
const PAGER_SIZE: f32 = 0.7;

/// The two arrows under a board's pages, and the count between them.
pub(super) struct Pager {
    back: Path,
    on: Path,
    count: Words,
}

impl Pager {
    /// Puts the arrows and the count in the clip at `holder`, which lies
    /// over the whole stage.
    pub fn put(holder: &[u16], stage: &mut Stage) -> Option<Pager> {
        let mut sheet = Sheet::on(holder.to_vec(), 500);
        // The art's arrow points on. The one back is the same, turned
        // round.
        let down = PAGER_TOP;
        let back = sheet.add(
            stage,
            art::BOARD_TURN,
            "pageBack",
            (MIDDLE - PAGER_REACH, down),
            (-1.0, 1.0),
        )?;
        let on = sheet.add(
            stage,
            art::BOARD_TURN,
            "pageOn",
            (MIDDLE + PAGER_REACH, down),
            (1.0, 1.0),
        )?;
        let depth = sheet.depth;
        let top = (MIDDLE, PAGER_TOP - 1.0);
        let count = Words::new(holder, depth, "pageCount", top, PAGER_SIZE, stage)?;
        Some(Pager { back, on, count })
    }

    /// The page a click on the button at `path` turns to from `page`, of
    /// `pages` counted from nought, if the button is one of the arrows.
    /// They go round: back from the first page is the last.
    pub fn turned(&self, path: &[u16], page: usize, pages: usize) -> Option<usize> {
        if pages == 0 {
            return None;
        }
        if self.back == path {
            Some((page + pages - 1) % pages)
        } else if self.on == path {
            Some((page + 1) % pages)
        } else {
            None
        }
    }

    /// Says which page is up, counting from 1, and how many there are.
    pub fn say(&self, page: usize, pages: usize, stage: &mut Stage) {
        let says = format!("PAGE {} OF {}", page + 1, pages);
        self.count.say(&says, CREAM, stage);
    }
}
