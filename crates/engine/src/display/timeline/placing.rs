//! What one instruction of a frame does to a clip's children: puts a new
//! one in, changes one that is there, or takes one away.

use bb_format::{Look, Op, Place, PlaceAction, SymbolId, SymbolInfo};

use crate::display::{
    ButtonMode, ButtonState, Child, Children, ClipState, Content, Event, Held, Path,
};
use crate::library::Library;
use crate::math::{ColorTransform, Matrix};

/// Applies one change to `children`, the display list of the clip at `path`.
pub(super) fn apply(
    children: &mut Children,
    op: &Op,
    frame: u16,
    library: &Library,
    events: &mut Vec<Event>,
    path: &mut Path,
) {
    // The path of whatever this change puts at its depth.
    let mut placed = |symbol: SymbolId, depth: u16, events: &mut Vec<Event>| {
        path.push(depth);
        let child = new_child(symbol, frame, library, events, path);
        path.pop();
        child
    };
    match op {
        Op::Remove { depth } => {
            children.remove(depth);
        }
        Op::Place(place) => match place.action {
            PlaceAction::Place(symbol) => {
                // Flash ignores a placement at a depth that is already taken.
                if !children.contains_key(&place.depth)
                    && let Some(mut child) = placed(symbol, place.depth, events)
                {
                    update(&mut child, place);
                    children.insert(place.depth, child);
                }
            }
            PlaceAction::Modify => {
                if let Some(child) = children.get_mut(&place.depth) {
                    update(child, place);
                }
            }
            PlaceAction::Replace(symbol) => {
                let Some(mut child) = placed(symbol, place.depth, events) else {
                    return;
                };
                // The new object takes over the old one's settings, apart
                // from the ones this placement gives.
                if let Some(old) = children.remove(&place.depth) {
                    child.matrix = old.matrix;
                    child.color = old.color;
                    child.ratio = old.ratio;
                    child.clip_depth = old.clip_depth;
                    child.name = old.name;
                    child.visible = old.visible;
                    child.filters = old.filters;
                    child.held = old.held;
                }
                update(&mut child, place);
                children.insert(place.depth, child);
            }
        },
    }
}

/// Applies the settings a placement names, leaving the rest alone.
fn update(child: &mut Child, place: &Place) {
    if let Some(matrix) = place.matrix
        && !child.held.matrix
    {
        child.matrix = matrix.into();
    }
    if let Some(color) = place.color
        && !child.held.color
    {
        child.color = color.into();
    }
    if let Some(ratio) = place.ratio {
        child.ratio = ratio;
    }
    if let Some(name) = &place.name {
        child.name = Some(name.clone());
    }
    if let Some(clip_depth) = place.clip_depth {
        child.clip_depth = Some(clip_depth);
    }
    if let Some(visible) = place.visible
        && !child.held.visible
    {
        child.visible = visible;
    }
    if let Some(filters) = &place.filters {
        child.filters = filters.clone();
    }
}

/// A fresh instance of `symbol` to sit at `path`, or `None` if there is
/// nothing to show for it.
pub(super) fn new_child(
    symbol: SymbolId,
    placed_on: u16,
    library: &Library,
    events: &mut Vec<Event>,
    path: &mut Path,
) -> Option<Child> {
    let content = match &library.manifest.symbols.get(&symbol)?.info {
        SymbolInfo::Clip { .. } => {
            Content::Clip(ClipState::new(Some(symbol), library, events, path))
        }
        SymbolInfo::Button => {
            let button = library.buttons.get(&symbol)?;
            let mut state = ButtonState {
                mode: ButtonMode::Up,
                up: Children::new(),
                over: Children::new(),
                down: Children::new(),
                hit: Children::new(),
            };
            for record in &button.records {
                for name in &record.states {
                    let children = match name {
                        Look::Up => &mut state.up,
                        Look::Over => &mut state.over,
                        Look::Down => &mut state.down,
                        Look::Hit => &mut state.hit,
                        Look::Other(_) => continue,
                    };
                    path.push(record.depth);
                    let made = new_child(record.symbol, 0, library, events, path);
                    path.pop();
                    if let Some(mut child) = made {
                        child.matrix = record.matrix.into();
                        if let Some(color) = record.color {
                            child.color = color.into();
                        }
                        child.filters = record.filters.clone();
                        children.insert(record.depth, child);
                    }
                }
            }
            Content::Button(state)
        }
        SymbolInfo::Shape { .. }
        | SymbolInfo::MorphShape
        | SymbolInfo::Text
        | SymbolInfo::EditText => Content::Graphic,
        SymbolInfo::Bitmap { .. } | SymbolInfo::Sound { .. } | SymbolInfo::Font { .. } => {
            return None;
        }
    };
    Some(Child {
        symbol,
        matrix: Matrix::IDENTITY,
        color: ColorTransform::IDENTITY,
        ratio: 0,
        clip_depth: None,
        name: None,
        visible: true,
        filters: Vec::new(),
        placed_on,
        held: Held::default(),
        attached: false,
        said: None,
        content,
    })
}
