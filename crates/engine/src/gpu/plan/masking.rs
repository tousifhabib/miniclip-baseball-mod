//! Which masks are in force as a frame's draws are planned.

use crate::gpu::pipelines::Mode;

/// Mask state, which starts afresh inside each layer.
#[derive(Clone, Copy)]
pub(super) struct Masking {
    /// How many masks are in force.
    depth: u32,
    pub(super) mode: Mode,
    pub(super) stencil: u32,
}

impl Masking {
    pub(super) const NONE: Masking = Masking {
        depth: 0,
        mode: Mode::Content,
        stencil: 0,
    };

    /// The draws that follow are the outline of a new mask.
    pub(super) fn push(&mut self) {
        self.mode = Mode::MaskWrite;
        self.stencil = self.depth;
        self.depth += 1;
    }

    /// The outline is done: from here on, only what is inside it is drawn.
    pub(super) fn activate(&mut self) {
        self.mode = Mode::Content;
        self.stencil = self.depth;
    }

    /// The draws that follow repeat the innermost mask's outline, to take
    /// it away.
    pub(super) fn deactivate(&mut self) {
        self.mode = Mode::MaskClear;
        self.stencil = self.depth;
    }

    /// The innermost mask has gone.
    pub(super) fn pop(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        self.mode = Mode::Content;
        self.stencil = self.depth;
    }
}
