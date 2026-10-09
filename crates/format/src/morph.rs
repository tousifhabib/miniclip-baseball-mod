//! `morphs/*.json`: a shape that blends between two outlines, and what it
//! is filled and drawn with.

use serde::{Deserialize, Serialize};

use crate::{Color, Matrix, Rect, SymbolId};

/// A shape that blends between two outlines. Every path has the same commands
/// in `start` and `end`, so a blend is a straight interpolation of the numbers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MorphShape {
    pub id: SymbolId,
    pub start_bounds: Rect,
    pub end_bounds: Rect,
    /// In drawing order.
    pub paths: Vec<MorphPath>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MorphPath {
    /// SVG path data: only `M`, `Q` and `Z`.
    pub start: String,
    pub end: String,
    #[serde(flatten)]
    pub style: MorphStyle,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "draw", rename_all = "snake_case")]
pub enum MorphStyle {
    Fill {
        start_paint: Paint,
        end_paint: Paint,
    },
    Stroke {
        start_width: f64,
        end_width: f64,
        start_paint: Paint,
        end_paint: Paint,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Paint {
    Solid {
        color: Color,
    },
    /// Runs along x from -819.2 to 819.2 before `matrix` is applied.
    LinearGradient {
        matrix: Matrix,
        stops: Vec<GradientStop>,
    },
    /// Centred on the origin with radius 819.2 before `matrix` is applied.
    RadialGradient {
        matrix: Matrix,
        stops: Vec<GradientStop>,
        /// Focal point along x, from -1 to 1.
        #[serde(default, skip_serializing_if = "is_zero_f64")]
        focal: f64,
    },
    /// `matrix` maps bitmap pixels to shape coordinates.
    Bitmap {
        bitmap: SymbolId,
        matrix: Matrix,
        smoothed: bool,
        repeating: bool,
    },
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde hands the field over by reference"
)]
fn is_zero_f64(n: &f64) -> bool {
    *n == 0.0
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    /// 0 to 1.
    pub offset: f64,
    pub color: Color,
}
