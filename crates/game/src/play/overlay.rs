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
    #[allow(clippy::too_many_arguments)]
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

/// A line of words put up in the view for a while, such as what a fielder
/// has just done, or for as long as the view lasts.
pub(crate) struct Notice {
    /// What the words are called on the stage, by which a notice is told
    /// from the others.
    name: String,
    /// The clip the words are in.
    holder: Path,
    /// Frames it has left. `None` stays until the view is built again.
    left: Option<u32>,
}

impl Notice {
    /// Puts `text` up with the middle of its top edge at `top`, taking the
    /// place of any notice of the same name. `size` is the size of the
    /// lettering, its own being 1.
    #[allow(clippy::too_many_arguments)]
    pub fn put(
        notices: &mut Vec<Notice>,
        parts: &Parts,
        name: &str,
        text: &str,
        top: Point,
        size: f32,
        colour: Rgb,
        frames: Option<u32>,
        stage: &mut Stage,
        library: &Library,
    ) {
        Notice::take_down(notices, name, stage);
        let Some(holder) = holder(parts, "notice", stage, library) else {
            return;
        };
        if let Some(words) = Words::new(&holder, 1, name, top, size, stage, library) {
            words.say(text, colour, stage);
        }
        notices.push(Notice {
            name: name.to_owned(),
            holder,
            left: frames,
        });
    }

    /// Takes the notice of this name down, if it is up.
    pub fn take_down(notices: &mut Vec<Notice>, name: &str, stage: &mut Stage) {
        notices.retain(|notice| {
            if notice.name == name {
                stage.remove(&notice.holder);
            }
            notice.name != name
        });
    }

    /// Counts a frame off the time of every notice that has one, and takes
    /// down those that have had theirs.
    pub fn fade(notices: &mut Vec<Notice>, stage: &mut Stage) {
        notices.retain_mut(|notice| match &mut notice.left {
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
