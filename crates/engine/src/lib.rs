//! Plays the extracted game: a Flash-style tree of clips with timelines,
//! drawn with wgpu.

// Outside the tests nothing is taken for granted: what may be missing is
// dealt with, or the reason it cannot be is given.
#![warn(clippy::unwrap_used)]

pub mod app;
pub mod audio;
pub mod display;
pub mod gpu;
pub mod input;
pub mod inspector;
pub mod library;
pub mod math;
pub mod meshes;
pub mod pace;
pub mod stage;
pub mod tess;
#[cfg(test)]
pub(crate) mod testing;
pub mod window;
