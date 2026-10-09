//! The notices the mods put up: a line in the corner of the view, or
//! news across it that fades.

use bb_engine::display::Path;
use bb_engine::stage::Stage;

use super::holder;
use super::words::Words;
use crate::look::Rgb;
use crate::play::pitch::Point;
use crate::play::view::Parts;

/// The size of the lettering a line that stays up is written in, the
/// lettering's own being 1.
const LINE_SIZE: f32 = 0.8;

/// What a notice is to say, and how it is to look.
#[derive(Clone, Copy)]
pub(crate) struct Says<'a> {
    /// What the words are called on the stage, by which a notice is told
    /// from the others.
    name: &'a str,
    words: &'a str,
    /// Where the middle of the words' top edge goes.
    top: Point,
    /// The size of the lettering, its own being 1.
    size: f32,
    colour: Rgb,
    /// Frames it stays up. `None` stays until the view is built again.
    frames: Option<u32>,
}

impl<'a> Says<'a> {
    /// A line that stays up for as long as the view lasts, in the small
    /// lettering the corner of the batting view is written in.
    pub fn line(name: &'a str, words: &'a str, colour: Rgb) -> Says<'a> {
        Says {
            name,
            words,
            top: (0.0, 0.0),
            size: LINE_SIZE,
            colour,
            frames: None,
        }
    }

    /// News that is put up for `frames` frames and then taken down.
    pub fn news(name: &'a str, words: &'a str, colour: Rgb, frames: u32) -> Says<'a> {
        Says {
            name,
            words,
            top: (0.0, 0.0),
            size: 1.0,
            colour,
            frames: Some(frames),
        }
    }

    /// With the middle of the words' top edge at `top`.
    pub fn at(self, top: Point) -> Says<'a> {
        Says { top, ..self }
    }

    /// In lettering of this size, its own being 1.
    pub fn sized(self, size: f32) -> Says<'a> {
        Says { size, ..self }
    }
}

/// A line of words put up in the view for a while, such as what a fielder
/// has just done, or for as long as the view lasts.
struct Notice {
    name: String,
    /// The clip the words are in.
    holder: Path,
    /// Frames it has left. `None` stays until the view is built again.
    left: Option<u32>,
}

/// The notices that are up in the view.
#[derive(Default)]
pub(crate) struct Notices(Vec<Notice>);

impl Notices {
    /// Puts a notice up, taking the place of any of the same name.
    pub fn put(&mut self, says: Says<'_>, parts: &Parts, stage: &mut Stage) {
        self.take_down(says.name, stage);
        let Some(holder) = holder(parts, "notice", stage) else {
            return;
        };
        let (name, top, size) = (says.name, says.top, says.size);
        if let Some(words) = Words::new(&holder, 1, name, top, size, stage) {
            words.say(says.words, says.colour, stage);
        }
        self.0.push(Notice {
            name: name.to_owned(),
            holder,
            left: says.frames,
        });
    }

    /// Takes the notice of this name down, if it is up.
    pub fn take_down(&mut self, name: &str, stage: &mut Stage) {
        self.0.retain(|notice| {
            if notice.name == name {
                stage.remove(&notice.holder);
            }
            notice.name != name
        });
    }

    /// Counts a frame off the time of every notice that has one, and takes
    /// down those that have had theirs.
    pub fn fade(&mut self, stage: &mut Stage) {
        self.0.retain_mut(|notice| match &mut notice.left {
            Some(0) => {
                stage.remove(&notice.holder);
                false
            }
            Some(left) => {
                *left -= 1;
                true
            }
            None => true,
        });
    }
}
