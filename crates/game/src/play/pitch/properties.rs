use proptest::prelude::*;

use super::*;
use crate::play::pitch::tests::mound;
use crate::rules::Rules;

/// A pitch such as might be thrown: quick or slow, curving and dropping
/// a little or not at all, at the strike zone or somewhere about it.
pub(crate) fn any_choice() -> impl Strategy<Value = Choice> {
    let speed = 20.0f32..120.0;
    let (swing, dip) = (-2.0f32..2.0, 0.0f32..1.5);
    let aim = (200.0f32..400.0, 150.0f32..350.0);
    (speed, swing, dip, aim).prop_map(|(speed, swing, dip, aim)| Choice {
        speed,
        swing,
        dip,
        aim,
    })
}

/// A timing window such as a file of rules might give: a few frames
/// after the swing, not always one after another, each met well or
/// badly and with one of a few powers. It may have no frames at all.
pub(crate) fn any_window() -> impl Strategy<Value = Vec<(u32, Quality, f32)>> {
    let quality = prop::sample::select(
        &[
            Quality::Poor,
            Quality::MediumPoor,
            Quality::Medium,
            Quality::Good,
        ][..],
    );
    let power = prop::sample::select(&[14.0f32, 15.0, 17.0, 18.0, 25.0][..]);
    prop::collection::btree_map(0u32..30, (quality, power), 0..8).prop_map(|window| {
        let frame = |(frames, (quality, power))| (frames, quality, power);
        window.into_iter().map(frame).collect()
    })
}

/// The easy level's table, with this window in place of its own.
fn table_with(window: Vec<(u32, Quality, f32)>) -> PitchRules {
    PitchRules {
        window,
        ..Rules::default().pitch.easy
    }
}

proptest! {
    #[test]
    fn any_pitch_crosses_where_its_ball_is_as_its_shadow_comes_to_the_plate_and_then_fades_out(
        choice in any_choice(),
    ) {
        let (rules, mound) = (Rules::default(), mound());
        let pitch = Pitch::throw(&choice, &mound, &rules.throw);
        let samples = &pitch.samples;
        // It ends by itself, long before the guard against a pitch that
        // never comes in.
        prop_assert!(samples.len() < 3000, "{} frames", samples.len());
        prop_assert_eq!(samples.last().map(|sample| sample.alpha), Some(0.0));
        // The shadow only ever comes on, and the ball only ever grows.
        for pair in samples.windows(2) {
            prop_assert!(pair[0].shadow.1 <= pair[1].shadow.1, "{:?}", pair);
            prop_assert!(pair[0].size <= pair[1].size, "{:?}", pair);
        }
        let at_plate = samples
            .iter()
            .position(|sample| sample.shadow.1 >= mound.plate)
            .expect("a frame with the shadow at the plate");
        // Up to there the ball is solid, and from there on it is not.
        for (step, sample) in samples.iter().enumerate() {
            prop_assert_eq!(sample.alpha == 1.0, step < at_plate, "step {}", step);
        }
        let ball = samples[at_plate].ball;
        prop_assert_eq!(pitch.crosses, ball);
        let [left, top, right, bottom] = mound.zone;
        let inside = (left..=right).contains(&ball.0) && (top..=bottom).contains(&ball.1);
        prop_assert_eq!(pitch.in_zone, inside, "crossing at {:?}", ball);
    }

    #[test]
    fn widening_a_window_by_nothing_changes_nothing_and_in_two_goes_is_the_same_as_in_one(
        window in any_window(),
        first in 0u32..6,
        then in 0u32..6,
    ) {
        prop_assert_eq!(widened(&window, 0), window.clone());
        let in_two = widened(&widened(&window, first), then);
        prop_assert_eq!(in_two, widened(&window, first + then));
    }

    #[test]
    fn a_wider_window_meets_the_ball_as_the_old_one_did_and_past_its_ends_as_the_ends_did(
        window in any_window(),
        more in 0u32..6,
    ) {
        let wider = table_with(widened(&window, more));
        // The window's frames are listed from the first to the last.
        let (first, last) = (window.first().copied(), window.last().copied());
        let narrow = table_with(window);
        for frames in 0..40 {
            let wanted = match (first, last) {
                (Some((first, quality, power)), _) if frames < first => {
                    (first - frames <= more).then_some((quality, power))
                }
                (_, Some((last, quality, power))) if frames > last => {
                    (frames - last <= more).then_some((quality, power))
                }
                // From the one end to the other it is as it was: a gap
                // in the window is a gap still.
                _ => meets(&narrow, frames),
            };
            prop_assert_eq!(meets(&wider, frames), wanted, "{} frames after", frames);
        }
    }

    #[test]
    fn a_swing_is_somewhere_from_the_worst_to_the_best_exactly_when_it_meets_the_ball(
        window in any_window(),
    ) {
        let table = table_with(window);
        let top = table.window.iter().map(|&(_, quality, _)| quality).max();
        let mut best = 0;
        for frames in 0..32 {
            let near = nearness(&table, frames);
            let met = meets(&table, frames);
            prop_assert_eq!(near.is_some(), met.is_some(), "{} frames after", frames);
            if let (Some(near), Some((quality, _))) = (near, met) {
                prop_assert!((0.0..=1.0).contains(&near), "{}", near);
                // Only a frame that meets the ball as well as any does
                // is the best.
                if near == 1.0 {
                    prop_assert_eq!(Some(quality), top);
                    best += 1;
                }
            }
        }
        // And a window with any frames at all has one.
        prop_assert_eq!(best > 0, !table.window.is_empty());
    }
}
