//! What the mods lay over the game's own views: a clip to keep their parts
//! in, and words that can be read against whatever is behind them.

mod notices;
mod words;

use bb_engine::display::{ClipState, Path};
use bb_engine::stage::Stage;

use super::Parts;
use crate::art;
use crate::play::pitch::Point;
pub(crate) use notices::{Notices, Says};
pub(crate) use words::{DARK, Lettering, Words};

/// Puts an empty clip into the view for a mod to draw in, and returns where
/// it is. It goes just over the field: over the batter, and under
/// everything the art lays over the game.
pub(crate) fn holder(parts: &Parts, name: &str, stage: &mut Stage) -> Option<Path> {
    let (&field, _) = parts.field.split_last()?;
    let view = stage.clip(&parts.main)?;
    let depth = free_above(view, field)?;
    stage.attach(&parts.main, art::HOLDER, depth, name)
}

/// The first free depth over `depth` in a clip: where a thing put there is
/// drawn just on top of what is at `depth`.
pub(crate) fn free_above(clip: &ClipState, depth: u16) -> Option<u16> {
    (depth.checked_add(1)?..=u16::MAX).find(|depth| !clip.children.contains_key(depth))
}

/// The first free depth under `depth` in a clip: where a thing put there is
/// drawn just beneath what is at `depth`.
pub(crate) fn free_below(clip: &ClipState, depth: u16) -> Option<u16> {
    (1..depth)
        .rev()
        .find(|depth| !clip.children.contains_key(depth))
}

/// Where a full match and the mods write in the corner of the batting view,
/// under the little field: the middle of the top of the first line, and how
/// far under each line the next one is. A full match says which half of
/// which innings it is, and each mod with something to say says it under
/// that.
const CORNER_AT: Point = (60.0, 88.0);
const CORNER_ROW: f32 = 16.0;
/// The lines written in the corner of the batting view, each under the last.
#[derive(Default)]
pub(crate) struct Corner {
    lines: u32,
}

impl Corner {
    /// Where the next line goes: the middle of the top of its words.
    pub fn line(&mut self) -> Point {
        let at = (CORNER_AT.0, CORNER_AT.1 + self.lines as f32 * CORNER_ROW);
        self.lines += 1;
        at
    }
}
