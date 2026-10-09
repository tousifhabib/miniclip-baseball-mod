//! `fonts/*.json`: the outlines of a font's letters, and how they are
//! spaced.

use serde::{Deserialize, Serialize};

use crate::SymbolId;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Font {
    pub id: SymbolId,
    pub name: String,
    pub bold: bool,
    pub italic: bool,
    /// Glyph coordinates and advances are in units of `1 / em_size` of the
    /// font size.
    pub em_size: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<FontMetrics>,
    pub glyphs: Vec<Glyph>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FontMetrics {
    pub ascent: f64,
    pub descent: f64,
    pub leading: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kerning: Vec<Kerning>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Kerning {
    pub left: u16,
    pub right: u16,
    pub adjustment: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Glyph {
    pub code: u16,
    /// The character for `code`, for reading and searching.
    pub char: String,
    pub advance: f64,
    /// SVG path data, filled with the even-odd rule.
    pub path: String,
}
