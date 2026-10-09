//! Data types for the art's files, which the engine reads.
//!
//! Lengths and positions are in pixels (Flash stores twentieths of a pixel).
//! Frame numbers start at 1, as they do in ActionScript.
//!
//! There is a file here for each kind of file the art has, and one each
//! for the things they share.

// Outside the tests nothing is taken for granted: what may be missing is
// dealt with, or the reason it cannot be is given.
#![warn(clippy::unwrap_used)]

mod button;
mod clip;
mod color;
mod font;
mod geometry;
mod manifest;
mod morph;
mod sound;
mod text;
mod words;

pub use button::{Button, ButtonAction, ButtonRecord, ButtonSounds, Look};
pub use clip::{Clip, Filter, Frame, Op, Place, PlaceAction};
pub use color::{Color, ColorTransform};
pub use font::{Font, FontMetrics, Glyph, Kerning};
pub use geometry::{IDENTITY, Matrix, Rect};
pub use manifest::{Manifest, Stage, Symbol, SymbolInfo};
pub use morph::{GradientStop, MorphPath, MorphShape, MorphStyle, Paint};
pub use sound::{EnvelopePoint, SoundEvent, SoundStart};
pub use text::{Align, EditText, FieldFlag, GlyphPlacement, Text, TextLayout, TextRun};

/// Bumped whenever a change would make older extracted files unreadable.
pub const FORMAT_VERSION: u32 = 2;

pub type SymbolId = u16;
