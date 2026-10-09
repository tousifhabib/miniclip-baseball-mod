//! Works out the area that objects of the tree cover.

use bb_format::SymbolInfo;

use super::{Child, Children, Content};
use crate::library::Library;
use crate::math::Matrix;

/// A rectangle: left, top, right, bottom.
pub type Bounds = [f32; 4];

/// The least rectangle that holds both, where `None` is no area at all.
pub fn union(a: Option<Bounds>, b: Option<Bounds>) -> Option<Bounds> {
    match (a, b) {
        (Some(a), Some(b)) => Some([
            a[0].min(b[0]),
            a[1].min(b[1]),
            a[2].max(b[2]),
            a[3].max(b[3]),
        ]),
        (one, other) => one.or(other),
    }
}

/// The area `children` cover once moved through `matrix`, or `None` if they
/// draw nothing.
pub fn bounds_of(children: &Children, matrix: Matrix, library: &Library) -> Option<Bounds> {
    children
        .values()
        .filter(|child| child.visible && child.clip_depth.is_none())
        .fold(None, |all, child| {
            union(all, child_bounds(child, matrix, library))
        })
}

/// The area one object covers once moved through `matrix`, the transform of
/// its parent.
pub fn child_bounds(child: &Child, matrix: Matrix, library: &Library) -> Option<Bounds> {
    let matrix = matrix.then_inner(child.matrix);
    let own = |r: &bb_format::Rect| {
        let corners = [
            (r.x_min, r.y_min),
            (r.x_max, r.y_min),
            (r.x_max, r.y_max),
            (r.x_min, r.y_max),
        ]
        .map(|(x, y)| matrix.apply(x as f32, y as f32));
        corners
            .iter()
            .fold(None, |all, &(x, y)| union(all, Some([x, y, x, y])))
    };
    match &child.content {
        Content::Clip(clip) => bounds_of(&clip.children, matrix, library),
        Content::Button(button) => bounds_of(button.shown(), matrix, library),
        Content::Graphic => match &library.manifest.symbols.get(&child.symbol)?.info {
            SymbolInfo::Shape { bounds } => own(bounds),
            SymbolInfo::Text => own(&library.texts.get(&child.symbol)?.bounds),
            SymbolInfo::EditText => own(&library.edit_texts.get(&child.symbol)?.bounds),
            SymbolInfo::MorphShape => {
                let morph = library.morphs.get(&child.symbol)?;
                union(own(&morph.start_bounds), own(&morph.end_bounds))
            }
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use bb_format::{Op, Place, PlaceAction};

    use super::*;
    use crate::testing::{INNER, SHAPE, frame, library_with, place, put, start};

    #[test]
    fn bounds_cover_everything_inside_a_clip() {
        let far = Place {
            matrix: Some([1.0, 0.0, 0.0, 1.0, 50.0, 0.0]),
            ..place(2, PlaceAction::Place(SHAPE))
        };
        let inner = vec![frame(vec![put(1, SHAPE), Op::Place(Box::new(far))])];
        let library = library_with(vec![frame(vec![put(1, INNER)])], inner);
        let root = start(&library);
        assert_eq!(
            bounds_of(&root.children, Matrix::IDENTITY, &library),
            Some([0.0, 0.0, 60.0, 10.0])
        );
    }
}
