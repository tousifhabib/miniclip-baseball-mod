//! How the batting view lies against the field: which place on the field a
//! point of the view is over, and how a thing lying on the field is drawn
//! in the view.

use bb_engine::display::Path;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

use super::{Parts, at};
use crate::play::pitch::Point;

/// The lie of the batting view against the field, read from the art's own
/// markers. The arcade game puts its target into the batting view by it,
/// and a called shot's the same way.
pub(crate) struct Lie {
    /// The middle of the field across, and its far edge.
    middle: f32,
    back: f32,
    /// The middle of the batting view across, and the horizon in it.
    centre: f32,
    horizon: f32,
}

impl Lie {
    pub fn read(parts: &Parts, stage: &Stage) -> Lie {
        let mark = |from: &Path, name: &str| stage.find(from, &[name]).map(|path| at(stage, &path));
        Lie {
            middle: mark(&parts.field, "centreMarker").map_or(301.45, |at| at.0),
            back: mark(&parts.field, "bMarker").map_or(108.45, |at| at.1),
            centre: parts.centre_x,
            horizon: mark(&parts.main, "hMarker").map_or(176.1, |at| at.1),
        }
    }

    /// The place on the field that a point of the batting view is over.
    pub fn on_field(&self, seen: Point) -> Point {
        (
            self.middle + (seen.0 - self.centre) / 1.3,
            self.back + (seen.1 - self.horizon) / 0.3,
        )
    }

    /// How to draw something lying on the field at `place` in the batting
    /// view: smaller and flatter the further up the field it is.
    pub fn in_view(&self, place: Point, size: f32) -> Matrix {
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
