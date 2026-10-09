//! What the tests of more than one module are built from: a small library
//! of art, the pieces a timeline is made of, and a pointer with nothing
//! under it.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use bb_format::{
    Clip, EditText, Frame, Manifest, Op, Place, PlaceAction, Rect, Stage, Symbol, SymbolId,
    SymbolInfo,
};

use crate::display::{ClipState, Event, Path};
use crate::input::Geometry;
use crate::library::Library;

/// The two shapes every library made here has, each 10 by 10.
pub(crate) const SHAPE: SymbolId = 1;
pub(crate) const OTHER_SHAPE: SymbolId = 2;
/// The one clip every library made here has.
pub(crate) const INNER: SymbolId = 10;
/// The text field that [`add_field`] adds.
pub(crate) const FIELD: SymbolId = 30;

/// A placement at `depth` that says nothing of where the object goes or how
/// it looks.
pub(crate) fn place(depth: u16, action: PlaceAction) -> Place {
    Place {
        depth,
        action,
        matrix: None,
        color: None,
        ratio: None,
        name: None,
        clip_depth: None,
        filters: None,
        blend_mode: None,
        visible: None,
        clip_events: Vec::new(),
    }
}

/// The change that puts `symbol` at `depth`.
pub(crate) fn put(depth: u16, symbol: SymbolId) -> Op {
    Op::Place(Box::new(place(depth, PlaceAction::Place(symbol))))
}

/// A frame that makes these changes and does nothing else.
pub(crate) fn frame(ops: Vec<Op>) -> Frame {
    Frame {
        ops,
        ..Frame::default()
    }
}

fn clip(id: Option<SymbolId>, frames: Vec<Frame>) -> Clip {
    Clip {
        id,
        labels: BTreeMap::new(),
        frames,
    }
}

pub(crate) fn symbol(file: &str, info: SymbolInfo) -> Symbol {
    Symbol {
        file: file.to_owned(),
        export_name: None,
        info,
    }
}

/// A library with two 10 by 10 shapes, the given main timeline, and one
/// inner clip with the given frames.
pub(crate) fn library_with(root: Vec<Frame>, inner: Vec<Frame>) -> Library {
    let shape = SymbolInfo::Shape {
        bounds: Rect {
            x_min: 0.0,
            y_min: 0.0,
            x_max: 10.0,
            y_max: 10.0,
        },
    };
    let mut symbols = BTreeMap::new();
    symbols.insert(SHAPE, symbol("shapes/1.svg", shape.clone()));
    symbols.insert(OTHER_SHAPE, symbol("shapes/2.svg", shape));
    let inner_info = SymbolInfo::Clip {
        frame_count: inner.len() as u16,
    };
    symbols.insert(INNER, symbol("clips/10.json", inner_info));
    Library {
        dir: PathBuf::new(),
        obey_stops: true,
        manifest: Manifest {
            format_version: bb_format::FORMAT_VERSION,
            swf_version: 8,
            stage: Stage {
                width: 100.0,
                height: 100.0,
                frame_rate: 60.0,
                frame_count: root.len() as u16,
                background: None,
            },
            symbols,
            exports: BTreeMap::new(),
        },
        root: clip(None, root),
        clips: HashMap::from([(INNER, clip(Some(INNER), inner))]),
        buttons: HashMap::new(),
        texts: HashMap::new(),
        edit_texts: HashMap::new(),
        fonts: HashMap::new(),
        morphs: HashMap::new(),
    }
}

/// The same, with an inner clip of so many frames that do nothing.
pub(crate) fn library(root: Vec<Frame>, inner_frames: usize) -> Library {
    library_with(root, vec![Frame::default(); inner_frames])
}

/// The main timeline of `library`, on its first frame.
pub(crate) fn start(library: &Library) -> ClipState {
    ClipState::new(None, library, &mut Vec::new(), &mut Path::new())
}

/// Plays one frame of `clip`, and returns what it reported.
pub(crate) fn tick(clip: &mut ClipState, library: &Library) -> Vec<Event> {
    let mut events = Vec::new();
    clip.advance(library, &mut events, &mut Path::new());
    events
}

/// Adds a text field 50 wide and 20 high, showing `variable`, as symbol
/// [`FIELD`].
pub(crate) fn add_field(library: &mut Library, variable: &str, flags: &[&str]) {
    library
        .manifest
        .symbols
        .insert(FIELD, symbol("texts/30.json", SymbolInfo::EditText));
    library.edit_texts.insert(
        FIELD,
        EditText {
            id: FIELD,
            bounds: Rect {
                x_min: 0.0,
                y_min: 0.0,
                x_max: 50.0,
                y_max: 20.0,
            },
            font: None,
            height: None,
            color: None,
            max_length: None,
            layout: None,
            variable: variable.to_owned(),
            initial_text: Some("0".to_owned()),
            flags: flags.iter().map(|flag| (*flag).into()).collect(),
        },
    );
}

/// Nothing is ever under the pointer.
pub(crate) struct Empty;

impl Geometry for Empty {
    fn contains(&mut self, _: &Library, _: SymbolId, _: u16, _: f32, _: f32) -> bool {
        false
    }
}
