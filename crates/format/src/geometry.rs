//! Where a thing is and how much room it takes: a transform and a
//! rectangle.

use serde::{Deserialize, Serialize};

/// `[a, b, c, d, tx, ty]`, the same order as SVG's `matrix()`.
pub type Matrix = [f64; 6];

pub const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x_min: f64,
    pub y_min: f64,
    pub x_max: f64,
    pub y_max: f64,
}
