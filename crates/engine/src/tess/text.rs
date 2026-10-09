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
    use crate::testing::{FIELD, add_field, frame, library_with};

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

    const FONT: f::SymbolId = 40;

    /// A library with a text field from (0, 0) to (100, 40), whose letters
    /// are 20 high. Its font has one letter: an `a` that is a square half as
    /// high as that, standing on the line, with a fifth as much again to
    /// the next letter. A line reaches 18 above where it stands and 6 below.
    fn with_a_field() -> Library {
        let mut library = library_with(vec![frame(vec![])], vec![]);
        add_field(&mut library, "said", &[]);
        let square = f::Glyph {
            code: 97,
            char: "a".to_owned(),
            advance: 6.0,
            path: "M0 0 L5 0 L5 -5 L0 -5 Z".to_owned(),
        };
        let font = f::Font {
            id: FONT,
            name: "Squares".to_owned(),
            bold: false,
            italic: false,
            em_size: 10.0,
            metrics: Some(f::FontMetrics {
                ascent: 9.0,
                descent: 3.0,
                leading: 0.0,
                kerning: Vec::new(),
            }),
            glyphs: vec![square],
        };
        library.fonts.insert(FONT, font);
        let field = field(&mut library);
        (field.bounds.x_max, field.bounds.y_max) = (100.0, 40.0);
        field.font = Some(FONT);
        field.height = Some(20.0);
        library
    }

    fn field(library: &mut Library) -> &mut f::EditText {
        library.edit_texts.get_mut(&FIELD).unwrap()
    }

    /// The triangles of the field when it says `said`.
    fn cut(library: &Library, said: &str) -> Mesh {
        let field = &library.edit_texts[&FIELD];
        Tessellator::default()
            .edit_text(field, Some(said), library)
            .unwrap()
    }

    /// Where a mesh has corners: every place across that has one, and every
    /// place down, each in order.
    fn edges(mesh: &Mesh) -> (Vec<f32>, Vec<f32>) {
        let along = |axis: usize| {
            let mut places: Vec<f32> = mesh
                .vertices
                .iter()
                .map(|vertex| vertex.position[axis])
                .collect();
            places.sort_by(f32::total_cmp);
            places.dedup();
            places
        };
        (along(0), along(1))
    }

    /// How a field with margins of 3 and 5, lines begun 7 in and 4 between
    /// one line and the next sets its lines against a side.
    fn set_against(align: f::Align) -> f::TextLayout {
        f::TextLayout {
            align,
            left_margin: 3.0,
            right_margin: 5.0,
            indent: 7.0,
            leading: 4.0,
        }
    }

    #[test]
    fn a_field_sets_its_letters_from_the_left_inside_its_gutter() {
        let library = with_a_field();
        // Two pixels in, each letter 10 across and 12 on from the last,
        // standing 18 below the gutter at the top.
        let across = vec![2.0, 12.0, 14.0, 24.0];
        assert_eq!(edges(&cut(&library, "aa")), (across, vec![10.0, 20.0]));
    }

    #[test]
    fn a_field_sets_its_lines_against_the_side_it_names() {
        let mut library = with_a_field();
        let sides = [
            // Inside the left margin, and begun as far in as the field says.
            (f::Align::Left, [12.0, 22.0, 24.0, 34.0]),
            // Ending at the right margin.
            (f::Align::Right, [69.0, 79.0, 81.0, 91.0]),
            // Half way between the two margins.
            (f::Align::Center, [37.0, 47.0, 49.0, 59.0]),
            // A side the engine has no name for is taken to be the left.
            (f::Align::from("justify"), [12.0, 22.0, 24.0, 34.0]),
        ];
        for (side, across) in sides {
            field(&mut library).layout = Some(set_against(side.clone()));
            assert_eq!(edges(&cut(&library, "aa")).0, across, "{side:?}");
        }
    }

    #[test]
    fn each_line_stands_below_the_last_by_its_height_and_the_room_between() {
        let mut library = with_a_field();
        field(&mut library).layout = Some(set_against(f::Align::Left));
        // A line is 18 and 6 high, and there are 4 between lines. Either
        // kind of line end begins a new one.
        let down = vec![10.0, 20.0, 38.0, 48.0, 66.0, 76.0];
        assert_eq!(edges(&cut(&library, "a\na\ra")), (vec![12.0, 22.0], down));
    }

    #[test]
    fn the_caret_is_a_bar_as_high_as_the_line_where_the_next_letter_would_go() {
        let mut library = with_a_field();
        field(&mut library).color = Some(f::Color {
            r: 200,
            g: 10,
            b: 20,
            a: 255,
        });
        // A sixteenth as wide as the letters are high, from the top of the
        // line to the bottom of it.
        let mesh = cut(&library, &format!("a{CARET}"));
        let across = vec![2.0, 12.0, 14.0, 15.25];
        assert_eq!(edges(&mesh), (across, vec![2.0, 10.0, 20.0, 26.0]));
        // It is the colour of the letters.
        assert!(mesh.vertices.iter().all(|v| v.color == [200, 10, 20, 255]));
        // It takes up no room: the letter after it is where it would have
        // been.
        let mesh = cut(&library, &format!("{CARET}a"));
        assert_eq!(edges(&mesh).0, [2.0, 3.25, 12.0]);
        // And it is never less than a pixel wide.
        field(&mut library).height = Some(10.0);
        let mesh = cut(&library, &CARET.to_string());
        assert_eq!(edges(&mesh), (vec![2.0, 3.0], vec![2.0, 14.0]));
    }

    #[test]
    fn a_field_shows_only_what_its_font_has_letters_for() {
        let mut library = with_a_field();
        // A letter the font lacks is left out and takes up no room.
        assert_eq!(edges(&cut(&library, "bab")).0, [2.0, 12.0]);
        assert!(cut(&library, "").vertices.is_empty());
        assert!(cut(&library, "b").vertices.is_empty());
        // Told nothing, a field says what it starts with.
        let starts_with = |library: &Library| {
            let field = &library.edit_texts[&FIELD];
            let mut tessellator = Tessellator::default();
            tessellator.edit_text(field, None, library).unwrap()
        };
        assert!(starts_with(&library).vertices.is_empty());
        field(&mut library).initial_text = Some("a".to_owned());
        assert_eq!(edges(&starts_with(&library)).0, [2.0, 12.0]);
        // With no font, or no size, there is nothing to cut.
        field(&mut library).height = None;
        assert!(cut(&library, "a").vertices.is_empty());
        field(&mut library).height = Some(20.0);
        field(&mut library).font = None;
        assert!(cut(&library, "a").vertices.is_empty());
    }

    #[test]
    fn a_font_that_does_not_say_how_high_a_line_is_has_four_fifths_of_it_above() {
        let mut library = with_a_field();
        library.fonts.get_mut(&FONT).unwrap().metrics = None;
        // 16 of the 20 above where the line stands, and 4 below.
        assert_eq!(edges(&cut(&library, "a")).1, [8.0, 18.0]);
        assert_eq!(edges(&cut(&library, "a\na")).1, [8.0, 18.0, 28.0, 38.0]);
    }

    #[test]
    fn a_field_of_markup_is_cut_as_its_plain_words() {
        let mut library = with_a_field();
        let markup = "<p>a</p><p>a</p>";
        // Left as it is, the marks are letters like any others, and the
        // font has only the two.
        assert_eq!(
            edges(&cut(&library, markup)),
            (vec![2.0, 12.0, 14.0, 24.0], vec![10.0, 20.0])
        );
        field(&mut library).flags = vec![f::FieldFlag::Html];
        assert_eq!(
            edges(&cut(&library, markup)),
            (vec![2.0, 12.0], vec![10.0, 20.0, 34.0, 44.0])
        );
    }
}
