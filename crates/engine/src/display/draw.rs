//! Lists what there is to draw of the tree, back to front, as steps for the
//! renderer to carry out.

use bb_format::{Filter, SymbolId, SymbolInfo};

use super::{Bounds, Child, Children, ClipState, Content, child_bounds};
use crate::library::Library;
use crate::math::{ColorTransform, Matrix};

/// One step of drawing a frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Draw {
        symbol: SymbolId,
        ratio: u16,
        matrix: Matrix,
        color: ColorTransform,
        /// For a text field: what it says now, if the game has set it.
        text: Option<String>,
    },
    /// The draws up to `ActivateMask` are the outline of a mask.
    PushMask,
    /// From here on, only pixels inside the mask are drawn.
    ActivateMask,
    /// The draws up to `PopMask` repeat the mask's outline, to remove it.
    DeactivateMask,
    PopMask,
    /// The draws up to `EndBlur` are drawn together as one picture, which is
    /// then blurred. `bounds` is the area they cover and the blur sizes are
    /// box widths, all in pixels of the target.
    BeginBlur {
        blur_x: f32,
        blur_y: f32,
        passes: u8,
        bounds: Bounds,
    },
    EndBlur,
}

/// What the game's text fields say, by the name of the variable each field
/// shows. A field that the original bound to `_root.game.score` shows the
/// entry for `score`: only the last part of the name counts.
pub type Texts = std::collections::HashMap<String, String>;

/// The part of a text field's variable name that [`Texts`] is keyed by.
pub fn text_key(variable: &str) -> &str {
    variable.rsplit(['.', ':', '/']).next().unwrap_or(variable)
}

/// Stands for the caret in the text handed to the renderer. It is in the
/// range set aside for private use, so no font has a letter for it.
pub const CARET: char = '\u{e000}';

/// Lists what to draw for a clip, back to front.
pub fn commands(clip: &ClipState, base: Matrix, library: &Library, texts: &Texts) -> Vec<Command> {
    commands_upright(clip, base, library, texts, false)
}

/// The same. With `upright_text`, a text or a text field that would have
/// been drawn mirrored, because a clip it is in has been turned over, is
/// drawn the right way round: turned over again about its own middle, so
/// that it stays where it was. A game that mirrors a figure has no wish to
/// mirror the number on his shirt.
pub fn commands_upright(
    clip: &ClipState,
    base: Matrix,
    library: &Library,
    texts: &Texts,
    upright_text: bool,
) -> Vec<Command> {
    let mut out = Vec::new();
    let context = Context {
        library,
        texts,
        upright_text,
        // Filter sizes are in stage pixels, so they grow with the view.
        view_scale: (base.a * base.d - base.b * base.c).abs().sqrt(),
    };
    draw_children(
        &clip.children,
        base,
        ColorTransform::IDENTITY,
        false,
        &context,
        &mut out,
    );
    out
}

struct Context<'a> {
    library: &'a Library,
    texts: &'a Texts,
    upright_text: bool,
    view_scale: f32,
}

/// The transform to draw a graphic by, given the one its place in the tree
/// gives it: the same, unless that would mirror text that is to be kept
/// the right way round.
fn righted(child: &Child, matrix: Matrix, context: &Context<'_>) -> Matrix {
    if !context.upright_text || matrix.a * matrix.d - matrix.b * matrix.c >= 0.0 {
        return matrix;
    }
    let library = context.library;
    let bounds = match library.manifest.symbols.get(&child.symbol).map(|s| &s.info) {
        Some(SymbolInfo::Text) => library.texts.get(&child.symbol).map(|text| &text.bounds),
        Some(SymbolInfo::EditText) => library
            .edit_texts
            .get(&child.symbol)
            .map(|field| &field.bounds),
        _ => None,
    };
    let Some(bounds) = bounds else {
        return matrix;
    };
    // Turned over about the middle of what it covers.
    matrix.then_inner(Matrix {
        a: -1.0,
        tx: (bounds.x_min + bounds.x_max) as f32,
        ..Matrix::IDENTITY
    })
}

fn draw_children(
    children: &Children,
    matrix: Matrix,
    color: ColorTransform,
    in_mask: bool,
    context: &Context<'_>,
    out: &mut Vec<Command>,
) {
    // Masks that are in force, innermost last, with the depth each ends at.
    let mut masks: Vec<(u16, &Child)> = Vec::new();
    let end_mask = |mask: &Child, out: &mut Vec<Command>| {
        out.push(Command::DeactivateMask);
        draw_child(mask, matrix, color, true, context, out);
        out.push(Command::PopMask);
    };

    for (&depth, child) in children {
        while let Some(&(last_depth, mask)) = masks.last()
            && last_depth < depth
        {
            masks.pop();
            end_mask(mask, out);
        }
        match child.clip_depth {
            // Inside a mask's outline everything is plain geometry.
            Some(clip_depth) if !in_mask => {
                out.push(Command::PushMask);
                draw_child(child, matrix, color, true, context, out);
                out.push(Command::ActivateMask);
                masks.push((clip_depth, child));
            }
            Some(_) => {}
            None if child.visible => draw_child(child, matrix, color, in_mask, context, out),
            None => {}
        }
    }
    while let Some((_, mask)) = masks.pop() {
        end_mask(mask, out);
    }
}

fn draw_child(
    child: &Child,
    parent_matrix: Matrix,
    parent_color: ColorTransform,
    in_mask: bool,
    context: &Context<'_>,
    out: &mut Vec<Command>,
) {
    // A mask is only an outline, so filters do nothing to it.
    let blur = (!in_mask)
        .then(|| {
            child.filters.iter().find_map(|filter| match *filter {
                Filter::Blur {
                    blur_x,
                    blur_y,
                    passes,
                } if passes > 0 && (blur_x > 1.0 || blur_y > 1.0) => Some((blur_x, blur_y, passes)),
                _ => None,
            })
        })
        .flatten()
        .zip(child_bounds(child, parent_matrix, context.library));
    if let Some(((blur_x, blur_y, passes), bounds)) = blur {
        out.push(Command::BeginBlur {
            blur_x: blur_x as f32 * context.view_scale,
            blur_y: blur_y as f32 * context.view_scale,
            passes,
            bounds,
        });
    }

    let matrix = parent_matrix.then_inner(child.matrix);
    let color = parent_color.then_inner(child.color);
    match &child.content {
        Content::Graphic => out.push(Command::Draw {
            symbol: child.symbol,
            ratio: child.ratio,
            matrix: righted(child, matrix, context),
            color,
            text: child.said.clone().or_else(|| {
                context
                    .library
                    .edit_texts
                    .get(&child.symbol)
                    .and_then(|field| context.texts.get(text_key(&field.variable)))
                    .cloned()
            }),
        }),
        Content::Clip(clip) => {
            draw_children(&clip.children, matrix, color, in_mask, context, out);
        }
        Content::Button(button) => {
            draw_children(button.shown(), matrix, color, in_mask, context, out);
        }
    }

    if blur.is_some() {
        out.push(Command::EndBlur);
    }
}

#[cfg(test)]
mod tests {
    use bb_format::{Op, Place, PlaceAction};

    use super::*;
    use crate::testing::{FIELD, OTHER_SHAPE, SHAPE, add_field, frame, library, place, put, start};

    fn draw(clip: &ClipState, base: Matrix, library: &Library) -> Vec<Command> {
        commands(clip, base, library, &Texts::new())
    }

    fn kinds(commands: &[Command]) -> Vec<String> {
        commands
            .iter()
            .map(|command| match command {
                Command::Draw { symbol, .. } => format!("draw {symbol}"),
                Command::BeginBlur { .. } => "BeginBlur".to_owned(),
                other => format!("{other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_text_field_shows_what_the_game_has_set() {
        let mut library = library(vec![frame(vec![put(1, FIELD)])], 1);
        add_field(&mut library, "_root.game.score", &["read_only"]);
        let root = start(&library);
        let said = |texts: &Texts| match &commands(&root, Matrix::IDENTITY, &library, texts)[0] {
            Command::Draw { text, .. } => text.clone(),
            other => panic!("expected a draw, found {other:?}"),
        };
        assert_eq!(said(&Texts::new()), None);
        // Only the last part of the field's variable name counts.
        let texts = Texts::from([("score".to_owned(), "12".to_owned())]);
        assert_eq!(said(&texts), Some("12".to_owned()));
    }

    #[test]
    fn text_in_a_mirrored_clip_is_drawn_the_right_way_round_when_asked() {
        let mut library = library(vec![frame(vec![put(1, FIELD)])], 1);
        add_field(&mut library, "score", &["read_only"]);
        let root = start(&library);
        // The whole thing turned over about the line 100 across.
        let mirror = Matrix {
            a: -1.0,
            tx: 200.0,
            ..Matrix::IDENTITY
        };
        let drawn = |upright: bool| {
            let texts = Texts::new();
            match &commands_upright(&root, mirror, &library, &texts, upright)[0] {
                Command::Draw { matrix, .. } => *matrix,
                other => panic!("expected a draw, found {other:?}"),
            }
        };
        // Left alone it is mirrored: the field, 50 wide, runs from 200
        // back to 150.
        let plain = drawn(false);
        assert_eq!((plain.a, plain.apply(0.0, 0.0).0), (-1.0, 200.0));
        // Kept upright it covers the same 150 to 200, and reads forwards.
        let upright = drawn(true);
        assert_eq!(upright.a, 1.0);
        assert_eq!(upright.apply(0.0, 0.0).0, 150.0);
        assert_eq!(upright.apply(50.0, 0.0).0, 200.0);
        // Nothing is done to what is not mirrored.
        let texts = Texts::new();
        let same = commands_upright(&root, Matrix::IDENTITY, &library, &texts, true);
        assert_eq!(same, commands(&root, Matrix::IDENTITY, &library, &texts));
    }

    #[test]
    fn a_mask_wraps_the_depths_it_covers() {
        let mask = Place {
            clip_depth: Some(2),
            ..place(1, PlaceAction::Place(SHAPE))
        };
        let library = library(
            vec![frame(vec![
                Op::Place(Box::new(mask)),
                put(2, OTHER_SHAPE),
                put(3, OTHER_SHAPE),
            ])],
            1,
        );
        let root = start(&library);
        // Symbol 1 is the mask. Symbol 2 sits at depth 2, inside the mask, and
        // again at depth 3, past its end.
        assert_eq!(
            kinds(&draw(&root, Matrix::IDENTITY, &library)),
            [
                "PushMask",
                "draw 1",
                "ActivateMask",
                "draw 2",
                "DeactivateMask",
                "draw 1",
                "PopMask",
                "draw 2",
            ]
        );
    }

    #[test]
    fn nested_transforms_multiply_outermost_first() {
        let moved = Place {
            matrix: Some([1.0, 0.0, 0.0, 1.0, 3.0, 4.0]),
            ..place(1, PlaceAction::Place(SHAPE))
        };
        let library = library(vec![frame(vec![Op::Place(Box::new(moved))])], 1);
        let root = start(&library);
        let commands = draw(&root, Matrix::scale(2.0, 2.0), &library);
        let Command::Draw { matrix, .. } = &commands[0] else {
            panic!("expected a draw");
        };
        assert_eq!(matrix.apply(0.0, 0.0), (6.0, 8.0));
    }

    #[test]
    fn a_blurred_object_is_wrapped_with_its_area_and_scaled_blur() {
        let blurred = Place {
            matrix: Some([1.0, 0.0, 0.0, 1.0, 20.0, 30.0]),
            filters: Some(vec![Filter::Blur {
                blur_x: 5.0,
                blur_y: 4.0,
                passes: 1,
            }]),
            ..place(1, PlaceAction::Place(SHAPE))
        };
        let library = library(vec![frame(vec![Op::Place(Box::new(blurred))])], 1);
        let root = start(&library);
        let commands = draw(&root, Matrix::scale(2.0, 2.0), &library);
        assert_eq!(kinds(&commands), ["BeginBlur", "draw 1", "EndBlur"]);
        // The 10 by 10 shape at (20, 30), drawn at twice the size.
        assert_eq!(
            commands[0],
            Command::BeginBlur {
                blur_x: 10.0,
                blur_y: 8.0,
                passes: 1,
                bounds: [40.0, 60.0, 60.0, 80.0],
            }
        );
    }
}
