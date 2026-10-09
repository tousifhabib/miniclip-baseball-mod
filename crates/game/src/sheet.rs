//! A clip that is being written and drawn on. The boards of a full match,
//! the menu's own pages and the list of mods are all put together this
//! way: words, things of the art's, blocks and dots of colour, each going
//! on over the last.

use bb_engine::display::Path;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;
use bb_format::SymbolId;

use crate::art;
use crate::look::{self, Rgb};
use crate::play::overlay::{Lettering, Words};

type Point = (f32, f32);

pub(crate) struct Sheet {
    pub holder: Path,
    /// The depth the next thing goes at. Each thing takes the next depth,
    /// and words with a shadow the next two, whether or not the art had
    /// what it takes to put them there.
    pub depth: u16,
    /// Which of the art's text fields its words are written in.
    lettering: SymbolId,
}

impl Sheet {
    /// A sheet on the clip at `holder`, whose first thing goes at `depth`.
    /// Its words are in the lettering of the score table.
    pub fn on(holder: Path, depth: u16) -> Sheet {
        Sheet {
            holder,
            depth,
            lettering: art::TABLE_FIELD,
        }
    }

    /// The same sheet with its words in the lettering of another of the
    /// art's text fields, which has to be one that centres what it says.
    pub fn lettered(self, field: SymbolId) -> Sheet {
        Sheet {
            lettering: field,
            ..self
        }
    }

    /// Where everything put on the sheet from `depth` on is, or would be.
    pub fn put_since(&self, depth: u16) -> Vec<Path> {
        (depth..self.depth)
            .map(|depth| {
                let mut path = self.holder.clone();
                path.push(depth);
                path
            })
            .collect()
    }

    /// A line of words with a shadow, centred on the middle of its top
    /// edge.
    pub fn write(
        &mut self,
        stage: &mut Stage,
        name: &str,
        text: &str,
        top: Point,
        size: f32,
        colour: Rgb,
    ) {
        let lettering = Lettering {
            field: self.lettering,
            size,
        };
        let (holder, depth) = (&self.holder, self.depth);
        let words = Words::in_field(lettering, holder, depth, name, top, stage);
        if let Some(words) = words {
            words.say(text, colour, stage);
        }
        // Words are written twice, the second their shadow.
        self.depth += 2;
    }

    /// A line of words with a shadow that starts from the left end of its
    /// top edge.
    pub fn write_left(
        &mut self,
        stage: &mut Stage,
        name: &str,
        text: &str,
        left: Point,
        size: f32,
        colour: Rgb,
    ) {
        let (holder, depth) = (&self.holder, self.depth);
        let words = Words::from_left(holder, depth, name, left, size, stage);
        if let Some(words) = words {
            words.say(text, colour, stage);
        }
        self.depth += 2;
    }

    /// A line of words with no shadow, centred on the middle of its top
    /// edge.
    pub fn write_plain(
        &mut self,
        stage: &mut Stage,
        name: &str,
        text: &str,
        top: Point,
        size: f32,
        colour: Rgb,
    ) -> Option<Path> {
        let field = stage.library().edit_texts.get(&self.lettering)?;
        // The field centres what it says, so it is placed by its middle.
        let middle = ((field.bounds.x_min + field.bounds.x_max) / 2.0) as f32;
        let at = (top.0 - middle * size, top.1);
        let path = self.add(stage, self.lettering, name, at, (size, size))?;
        Sheet::say(&path, text, colour, stage)?;
        Some(path)
    }

    /// A line of words with no shadow in the lettering of the art's
    /// labels, whose field is put at a point.
    pub fn label(
        &mut self,
        stage: &mut Stage,
        name: &str,
        text: &str,
        at: Point,
        size: f32,
        colour: Rgb,
    ) -> Option<Path> {
        let path = self.add(stage, art::LABEL_FIELD, name, at, (size, size))?;
        Sheet::say(&path, text, colour, stage)?;
        Some(path)
    }

    /// Gives one of the art's text fields its words and their colour.
    fn say(field: &[u16], text: &str, colour: Rgb, stage: &mut Stage) -> Option<()> {
        let words = stage.child_mut(field)?;
        words.said = Some(text.to_owned());
        words.set_color(look::tint(colour));
        Some(())
    }

    /// Something of the art's, at a point and a size across and down, its
    /// own being 1.
    pub fn add(
        &mut self,
        stage: &mut Stage,
        symbol: SymbolId,
        name: &str,
        at: Point,
        size: (f32, f32),
    ) -> Option<Path> {
        let depth = self.depth;
        self.depth += 1;
        let path = stage.attach(&self.holder, symbol, depth, name)?;
        stage.child_mut(&path)?.set_matrix(Matrix {
            a: size.0,
            d: size.1,
            tx: at.0,
            ty: at.1,
            ..Matrix::IDENTITY
        });
        Some(path)
    }

    /// A plain block of colour: its left, top, width and height.
    pub fn block(&mut self, stage: &mut Stage, name: &str, at: [f32; 4], colour: Rgb, alpha: f32) {
        let [left, top, wide, high] = at;
        let size = (wide / art::BLOCK_SIDE, high / art::BLOCK_SIDE);
        let path = self.add(stage, art::BLOCK, name, (left, top), size);
        if let Some(block) = path.and_then(|path| stage.child_mut(&path)) {
            block.set_color(look::tint(colour));
            block.set_alpha(alpha);
        }
    }

    /// A dot of colour, centred on a point.
    pub fn dot(&mut self, stage: &mut Stage, name: &str, at: Point, size: f32, colour: Rgb) {
        let path = self.add(stage, art::DOT, name, at, (size, size));
        if let Some(dot) = path.and_then(|path| stage.child_mut(&path)) {
            dot.set_color(look::tint(colour));
        }
    }
}
