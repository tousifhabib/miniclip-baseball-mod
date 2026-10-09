//! The called shot: a mod that lets the batter name where his hit will
//! come down.
//!
//! Before a pitch, while the pitcher stands and waits, a click on the
//! outfield puts a target there. A hit that first comes down on the target
//! is worth runs on top of whatever it earns by the usual way, more of
//! them the nearer the middle. The target is the arcade game's, drawn
//! smaller, and is measured the same way: it lies flat on the grass, so a
//! miss up or down the field counts for more than one to the side.

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

use crate::art;
use crate::mods::About;
use crate::play::field::distance;
use crate::play::pitch::Point;
use crate::play::view::Lie;
use crate::play::{Parts, overlay};
use crate::rules::Rules;

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "called_shot",
    name: "CALLED SHOT",
    does: "CLICK THE OUTFIELD BEFORE A PITCH: LAND IT THERE FOR RUNS",
    setting: None,
};

/// The mod, in play. A shot is called afresh for each pitch, so nothing is
/// kept between them.
pub(crate) struct CalledShot;

/// A shot that has been called.
pub(crate) struct Called {
    /// Where on the field the target is.
    pub at: Point,
    /// The target as it is seen from behind the batter, and from over the
    /// field.
    in_view: Path,
    on_field: Path,
    /// Where the hit came down, once it has, and the runs that was worth.
    pub down: Option<Point>,
    pub runs: u32,
}

impl Called {
    /// Calls the shot for the place on the outfield under `pointer`, a
    /// point of the batting view, or moves a call already made to there.
    /// A click off the part of the outfield that can be called is taken as
    /// the nearest place on it.
    pub fn call(
        called: &mut Option<Called>,
        pointer: Point,
        parts: &Parts,
        rules: &Rules,
        stage: &mut Stage,
        library: &Library,
    ) {
        let lie = Lie::read(parts, stage);
        let area = &rules.called_shot.area;
        let place = lie.on_field(pointer);
        let place = (
            place.0.clamp(area.x, area.x + area.width),
            place.1.clamp(area.y, area.y + area.height),
        );
        let size = rules.called_shot.size;
        if called.is_none() {
            *called = Called::put_out(place, parts, stage, library);
        }
        let Some(called) = called else {
            return;
        };
        called.at = place;
        if let Some(target) = stage.child_mut(&called.in_view) {
            target.set_matrix(lie.in_view(place, size));
        }
        if let Some(target) = stage.child_mut(&called.on_field) {
            target.set_matrix(Matrix {
                a: size,
                d: size,
                tx: place.0,
                ty: place.1,
                ..Matrix::IDENTITY
            });
        }
    }

    /// Puts the two targets on the stage: one behind the players in the
    /// batting view, and one on the field under the ball.
    fn put_out(
        place: Point,
        parts: &Parts,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Called> {
        let free_from = |clip: &Path, from: u16, up: bool, stage: &Stage| {
            let clip = stage.clip(clip)?;
            if up {
                overlay::free_above(clip, from)
            } else {
                overlay::free_below(clip, from)
            }
        };
        // Just over the scoreboard, which is behind everybody.
        let board = parts
            .scoreboard
            .as_ref()
            .and_then(|path| path.last().copied())
            .unwrap_or(4);
        let depth = free_from(&parts.main, board, true, stage)?;
        let in_view = stage.attach(&parts.main, art::TARGET, depth, "calledShot", library)?;
        let (&ball, _) = parts.field_ball.split_last()?;
        let depth = free_from(&parts.field, ball, false, stage)?;
        let on_field = stage.attach(&parts.field, art::TARGET, depth, "calledShot", library)?;
        Some(Called {
            at: place,
            in_view,
            on_field,
            down: None,
            runs: 0,
        })
    }

    /// The hit has come down at `ball`: works out the runs that is worth,
    /// lights the ring of the target it came down in, and returns the runs.
    pub fn landed(
        &mut self,
        ball: Point,
        rules: &Rules,
        stage: &mut Stage,
        library: &Library,
    ) -> u32 {
        self.down = Some(ball);
        let shot = &rules.called_shot;
        // As the arcade game measures it, on a target this much smaller.
        let off = distance(ball, self.at) + rules.arcade.depth_weight * (ball.1 - self.at.1).abs();
        let ring = rules
            .arcade
            .rings
            .iter()
            .position(|ring| off <= ring.within * shot.size);
        let Some(ring) = ring else {
            return 0;
        };
        self.runs = shot.runs.get(ring).copied().unwrap_or(0);
        // The art numbers its rings from the outside in.
        let name = format!("ring{}", rules.arcade.rings.len() - ring);
        if let Some(lit) = stage.find(&self.on_field, &[&name]) {
            stage.goto_clip(&lit, 2, library);
        }
        self.runs
    }
}
