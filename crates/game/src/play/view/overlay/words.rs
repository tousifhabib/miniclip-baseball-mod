//! Words put on the stage in a field of the art's lettering.

use bb_engine::display::Path;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;
use bb_format::SymbolId;

use crate::art;
use crate::look::{self, Rgb};
use crate::play::pitch::Point;

/// The dark colour the mods edge and shadow their drawings with.
pub(crate) const DARK: Rgb = [0x04, 0x1a, 0x2b];

/// The lettering words are written in: which of the art's text fields, and
/// at what size, its own being 1.
#[derive(Clone, Copy)]
pub(crate) struct Lettering {
    pub field: SymbolId,
    pub size: f32,
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
    ) -> Option<Words> {
        let lettering = Lettering {
            field: art::TABLE_FIELD,
            size,
        };
        Words::in_field(lettering, holder, depth, name, top, stage)
    }

    /// The same in the lettering of another of the art's text fields, which
    /// has to be one that centres what it says.
    pub fn in_field(
        lettering: Lettering,
        holder: &[u16],
        depth: u16,
        name: &str,
        top: Point,
        stage: &mut Stage,
    ) -> Option<Words> {
        let Lettering {
            field: symbol,
            size,
        } = lettering;
        let field = stage.library().edit_texts.get(&symbol)?;
        // The field centres what it says, so it is placed by its middle.
        let middle = ((field.bounds.x_min + field.bounds.x_max) / 2.0) as f32;
        let drop = size.max(1.0);
        let mut paths = [Path::new(), Path::new()];
        for (index, offset) in [drop, 0.0].into_iter().enumerate() {
            let depth = depth + index as u16;
            let path = stage.attach(holder, symbol, depth, name)?;
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
    ) -> Option<Words> {
        let field = stage.library().edit_texts.get(&art::LABEL_FIELD)?;
        let (from, to) = (field.bounds.x_min as f32, field.bounds.x_max as f32);
        // The field is placed by its middle, and starts what it says from
        // its left edge.
        let top = (left.0 + (to - from) / 2.0 * size, left.1);
        let lettering = Lettering {
            field: art::LABEL_FIELD,
            size,
        };
        Words::in_field(lettering, holder, depth, name, top, stage)
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
