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
use crate::play::field::distance;
use crate::play::pitch::Point;
use crate::play::{Parts, at};
use crate::rules::Rules;

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

/// The lie of the batting view against the field, read from the art's own
/// markers: the arcade game puts its target into the batting view by these.
struct Lie {
    /// The middle of the field across, and its far edge.
    middle: f32,
    back: f32,
    /// The middle of the batting view across, and the horizon in it.
    centre: f32,
    horizon: f32,
}

impl Lie {
    fn read(parts: &Parts, stage: &Stage) -> Lie {
        let mark = |from: &Path, name: &str| stage.find(from, &[name]).map(|path| at(stage, &path));
        Lie {
            middle: mark(&parts.field, "centreMarker").map_or(301.45, |at| at.0),
            back: mark(&parts.field, "bMarker").map_or(108.45, |at| at.1),
            centre: parts.centre_x,
            horizon: mark(&parts.main, "hMarker").map_or(176.1, |at| at.1),
        }
    }

    /// The place on the field that a point of the batting view is over.
    fn on_field(&self, seen: Point) -> Point {
        (
            self.middle + (seen.0 - self.centre) / 1.3,
            self.back + (seen.1 - self.horizon) / 0.3,
        )
    }

    /// How to draw something lying on the field at `place` in the batting
    /// view: smaller and flatter the further up the field it is.
    fn in_view(&self, place: Point, size: f32) -> Matrix {
        let (across, depth) = (place.0 - self.middle, place.1 - self.back);
        Matrix {
            a: size * (50.0 + depth) / 100.0,
            d: size * (10.0 + depth / 5.0) / 100.0,
            tx: self.centre + 1.3 * across,
            ty: self.horizon + 0.3 * depth,
            ..Matrix::IDENTITY
        }
    }
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
            let free = |depth: &u16| !clip.children.contains_key(depth);
            if up {
                (from + 1..).find(free)
            } else {
                (1..from).rev().find(free)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn lie() -> Lie {
        Lie {
            middle: 301.45,
            back: 108.45,
            centre: 295.0,
            horizon: 176.1,
        }
    }

    #[test]
    fn a_point_of_the_batting_view_is_over_one_place_on_the_field_and_back_again() {
        let lie = lie();
        for place in [(301.45, 108.45), (120.0, 150.0), (430.0, 215.5)] {
            let seen = lie.in_view(place, 1.0);
            let back = lie.on_field((seen.tx, seen.ty));
            assert!((back.0 - place.0).abs() < 0.01 && (back.1 - place.1).abs() < 0.01);
        }
        // The middle of the far edge of the field is on the horizon, in the
        // middle of the view.
        assert_eq!(lie.on_field((295.0, 176.1)), (301.45, 108.45));
    }

    #[test]
    fn a_target_is_drawn_smaller_and_flatter_further_up_the_field() {
        let lie = lie();
        let (far, near) = (
            lie.in_view((300.0, 130.0), 0.6),
            lie.in_view((300.0, 230.0), 0.6),
        );
        assert!(far.a < near.a && far.d < near.d);
        assert!(far.d < far.a);
        assert!(far.ty < near.ty);
    }
}
