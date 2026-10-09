//! `texts/*.json`: text that is fixed, and fields of text that can be
//! written in.

use serde::{Deserialize, Serialize};

use crate::words::words;
use crate::{Color, Matrix, Rect, SymbolId};

/// Fixed text, laid out glyph by glyph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Text {
    pub id: SymbolId,
    pub bounds: Rect,
    pub matrix: Matrix,
    pub runs: Vec<TextRun>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextRun {
    pub font: SymbolId,
    pub color: Color,
    /// Pen position of the first glyph.
    pub x: f64,
    pub y: f64,
    /// Font size.
    pub height: f64,
    /// The run's characters, for reading and searching. Drawing uses `glyphs`.
    pub text: String,
    pub glyphs: Vec<GlyphPlacement>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GlyphPlacement {
    /// Index into the font's `glyphs`.
    pub glyph: u32,
    pub advance: f64,
}

/// A text field the game can write to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditText {
    pub id: SymbolId,
    pub bounds: Rect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<SymbolId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<TextLayout>,
    /// The ActionScript variable the field shows.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub variable: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_text: Option<String>,
    pub flags: Vec<FieldFlag>,
}

words! {
    /// Something that is so of a text field.
    FieldFlag {
        WordWrap = "word_wrap",
        Multiline = "multiline",
        Password = "password",
        /// The player cannot type in it.
        ReadOnly = "read_only",
        AutoSize = "auto_size",
        Selectable = "selectable",
        Border = "border",
        /// What it starts by saying is marked up, and is shown without the
        /// marks.
        Html = "html",
        UseOutlines = "use_outlines",
    }
}

words! {
    /// Which side of a text field its lines are set against.
    Align {
        Left = "left",
        Right = "right",
        Center = "center",
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextLayout {
    pub align: Align,
    pub left_margin: f64,
    pub right_margin: f64,
    pub indent: f64,
    pub leading: f64,
}
