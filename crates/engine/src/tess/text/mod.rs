//! Cuts text into triangles: text that never changes, and text fields,
//! which say what the game sets them to.

mod markup;
mod setting;

use anyhow::Result;
use bb_format as f;

use super::outline::{lyon_path, parse_path};
use super::{Builder, Mesh, Paint, Tessellator};
use crate::library::Library;
use crate::math::Matrix;
use markup::plain_text;
use setting::Setting;

impl Tessellator {
    /// Tessellates fixed text.
    pub fn text(&mut self, text: &f::Text, library: &Library) -> Result<Mesh> {
        let mut builder = Builder::new();
        let text_matrix = Matrix::from(text.matrix);
        for run in &text.runs {
            let Some(font) = library.fonts.get(&run.font) else {
                continue;
            };
            let scale = (run.height / font.em_size) as f32;
            let color = [run.color.r, run.color.g, run.color.b, run.color.a];
            let mut pen = run.x as f32;
            for placement in &run.glyphs {
                if let Some(glyph) = font.glyphs.get(placement.glyph as usize) {
                    let place = text_matrix
                        .then_inner(Matrix::translate(pen, run.y as f32))
                        .then_inner(Matrix::scale(scale, scale));
                    let outline = lyon_path(parse_path(&glyph.path)?.into_iter(), place);
                    builder.fill(&outline, color, Paint::Solid)?;
                }
                pen += placement.advance as f32;
            }
        }
        Ok(builder.build())
    }

    /// Tessellates a text field showing `content`, or the text it starts
    /// with if that is `None`.
    pub fn edit_text(
        &mut self,
        text: &f::EditText,
        content: Option<&str>,
        library: &Library,
    ) -> Result<Mesh> {
        let mut builder = Builder::new();
        let (Some(font), Some(height), Some(content)) = (
            text.font.and_then(|font| library.fonts.get(&font)),
            text.height,
            content.or(text.initial_text.as_deref()),
        ) else {
            return Ok(builder.build());
        };
        // A field that holds markup is drawn as its plain words, in the
        // field's own font, size and colour.
        let plain;
        let content = if text.flags.contains(&f::FieldFlag::Html) {
            plain = plain_text(content);
            plain.as_str()
        } else {
            content
        };
        if content.is_empty() {
            return Ok(builder.build());
        }

        let setting = Setting::of(text, font, height);
        let mut baseline = text.bounds.y_min as f32 + GUTTER + setting.ascent;
        for line in content.split(['\n', '\r']) {
            setting.line(line, baseline, &mut builder)?;
            baseline += setting.ascent + setting.descent + setting.leading;
        }
        Ok(builder.build())
    }
}

/// Flash keeps a two pixel gutter inside a field's edges.
const GUTTER: f32 = 2.0;

#[cfg(test)]
mod tests;
