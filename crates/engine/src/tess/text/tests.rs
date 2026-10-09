use super::*;
use crate::stage::CARET;
use crate::testing::{FIELD, add_field, frame, library_with};

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
