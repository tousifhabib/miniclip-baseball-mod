//! Cuts text into triangles: text that never changes, and text fields,
//! which say what the game sets them to.

use anyhow::Result;
use bb_format as f;

use super::outline::{Segment, lyon_path, parse_path};
use super::{Builder, Mesh, Paint, Tessellator};
use crate::display::CARET;
use crate::library::Library;
use crate::math::Matrix;

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

        let scale = (height / font.em_size) as f32;
        let color = text.color.map_or([0, 0, 0, 255], |c| [c.r, c.g, c.b, c.a]);
        let glyph_for = |c: char| font.glyphs.iter().find(|glyph| glyph.char.starts_with(c));
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
        // Flash keeps a two pixel gutter inside a field's edges.
        const GUTTER: f32 = 2.0;
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

        let mut baseline = text.bounds.y_min as f32 + GUTTER + ascent;
        for line in content.split(['\n', '\r']) {
            let width: f32 = line
                .chars()
                .filter_map(glyph_for)
                .map(|glyph| glyph.advance as f32 * scale)
                .sum();
            let mut pen = match align {
                f::Align::Right => inner_right - width,
                f::Align::Center => (inner_left + inner_right - width) / 2.0,
                f::Align::Left | f::Align::Other(_) => inner_left + indent,
            };
            for c in line.chars() {
                if c == CARET {
                    // A thin bar the height of a line, taking up no room.
                    let width = (height as f32 / 16.0).max(1.0);
                    let (top, bottom) = (baseline - ascent, baseline + descent);
                    let bar = [
                        Segment::Move(pen, top),
                        Segment::Line(pen + width, top),
                        Segment::Line(pen + width, bottom),
                        Segment::Line(pen, bottom),
                        Segment::Close,
                    ];
                    let outline = lyon_path(bar.into_iter(), Matrix::IDENTITY);
                    builder.fill(&outline, color, Paint::Solid)?;
                    continue;
                }
                let Some(glyph) = glyph_for(c) else {
                    continue;
                };
                let place =
                    Matrix::translate(pen, baseline).then_inner(Matrix::scale(scale, scale));
                let outline = lyon_path(parse_path(&glyph.path)?.into_iter(), place);
                builder.fill(&outline, color, Paint::Solid)?;
                pen += glyph.advance as f32 * scale;
            }
            baseline += ascent + descent + leading;
        }
        Ok(builder.build())
    }
}

/// The words of some HTML-like markup, as Flash text fields hold it: tags
/// dropped, paragraphs and line breaks turned into new lines, and the common
/// entities turned back into their characters.
fn plain_text(markup: &str) -> String {
    let mut out = String::new();
    let mut rest = markup;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('>') else {
            // An unfinished tag: keep it as written.
            out.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let tag = rest[open + 1..open + close].trim().to_ascii_lowercase();
        let name = tag.trim_start_matches('/').split_whitespace().next();
        // A line ends at a break, and at the end of a paragraph.
        if matches!(name, Some("br" | "br/")) || tag.starts_with("/p") {
            out.push('\n');
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    let out = [
        ("&nbsp;", " "),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&apos;", "'"),
        ("&amp;", "&"),
    ]
    .iter()
    .fold(out, |text, (entity, character)| {
        text.replace(entity, character)
    });
    out.trim_end_matches('\n').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_becomes_plain_words_and_lines() {
        let markup = r#"<p align="left"><font face="Arial" size="12">Top &amp; <b>bold</b></font></p><p>Next<br>line</p>"#;
        assert_eq!(plain_text(markup), "Top & bold\nNext\nline");
        assert_eq!(plain_text("no tags at all"), "no tags at all");
        assert_eq!(plain_text("a < b"), "a < b");
    }

    #[test]
    fn a_line_ends_at_a_break_however_it_is_written_and_at_the_end_of_a_paragraph() {
        assert_eq!(
            plain_text("one<br>two<BR/>three<br />four"),
            "one\ntwo\nthree\nfour"
        );
        assert_eq!(plain_text("<P>one</P><p>two</p>"), "one\ntwo");
        // Empty lines are kept at the start and in the middle, and dropped
        // from the end.
        assert_eq!(plain_text("<br>one<br><br>two<br><br>"), "\none\n\ntwo");
        // Any other tag is dropped and leaves the line as it was.
        assert_eq!(plain_text("<b>one</b> <i>line</i>"), "one line");
    }

    #[test]
    fn what_stands_for_a_character_becomes_it_once_the_tags_are_gone() {
        assert_eq!(
            plain_text("&lt;b&gt; &quot;hi&quot; &apos;there&apos;&nbsp;you"),
            "<b> \"hi\" 'there' you"
        );
        // An ampersand that was written out is turned back once and no more.
        assert_eq!(plain_text("&amp;lt;"), "&lt;");
        assert_eq!(plain_text("this &amp; that"), "this & that");
    }

    #[test]
    fn a_tag_that_is_never_closed_is_kept_as_it_was_written() {
        assert_eq!(plain_text("<b>bold</b> and <i"), "bold and <i");
        assert_eq!(plain_text("<"), "<");
        assert_eq!(plain_text(""), "");
    }
}
