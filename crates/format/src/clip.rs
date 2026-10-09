//! `clips/*.json`: a movie clip's timeline, and what each frame of it does.
//! `clips/root.json` is the main timeline.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ColorTransform, Matrix, SoundStart, SymbolId};

/// A movie clip's timeline. `clips/root.json` is the main timeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    /// `None` for the main timeline.
    pub id: Option<SymbolId>,
    pub labels: BTreeMap<String, u16>,
    pub frames: Vec<Frame>,
}

impl Clip {
    /// The frame with this number. Frames are counted from 1, as the
    /// timeline counts them, and the timeline never asks for one it does
    /// not have.
    pub fn frame(&self, number: u16) -> &Frame {
        &self.frames[usize::from(number) - 1]
    }
}

/// What changes on the display list when the playhead enters this frame.
/// Frames hold changes, not full snapshots, exactly as Flash stores them.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Frame {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ops: Vec<Op>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sounds: Vec<SoundStart>,
    /// The original had ActionScript on this frame.
    #[serde(default, skip_serializing_if = "is_false")]
    pub has_script: bool,
    /// That script calls `stop()` before anything that depends on the game's
    /// state, so the timeline always halts here.
    #[serde(default, skip_serializing_if = "is_false")]
    pub stops: bool,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde hands the field over by reference"
)]
fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    Place(Box<Place>),
    Remove { depth: u16 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Place {
    pub depth: u16,
    pub action: PlaceAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix: Option<Matrix>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorTransform>,
    /// Morph position, 0-65535.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratio: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Makes this object a mask for everything up to this depth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip_depth: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<Filter>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blend_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    /// Events the original had `onClipEvent` scripts for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clip_events: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceAction {
    /// Put a new object at an empty depth.
    Place(SymbolId),
    /// Change the object already at this depth.
    Modify,
    /// Swap the object at this depth for another symbol.
    Replace(SymbolId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Filter {
    Blur {
        blur_x: f64,
        blur_y: f64,
        passes: u8,
    },
    /// A filter the extractor does not convert yet.
    Unsupported { name: String },
}
