//! Finds the text field under a point, for a click to give the typing to.

use bb_format::{FieldFlag, SymbolId};

use crate::display::{Children, Content, Path};
use crate::library::Library;

/// The topmost text field that can be typed in and has the point inside its
/// box: where it is in the tree, and its symbol. `x` and `y` are in the
/// coordinates of whatever owns `children`.
pub fn field_at(
    children: &Children,
    x: f32,
    y: f32,
    library: &Library,
    path: &mut Path,
) -> Option<(Path, SymbolId)> {
    for (&depth, child) in children.iter().rev() {
        if !child.visible || child.clip_depth.is_some() {
            continue;
        }
        let Some(inverse) = child.matrix.inverse() else {
            continue;
        };
        let (x, y) = inverse.apply(x, y);
        path.push(depth);
        let found = match &child.content {
            Content::Graphic => library
                .edit_texts
                .get(&child.symbol)
                .filter(|field| {
                    let bounds = &field.bounds;
                    !field.flags.contains(&FieldFlag::ReadOnly)
                        && (bounds.x_min as f32..=bounds.x_max as f32).contains(&x)
                        && (bounds.y_min as f32..=bounds.y_max as f32).contains(&y)
                })
                .map(|_| (path.clone(), child.symbol)),
            Content::Clip(clip) => field_at(&clip.children, x, y, library, path),
            // Nothing inside a button takes typing.
            Content::Button(_) => None,
        };
        path.pop();
        if found.is_some() {
            return found;
        }
    }
    None
}
