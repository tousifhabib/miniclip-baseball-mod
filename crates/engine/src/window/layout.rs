//! Works out how the stage sits in the window.

use crate::math::Matrix;

/// How a stage sits in a window of this size: the transform from stage
/// coordinates to window pixels, and the rectangle the stage covers. With
/// `centre_origin` it is the stage's own origin that goes to the middle of
/// that rectangle, where its top-left corner would have gone to the corner.
pub(super) fn stage_in_window(
    stage: &bb_format::Stage,
    centre_origin: bool,
    width: u32,
    height: u32,
) -> (Matrix, [u32; 4]) {
    let (stage_width, stage_height) = (stage.width as f32, stage.height as f32);
    // Fit the stage inside the window, centred, keeping its shape.
    let scale = (width as f32 / stage_width).min(height as f32 / stage_height);
    let (left, top) = (
        ((width as f32 - stage_width * scale) / 2.0).round(),
        ((height as f32 - stage_height * scale) / 2.0).round(),
    );
    let fit = Matrix::translate(left, top).then_inner(Matrix::scale(scale, scale));
    let base = if centre_origin {
        fit.then_inner(Matrix::translate(stage_width / 2.0, stage_height / 2.0))
    } else {
        fit
    };
    let rectangle = [
        left as u32,
        top as u32,
        ((stage_width * scale).round() as u32).min(width - left as u32),
        ((stage_height * scale).round() as u32).min(height - top as u32),
    ];
    (base, rectangle)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stage as wide and as high as the game's.
    fn stage() -> bb_format::Stage {
        bb_format::Stage {
            width: 590.0,
            height: 400.0,
            frame_rate: 30.0,
            frame_count: 1,
            background: None,
        }
    }

    #[test]
    fn a_stage_fills_a_window_of_its_own_size() {
        let (base, rectangle) = stage_in_window(&stage(), false, 590, 400);
        assert_eq!(base, Matrix::IDENTITY);
        assert_eq!(rectangle, [0, 0, 590, 400]);
    }

    #[test]
    fn a_stage_sits_in_the_middle_of_a_window_that_is_too_wide_for_it() {
        let (base, rectangle) = stage_in_window(&stage(), false, 1180, 400);
        assert_eq!(base, Matrix::translate(295.0, 0.0));
        assert_eq!(rectangle, [295, 0, 590, 400]);
    }

    #[test]
    fn a_stage_grows_or_shrinks_to_fit_and_keeps_its_shape() {
        // Twice as wide and two and a half times as high: there is room for
        // the stage at twice its size, with a hundred pixels over above it
        // and a hundred below.
        let (base, rectangle) = stage_in_window(&stage(), false, 1180, 1000);
        let twice = Matrix {
            a: 2.0,
            d: 2.0,
            ty: 100.0,
            ..Matrix::IDENTITY
        };
        assert_eq!(base, twice);
        assert_eq!(rectangle, [0, 100, 1180, 800]);
        // The pointer is carried the other way, from the window's pixels to
        // the stage's.
        let to_stage = base.inverse().unwrap();
        assert_eq!(to_stage.apply(590.0, 500.0), (295.0, 200.0));

        // Half as wide: the stage at half its size.
        let (base, rectangle) = stage_in_window(&stage(), false, 295, 400);
        let half = Matrix {
            a: 0.5,
            d: 0.5,
            ty: 100.0,
            ..Matrix::IDENTITY
        };
        assert_eq!(base, half);
        assert_eq!(rectangle, [0, 100, 295, 200]);
    }

    #[test]
    fn a_stage_is_put_on_whole_pixels() {
        // Eleven pixels over across: half of that is five and a half, which
        // is six.
        let (base, rectangle) = stage_in_window(&stage(), false, 601, 400);
        assert_eq!(base, Matrix::translate(6.0, 0.0));
        assert_eq!(rectangle, [6, 0, 590, 400]);
    }

    #[test]
    fn a_clip_looked_at_on_its_own_has_its_origin_in_the_middle() {
        let (base, rectangle) = stage_in_window(&stage(), true, 1180, 1000);
        let centred = Matrix {
            a: 2.0,
            d: 2.0,
            tx: 590.0,
            ty: 500.0,
            ..Matrix::IDENTITY
        };
        assert_eq!(base, centred);
        // What the stage covers is as it was.
        assert_eq!(rectangle, [0, 100, 1180, 800]);
    }
}
