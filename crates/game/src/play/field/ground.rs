//! The ground as the overhead view draws it: where a point of the field
//! is on the screen, and how far off it is in feet.

use super::{distance, reach};
use crate::play::pitch::Point;

/// The fixed points of the field that a hit is placed by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ground {
    pub home: Point,
    /// How far down the field's picture the row is that hits are aimed
    /// along, and where on it the foul lines cross it, left and right.
    pub mark_y: f32,
    pub foul: (f32, f32),
    /// How far off the wall is, and second base, as [`reach`] measures.
    pub wall: f32,
    pub infield: f32,
    /// How many feet one of [`reach`]'s units is taken to be.
    pub feet_each: f32,
}

impl Default for Ground {
    /// The field as the art draws it.
    fn default() -> Ground {
        Ground {
            home: (240.8, 336.85),
            mark_y: 168.7,
            foul: (-54.65, 638.3),
            wall: 820.0,
            infield: 440.0,
            feet_each: 400.0 / 820.0,
        }
    }
}

impl Ground {
    /// How far across the field from home a point is: nought on the left
    /// foul line, one on the right, and outside those in foul ground.
    pub fn across(&self, at: Point) -> f32 {
        let up = self.home.1 - at.1;
        if up <= 0.0 {
            return 0.5;
        }
        let along = self.home.0 + (at.0 - self.home.0) * (self.home.1 - self.mark_y) / up;
        (along - self.foul.0) / (self.foul.1 - self.foul.0)
    }

    /// The point that far across the field and that far from home, as
    /// [`reach`] measures.
    pub fn point(&self, across: f32, far: f32) -> Point {
        let mark = (
            self.foul.0 + across * (self.foul.1 - self.foul.0),
            self.mark_y,
        );
        let way = distance(self.home, mark).max(0.001);
        let towards = ((mark.0 - self.home.0) / way, (mark.1 - self.home.1) / way);
        // Along a straight line reach grows evenly.
        let from = reach(self.home, self.home);
        let each = reach(
            self.home,
            (self.home.0 + towards.0, self.home.1 + towards.1),
        ) - from;
        let pixels = (far - from) / each;
        (
            self.home.0 + towards.0 * pixels,
            self.home.1 + towards.1 * pixels,
        )
    }

    /// How far from home a point is, in feet.
    pub fn feet(&self, at: Point) -> u32 {
        (reach(self.home, at) * self.feet_each).max(0.0).round() as u32
    }
}
