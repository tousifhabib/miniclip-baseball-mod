//! What the mods lay over the game's own views: a clip to keep their parts
//! in, and words that can be read against whatever is behind them.

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;
use bb_format::SymbolId;

use super::Parts;
use super::pitch::Point;
use crate::art;
use crate::look::{self, Rgb};

/// The dark colour the mods edge and shadow their drawings with.
pub(crate) const DARK: Rgb = [0x04, 0x1a, 0x2b];

/// Puts an empty clip into the view for a mod to draw in, and returns where
/// it is. It goes just over the field: over the batter, and under
/// everything the art lays over the game.
pub(crate) fn holder(
    parts: &Parts,
    name: &str,
    stage: &mut Stage,
    library: &Library,
) -> Option<Path> {
    let (&field, _) = parts.field.split_last()?;
    let view = stage.clip(&parts.main)?;
    let depth = (field + 1..).find(|depth| !view.children.contains_key(depth))?;
    stage.attach(&parts.main, art::HOLDER, depth, name, library)
}

/// A line of words in the game's display lettering. It is written twice,
/// with a dark copy a little down and to the right of the one that is read,
/// so that it stands out from the ground behind it.
pub(crate) struct Words([Path; 2]);

impl Words {
    /// Puts the words into `holder`, at `depth` and the depth after it,
    /// with the middle of their top edge at `top`. `size` is the size of
    /// the lettering, its own being 1. Nothing is seen until they are given
    /// something to say.
    pub fn new(
        holder: &[u16],
        depth: u16,
        name: &str,
        top: Point,
        size: f32,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Words> {
        Words::in_field(
            art::TABLE_FIELD,
            holder,
            depth,
            name,
            top,
            size,
            stage,
            library,
        )
    }

    /// The same in the lettering of another of the art's text fields, which
    /// has to be one that centres what it says.
    #[allow(
        clippy::too_many_arguments,
        reason = "it takes each thing it needs on its own, until they are gathered up"
    )]
    pub fn in_field(
        symbol: SymbolId,
        holder: &[u16],
        depth: u16,
        name: &str,
        top: Point,
        size: f32,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Words> {
        let field = library.edit_texts.get(&symbol)?;
        // The field centres what it says, so it is placed by its middle.
        let middle = ((field.bounds.x_min + field.bounds.x_max) / 2.0) as f32;
        let drop = size.max(1.0);
        let mut paths = [Path::new(), Path::new()];
        for (index, offset) in [drop, 0.0].into_iter().enumerate() {
            let depth = depth + index as u16;
            let path = stage.attach(holder, symbol, depth, name, library)?;
            let words = stage.child_mut(&path)?;
            words.set_matrix(Matrix {
                a: size,
                d: size,
                tx: top.0 - middle * size + offset,
                ty: top.1 + offset,
                ..Matrix::IDENTITY
            });
            words.set_visible(false);
            paths[index] = path;
        }
        Some(Words(paths))
    }

    /// Words in the table's lettering that start from a point instead of
    /// being centred on one: `left` is the left end of their top edge.
    pub fn from_left(
        holder: &[u16],
        depth: u16,
        name: &str,
        left: Point,
        size: f32,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Words> {
        let field = library.edit_texts.get(&art::LABEL_FIELD)?;
        let (from, to) = (field.bounds.x_min as f32, field.bounds.x_max as f32);
        // The field is placed by its middle, and starts what it says from
        // its left edge.
        let top = (left.0 + (to - from) / 2.0 * size, left.1);
        let field = art::LABEL_FIELD;
        Words::in_field(field, holder, depth, name, top, size, stage, library)
    }

    /// Takes the words out of sight.
    pub fn hide(&self, stage: &mut Stage) {
        for path in &self.0 {
            if let Some(words) = stage.child_mut(path) {
                words.set_visible(false);
            }
        }
    }

    /// Shows the words saying `text`, in `colour`.
    pub fn say(&self, text: &str, colour: Rgb, stage: &mut Stage) {
        for (path, colour) in self.0.iter().zip([DARK, colour]) {
            if let Some(words) = stage.child_mut(path) {
                words.said = Some(text.to_owned());
                words.set_color(look::tint(colour));
                words.set_visible(true);
            }
        }
    }
}

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

/// The size of the lettering a line that stays up is written in, the
/// lettering's own being 1.
const LINE_SIZE: f32 = 0.8;

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
    pub fn put(&mut self, says: Says<'_>, parts: &Parts, stage: &mut Stage, library: &Library) {
        self.take_down(says.name, stage);
        let Some(holder) = holder(parts, "notice", stage, library) else {
            return;
        };
        let (name, top, size) = (says.name, says.top, says.size);
        if let Some(words) = Words::new(&holder, 1, name, top, size, stage, library) {
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
