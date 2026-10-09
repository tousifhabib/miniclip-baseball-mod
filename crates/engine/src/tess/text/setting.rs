//! Setting the words of a text field: which glyph each letter is, and
//! where on its line each one goes.

use anyhow::Result;
use bb_format as f;

use super::GUTTER;
use crate::display::CARET;
use crate::math::Matrix;
use crate::tess::outline::{Segment, lyon_path, parse_path};
use crate::tess::{Builder, Paint};

/// How a text field's lines are set: in what font, how big and what colour,
/// and between where.
pub(super) struct Setting<'a> {
    font: &'a f::Font,
    /// How high the letters are, in pixels.
    height: f32,
    /// Pixels to each of the font's own units.
    scale: f32,
    color: [u8; 4],
    /// Where a line may begin and must end, inside the gutter and the
    /// field's margins.
    inner_left: f32,
    inner_right: f32,
    /// How far in a line set against the left begins.
    indent: f32,
    /// Room left between one line and the next, beyond their own height.
    pub(super) leading: f32,
    align: &'a f::Align,
    /// How far a line reaches above its baseline, and below it.
    pub(super) ascent: f32,
    pub(super) descent: f32,
}

impl<'a> Setting<'a> {
    /// How `text` sets its lines in `font`, with letters `height` pixels
    /// high.
    pub(super) fn of(text: &'a f::EditText, font: &'a f::Font, height: f64) -> Setting<'a> {
        let scale = (height / font.em_size) as f32;
        let color = text.color.map_or([0, 0, 0, 255], |c| [c.r, c.g, c.b, c.a]);
        let (left, right, indent, leading, align) = match &text.layout {
            Some(layout) => (
                layout.left_margin as f32,
                layout.right_margin as f32,
                layout.indent as f32,
                layout.leading as f32,
                &layout.align,
            ),
            None => (0.0, 0.0, 0.0, 0.0, &f::Align::Left),
        };
        let inner_left = text.bounds.x_min as f32 + GUTTER + left;
        let inner_right = text.bounds.x_max as f32 - GUTTER - right;
        let (ascent, descent) = match &font.metrics {
            Some(metrics) => (
                metrics.ascent as f32 * scale,
                metrics.descent as f32 * scale,
            ),
            // A typical split for a font that does not say.
            None => (height as f32 * 0.8, height as f32 * 0.2),
        };
        Setting {
            font,
            height: height as f32,
            scale,
            color,
            inner_left,
            inner_right,
            indent,
            leading,
            align,
            ascent,
            descent,
        }
    }

    /// The font's glyph for a character, if it has one.
    fn glyph_for(&self, c: char) -> Option<&'a f::Glyph> {
        self.font
            .glyphs
            .iter()
            .find(|glyph| glyph.char.starts_with(c))
    }

    /// Where a line's first letter goes, by the side the field sets its
    /// lines against.
    fn start_of(&self, line: &str) -> f32 {
        let width: f32 = line
            .chars()
            .filter_map(|c| self.glyph_for(c))
            .map(|glyph| glyph.advance as f32 * self.scale)
            .sum();
        match self.align {
            f::Align::Right => self.inner_right - width,
            f::Align::Center => (self.inner_left + self.inner_right - width) / 2.0,
            f::Align::Left | f::Align::Other(_) => self.inner_left + self.indent,
        }
    }

    /// Fills the letters of one line, and the caret if it is in the line,
    /// with the line standing on `baseline`.
    pub(super) fn line(&self, line: &str, baseline: f32, builder: &mut Builder) -> Result<()> {
        let mut pen = self.start_of(line);
        for c in line.chars() {
            if c == CARET {
                builder.fill(&self.caret(pen, baseline), self.color, Paint::Solid)?;
                continue;
            }
            let Some(glyph) = self.glyph_for(c) else {
                continue;
            };
            let place =
                Matrix::translate(pen, baseline).then_inner(Matrix::scale(self.scale, self.scale));
            let outline = lyon_path(parse_path(&glyph.path)?.into_iter(), place);
            builder.fill(&outline, self.color, Paint::Solid)?;
            pen += glyph.advance as f32 * self.scale;
        }
        Ok(())
    }

    /// The caret where the pen is: a thin bar the height of a line, taking
    /// up no room.
    fn caret(&self, pen: f32, baseline: f32) -> lyon::path::Path {
        let width = (self.height / 16.0).max(1.0);
        let (top, bottom) = (baseline - self.ascent, baseline + self.descent);
        let bar = [
            Segment::Move(pen, top),
            Segment::Line(pen + width, top),
            Segment::Line(pen + width, bottom),
            Segment::Line(pen, bottom),
            Segment::Close,
        ];
        lyon_path(bar.into_iter(), Matrix::IDENTITY)
    }
}
