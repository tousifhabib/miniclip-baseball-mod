//! `buttons/*.json`: a button's looks, and what the pointer does to it.

use serde::{Deserialize, Serialize};

use crate::words::words;
use crate::{ColorTransform, Filter, Matrix, SoundStart, SymbolId};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Button {
    pub id: SymbolId,
    pub track_as_menu: bool,
    pub records: Vec<ButtonRecord>,
    /// Mouse transitions the original had scripts for.
    pub actions: Vec<ButtonAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sounds: Option<ButtonSounds>,
}

words! {
    /// One of the ways a button looks, or the area of it that reacts to the
    /// pointer.
    Look {
        Up = "up",
        Over = "over",
        Down = "down",
        /// Never drawn: where the pointer counts as being on the button.
        Hit = "hit",
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ButtonRecord {
    /// Which of the button's looks this object is part of, in the order the
    /// file gives them.
    pub states: Vec<Look>,
    pub symbol: SymbolId,
    pub depth: u16,
    pub matrix: Matrix,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorTransform>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<Filter>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ButtonAction {
    pub conditions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<u8>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ButtonSounds {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub over_to_up: Option<SoundStart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up_to_over: Option<SoundStart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub over_to_down: Option<SoundStart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub down_to_over: Option<SoundStart>,
}
