//! The view a pitch is played in: where its parts are on the stage, the
//! small things done to them over and over, and what is written and drawn
//! over them.

mod cues;
pub(crate) mod overlay;
mod parts;

use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

pub(crate) use cues::Cue;
pub(crate) use parts::Parts;

use super::pitch::Point;

/// Where an object is.
pub(crate) fn at(stage: &Stage, path: &[u16]) -> Point {
    stage
        .child(path)
        .map_or((0.0, 0.0), |child| (child.matrix.tx, child.matrix.ty))
}

/// Puts an object at a point, at a size, the art's own size being 1.
pub(crate) fn put(stage: &mut Stage, path: &[u16], at: Point, size: f32) {
    if let Some(child) = stage.child_mut(path) {
        child.set_matrix(Matrix {
            a: size,
            d: size,
            tx: at.0,
            ty: at.1,
            ..Matrix::IDENTITY
        });
    }
}

pub(crate) fn show(stage: &mut Stage, path: &[u16], visible: bool) {
    if let Some(child) = stage.child_mut(path) {
        child.set_visible(visible);
    }
}

/// The frame a clip is on. Nought, if it is not there.
pub(crate) fn frame_of(stage: &Stage, path: &[u16]) -> u16 {
    stage.clip(path).map_or(0, |clip| clip.frame)
}

/// Sends a clip to a frame and sets it playing from there.
pub(crate) fn play_from(stage: &mut Stage, path: &[u16], frame: u16, library: &Library) {
    stage.goto_clip(path, frame, library);
    if let Some(clip) = stage.clip_mut(path) {
        clip.playing = true;
    }
}
