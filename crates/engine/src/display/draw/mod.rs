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
mod tests;
