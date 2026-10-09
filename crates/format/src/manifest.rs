//! `manifest.json`: what there is in the art, and what kind of thing each
//! symbol is.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Color, Rect, SymbolId};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u32,
    pub swf_version: u8,
    pub stage: Stage,
    pub symbols: BTreeMap<SymbolId, Symbol>,
    /// Linkage names, as used by `attachMovie` and `attachSound`.
    pub exports: BTreeMap<String, SymbolId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stage {
    pub width: f64,
    pub height: f64,
    pub frame_rate: f64,
    pub frame_count: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Color>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Symbol {
    /// Where the symbol's file is, relative to the manifest.
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_name: Option<String>,
    #[serde(flatten)]
    pub info: SymbolInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SymbolInfo {
    Shape {
        bounds: Rect,
    },
    MorphShape,
    Bitmap {
        width: u32,
        height: u32,
    },
    Sound {
        sample_rate: u16,
        stereo: bool,
        sample_count: u32,
        /// Samples the player skips at the start, to hide encoder padding.
        skip_samples: i16,
    },
    Clip {
        frame_count: u16,
    },
    Button,
    Text,
    EditText,
    Font {
        name: String,
    },
}
