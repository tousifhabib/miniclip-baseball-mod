//! The display tree: clips nested inside clips, each playing its own
//! timeline.
//!
//! What the tree is made of is here. Playing a timeline is in `timeline`,
//! the area an object covers in `bounds`, the list of what there is to draw
//! in `draw`, and the tree written out to be read in `describe`.

mod bounds;
mod describe;
mod draw;
mod timeline;

use std::collections::BTreeMap;

use bb_format::{Filter, SoundStart, SymbolId};

use crate::library::Library;
use crate::math::{ColorTransform, Matrix};

pub use bounds::{Bounds, bounds_of, child_bounds, union};
pub use describe::describe_tree;
pub use draw::{CARET, Command, Texts, commands, commands_upright, text_key};

/// Children by depth. Higher depths draw on top.
pub type Children = BTreeMap<u16, Child>;

/// Where an object is in the tree: the depth to follow at each level, from
/// the top timeline down.
pub type Path = Vec<u16>;

/// One object on a display list.
#[derive(Clone, Debug)]
pub struct Child {
    pub symbol: SymbolId,
    pub matrix: Matrix,
    pub color: ColorTransform,
    /// Morph position, 0 to 65535.
    pub ratio: u16,
    /// When set, this object is a mask for the depths above it up to here.
    pub clip_depth: Option<u16>,
    pub name: Option<String>,
    pub visible: bool,
    /// Applied to this object and everything inside it, drawn as one picture.
    pub filters: Vec<Filter>,
    /// The frame of the parent timeline that put this object here.
    pub placed_on: u16,
    /// What the game's rules have taken charge of.
    pub held: Held,
    /// The rules put this object here themselves, as `attachMovie` did. No
    /// timeline placed it, so none removes it: it stays until the rules take
    /// it away or its parent goes.
    pub attached: bool,
    /// For a text field: what this one instance says, whatever the variable
    /// it shows holds. For a field the rules have added themselves.
    pub said: Option<String>,
    pub content: Content,
}

/// The settings of an object that the game's rules have set themselves. The
/// timeline leaves these alone from then on, as in Flash, where an object a
/// script has moved stops following its tween.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held {
    pub matrix: bool,
    pub color: bool,
    pub visible: bool,
}

impl Child {
    /// Puts the object's origin at this point of its parent, keeping its
    /// size and turn, and takes its placing out of the timeline's hands.
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.matrix.tx = x;
        self.matrix.ty = y;
        self.held.matrix = true;
    }

    /// Sets the object's whole transform, and takes it out of the timeline's
    /// hands.
    pub fn set_matrix(&mut self, matrix: Matrix) {
        self.matrix = matrix;
        self.held.matrix = true;
    }

    /// Sets the object's colour transform, and takes it out of the
    /// timeline's hands.
    pub fn set_color(&mut self, color: ColorTransform) {
        self.color = color;
        self.held.color = true;
    }

    /// Sets how solid the object is, from 0 (not there) to 1, leaving its
    /// colours alone.
    pub fn set_alpha(&mut self, alpha: f32) {
        self.color.mult[3] = alpha;
        self.held.color = true;
    }

    /// Shows or hides the object, and takes that out of the timeline's
    /// hands.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        self.held.visible = true;
    }
}

#[derive(Clone, Debug)]
pub enum Content {
    /// Drawn as it is: a shape, a morph shape or text.
    Graphic,
    Clip(ClipState),
    Button(ButtonState),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonMode {
    Up,
    Over,
    Down,
}

/// A button: one set of objects per look, and an area that reacts to the
/// pointer.
#[derive(Clone, Debug)]
pub struct ButtonState {
    pub mode: ButtonMode,
    pub up: Children,
    pub over: Children,
    pub down: Children,
    /// Never drawn.
    pub hit: Children,
}

impl ButtonState {
    /// The objects for the look the button has now.
    pub fn shown(&self) -> &Children {
        match self.mode {
            ButtonMode::Up => &self.up,
            ButtonMode::Over => &self.over,
            ButtonMode::Down => &self.down,
        }
    }

    fn shown_mut(&mut self) -> &mut Children {
        match self.mode {
            ButtonMode::Up => &mut self.up,
            ButtonMode::Over => &mut self.over,
            ButtonMode::Down => &mut self.down,
        }
    }
}

/// What the pointer did to a button, named after the ActionScript events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonEvent {
    RollOver,
    RollOut,
    Press,
    Release,
    ReleaseOutside,
    DragOut,
    DragOver,
}

/// Something that happened while the tree played, for the caller to act on.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A timeline or a button asked for a sound.
    Sound(SoundStart),
    Button {
        symbol: SymbolId,
        path: Path,
        event: ButtonEvent,
    },
    /// A clip landed on a frame where the original had a script. `symbol` is
    /// `None` for the main timeline, whose path is empty.
    Frame {
        symbol: Option<SymbolId>,
        path: Path,
        frame: u16,
    },
}

/// A playing instance of a timeline.
#[derive(Clone, Debug)]
pub struct ClipState {
    /// `None` for the main timeline.
    pub symbol: Option<SymbolId>,
    /// The current frame, counting from 1.
    pub frame: u16,
    pub playing: bool,
    pub children: Children,
}

impl ClipState {
    pub fn frame_count(&self, library: &Library) -> u16 {
        library
            .timeline(self.symbol)
            .map_or(0, |timeline| timeline.frames.len() as u16)
    }

    /// The object at `path`, counted from this clip.
    pub fn child_mut(&mut self, path: &[u16]) -> Option<&mut Child> {
        let (first, rest) = path.split_first()?;
        let child = self.children.get_mut(first)?;
        if rest.is_empty() {
            return Some(child);
        }
        match &mut child.content {
            Content::Clip(clip) => clip.child_mut(rest),
            _ => None,
        }
    }
}
