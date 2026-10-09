//! What the graphics card is set up with before anything is drawn: the
//! shader, and what it is told for each layer and each draw.

use bytemuck::{Pod, Zeroable};

pub(super) const SHADER: &str = include_str!("shader.wgsl");

/// What the shader is told once for each layer.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct Globals {
    pub view: [f32; 4],
    pub limits: [f32; 4],
}

/// What the shader is told for each draw, and for each blur.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct Item {
    pub world_abcd: [f32; 4],
    pub world_t: [f32; 4],
    pub color_mult: [f32; 4],
    pub color_add: [f32; 4],
    pub paint_abcd: [f32; 4],
    pub paint_t: [f32; 4],
    pub kind: [u32; 4],
}

/// How a draw treats the stencil buffer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// Draws colour where the stencil matches.
    Content = 0,
    /// Raises the stencil where it matches, drawing no colour.
    MaskWrite = 1,
    /// Lowers the stencil where it matches, drawing no colour.
    MaskClear = 2,
}
