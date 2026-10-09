//! 2D transforms and colour transforms.

use bb_format as f;

/// A 2D affine transform: `x' = a*x + c*y + tx`, `y' = b*x + d*y + ty`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub tx: f32,
    pub ty: f32,
}

impl Matrix {
    pub const IDENTITY: Matrix = Matrix {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    pub fn translate(tx: f32, ty: f32) -> Matrix {
        Matrix {
            tx,
            ty,
            ..Matrix::IDENTITY
        }
    }

    pub fn scale(sx: f32, sy: f32) -> Matrix {
        Matrix {
            a: sx,
            d: sy,
            ..Matrix::IDENTITY
        }
    }

    /// The transform that applies `inner` first and then `self`.
    pub fn then_inner(self, inner: Matrix) -> Matrix {
        Matrix {
            a: self.a * inner.a + self.c * inner.b,
            b: self.b * inner.a + self.d * inner.b,
            c: self.a * inner.c + self.c * inner.d,
            d: self.b * inner.c + self.d * inner.d,
            tx: self.a * inner.tx + self.c * inner.ty + self.tx,
            ty: self.b * inner.tx + self.d * inner.ty + self.ty,
        }
    }

    pub fn apply(self, x: f32, y: f32) -> (f32, f32) {
        (
            self.a * x + self.c * y + self.tx,
            self.b * x + self.d * y + self.ty,
        )
    }

    /// `None` when the transform squashes everything onto a line or point.
    pub fn inverse(self) -> Option<Matrix> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < 1e-12 {
            return None;
        }
        let (a, b, c, d) = (self.d / det, -self.b / det, -self.c / det, self.a / det);
        Some(Matrix {
            a,
            b,
            c,
            d,
            tx: -(a * self.tx + c * self.ty),
            ty: -(b * self.tx + d * self.ty),
        })
    }
}

impl From<f::Matrix> for Matrix {
    fn from(m: f::Matrix) -> Matrix {
        Matrix {
            a: m[0] as f32,
            b: m[1] as f32,
            c: m[2] as f32,
            d: m[3] as f32,
            tx: m[4] as f32,
            ty: m[5] as f32,
        }
    }
}

/// Each channel becomes `channel * mult + add`, with channels from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorTransform {
    pub mult: [f32; 4],
    pub add: [f32; 4],
}

impl ColorTransform {
    pub const IDENTITY: ColorTransform = ColorTransform {
        mult: [1.0; 4],
        add: [0.0; 4],
    };

    /// The transform that applies `inner` first and then `self`.
    pub fn then_inner(self, inner: ColorTransform) -> ColorTransform {
        let mut out = ColorTransform::IDENTITY;
        for i in 0..4 {
            out.mult[i] = self.mult[i] * inner.mult[i];
            out.add[i] = self.mult[i] * inner.add[i] + self.add[i];
        }
        out
    }
}

impl From<f::ColorTransform> for ColorTransform {
    fn from(c: f::ColorTransform) -> ColorTransform {
        ColorTransform {
            mult: c.mult.map(|m| m as f32),
            add: c.add.map(|a| f32::from(a) / 255.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4
    }

    #[test]
    fn then_inner_applies_the_inner_transform_first() {
        let outer = Matrix::translate(10.0, 0.0);
        let inner = Matrix::scale(2.0, 2.0);
        // Scale (1, 1) to (2, 2), then move it right by 10.
        assert!(close(outer.then_inner(inner).apply(1.0, 1.0), (12.0, 2.0)));
        // The other way round: move to (11, 1), then scale.
        assert!(close(inner.then_inner(outer).apply(1.0, 1.0), (22.0, 2.0)));
    }

    #[test]
    fn inverse_undoes_a_transform() {
        let m = Matrix {
            a: 2.0,
            b: 0.5,
            c: -1.0,
            d: 3.0,
            tx: 7.0,
            ty: -4.0,
        };
        let (x, y) = m.apply(3.0, 5.0);
        assert!(close(m.inverse().unwrap().apply(x, y), (3.0, 5.0)));
    }

    #[test]
    fn a_flat_transform_has_no_inverse() {
        assert!(Matrix::scale(0.0, 1.0).inverse().is_none());
    }

    #[test]
    fn color_transforms_compose_inner_first() {
        let halve = ColorTransform {
            mult: [0.5; 4],
            add: [0.0; 4],
        };
        let brighten = ColorTransform {
            mult: [1.0; 4],
            add: [0.2; 4],
        };
        // Brighten then halve: (c + 0.2) * 0.5.
        let both = halve.then_inner(brighten);
        assert_eq!(both.mult, [0.5; 4]);
        assert_eq!(both.add, [0.1; 4]);
    }

    /// A transform that turns, stretches and slants things by no more than
    /// four times, and moves them no further than a stage or so.
    fn matrix() -> impl Strategy<Value = Matrix> {
        let part = -4.0f32..4.0;
        let moved = -500.0f32..500.0;
        (
            part.clone(),
            part.clone(),
            part.clone(),
            part,
            moved.clone(),
            moved,
        )
            .prop_map(|(a, b, c, d, tx, ty)| Matrix { a, b, c, d, tx, ty })
    }

    /// A point on a stage or not far off it.
    fn point() -> impl Strategy<Value = (f32, f32)> {
        (-500.0f32..500.0, -500.0f32..500.0)
    }

    /// How many times its own area a square covers once a transform has
    /// moved it.
    fn area(m: Matrix) -> f32 {
        (m.a * m.d - m.b * m.c).abs()
    }

    /// Whether two points are within a tenth of a pixel of each other.
    fn a_tenth_apart_at_most(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 0.1 && (a.1 - b.1).abs() < 0.1
    }

    /// A colour transform that does no more than double a channel or turn
    /// it inside out, and adds or takes away no more than all of it.
    fn colour_transform() -> impl Strategy<Value = ColorTransform> {
        let mult = prop::array::uniform4(-2.0f32..2.0);
        let add = prop::array::uniform4(-1.0f32..1.0);
        (mult, add).prop_map(|(mult, add)| ColorTransform { mult, add })
    }

    /// What a colour transform makes of a colour: each channel times its
    /// `mult`, plus its `add`.
    fn tinted(by: ColorTransform, colour: [f32; 4]) -> [f32; 4] {
        std::array::from_fn(|channel| colour[channel] * by.mult[channel] + by.add[channel])
    }

    proptest! {
        #[test]
        fn the_inverse_of_a_transform_puts_any_point_back_to_within_a_tenth_of_a_pixel(
            // One that squashes things nearly flat has an inverse that
            // blows every small error up, so those are left out.
            m in matrix().prop_filter("it squashes things nearly flat", |m| area(*m) >= 0.5),
            (x, y) in point(),
        ) {
            let (moved_x, moved_y) = m.apply(x, y);
            let inverse = m.inverse().expect("a transform that keeps half a square's area");
            let back = inverse.apply(moved_x, moved_y);
            prop_assert!(a_tenth_apart_at_most(back, (x, y)), "it came back as {:?}", back);
        }

        #[test]
        fn a_transform_that_squashes_everything_onto_a_line_has_no_inverse(
            m in matrix(),
            // Doubling and halving are exact, where other sums are rounded.
            times in prop::sample::select(&[-2.0f32, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0][..]),
        ) {
            // Whatever goes across comes out along the same line as whatever
            // goes down: this many times as far along it.
            let flat = Matrix {
                c: m.a * times,
                d: m.b * times,
                ..m
            };
            prop_assert_eq!(flat.inverse(), None);
        }

        #[test]
        fn only_a_transform_that_leaves_a_square_next_to_no_area_has_no_inverse(
            // Squares shrunk to between a hundred millionth of their width
            // and a ten thousandth, which is about as many to one side of
            // the line as to the other. They may be slanted and moved too.
            shrunk in (-8.0f32..-4.0).prop_map(|tens| 10.0f32.powf(tens)),
            slant in -1.0f32..1.0,
            (tx, ty) in point(),
        ) {
            let m = Matrix {
                a: shrunk,
                b: 0.0,
                c: shrunk * slant,
                d: shrunk,
                tx,
                ty,
            };
            // The line is at a million millionth of the square's area.
            prop_assert_eq!(m.inverse().is_none(), area(m) < 1e-12);
            // The inverse of one that is all but flat is still made of
            // numbers.
            if let Some(Matrix { a, b, c, d, tx, ty }) = m.inverse() {
                prop_assert!([a, b, c, d, tx, ty].iter().all(|part| part.is_finite()));
            }
        }

        #[test]
        fn doing_nothing_before_or_after_a_transform_leaves_it_exactly_as_it_was(m in matrix()) {
            prop_assert_eq!(m.then_inner(Matrix::IDENTITY), m);
            prop_assert_eq!(Matrix::IDENTITY.then_inner(m), m);
        }

        #[test]
        fn two_transforms_made_into_one_move_a_point_as_the_two_would_in_turn(
            outer in matrix(),
            inner in matrix(),
            (x, y) in point(),
        ) {
            let (inner_x, inner_y) = inner.apply(x, y);
            let in_turn = outer.apply(inner_x, inner_y);
            let at_once = outer.then_inner(inner).apply(x, y);
            prop_assert!(
                a_tenth_apart_at_most(at_once, in_turn),
                "{:?} at once and {:?} in turn",
                at_once,
                in_turn
            );
        }

        #[test]
        fn doing_nothing_before_or_after_a_colour_transform_leaves_it_exactly_as_it_was(
            c in colour_transform(),
        ) {
            prop_assert_eq!(c.then_inner(ColorTransform::IDENTITY), c);
            prop_assert_eq!(ColorTransform::IDENTITY.then_inner(c), c);
        }

        #[test]
        fn two_colour_transforms_made_into_one_change_a_colour_as_the_two_would_in_turn(
            outer in colour_transform(),
            inner in colour_transform(),
            colour in prop::array::uniform4(0.0f32..=1.0),
        ) {
            let in_turn = tinted(outer, tinted(inner, colour));
            let at_once = tinted(outer.then_inner(inner), colour);
            for channel in 0..4 {
                prop_assert!(
                    (at_once[channel] - in_turn[channel]).abs() < 1e-4,
                    "{:?} at once and {:?} in turn",
                    at_once,
                    in_turn
                );
            }
        }
    }
}
