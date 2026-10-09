use proptest::prelude::*;

use super::*;

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
