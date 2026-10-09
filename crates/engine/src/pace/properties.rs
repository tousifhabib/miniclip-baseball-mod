use proptest::prelude::*;

use super::*;

/// How long a frame lasts in a game that plays this many a second.
fn frame_at(a_second: u32) -> Duration {
    Duration::from_nanos(1_000_000_000 / u64::from(a_second))
}

/// A redraw: how long it was in coming, and whether it came to
/// anything.
type Redraw = (Duration, bool);

/// A stretch of redraws of one kind, for a game whose frames last this
/// long: from a screen that keeps time with the game, give or take a
/// tenth, from one that draws whenever it likes, taking up to three
/// seconds over it, or asked of a window that is out of sight.
fn stretch(frame: Duration) -> impl Strategy<Value = Vec<Redraw>> {
    let steady =
        (1u32..=4, prop::collection::vec(0.9f64..1.1, 1..40)).prop_map(move |(every, wobbles)| {
            let drawn = |wobble: &f64| (frame.mul_f64(*wobble) / every, true);
            wobbles.iter().map(drawn).collect()
        });
    let any_time = || (0u64..3_000_000_000).prop_map(Duration::from_nanos);
    let unsteady = prop::collection::vec((any_time(), Just(true)), 1..10);
    let unseen = prop::collection::vec((any_time(), Just(false)), 1..10);
    prop_oneof![3 => steady, 1 => unsteady, 1 => unseen]
}

proptest! {
    #[test]
    fn whatever_the_redraws_do_no_more_than_a_few_frames_are_played_for_one(
        (frame, redraws) in (10u32..=240).prop_map(frame_at).prop_flat_map(|frame| {
            let stretches = prop::collection::vec(stretch(frame), 1..8);
            (Just(frame), stretches.prop_map(|stretches| stretches.concat()))
        }),
    ) {
        // Nothing here is to fall over either, whatever sums the times
        // lead to.
        let mut pace = Pace::new(frame);
        for (took, drawn) in redraws {
            let frames = pace.frames(took);
            prop_assert!(frames <= MOST_AT_ONCE, "{} frames at once", frames);
            if !drawn {
                pace.undrawn();
                prop_assert_eq!(pace.in_step(), None);
            }
            // In step, it is with a number of redraws that it counts.
            if let Some(every) = pace.in_step() {
                prop_assert!((1..=MOST_REDRAWS).contains(&every), "every {}", every);
            }
        }
    }

    #[test]
    fn going_by_the_clock_it_plays_every_whole_frame_of_the_time_gone_by_and_no_more(
        frame in (10u32..=240).prop_map(frame_at),
        // How long each redraw was in coming, in thousandths of a
        // frame. None is longer than the five frames that are the most
        // played for one, so nothing is skipped.
        took in prop::collection::vec(0u32..=5000, 1..200),
        hidden: bool,
    ) {
        // It goes by the clock while the window is out of sight, and
        // until it has seen enough redraws to judge the screen by.
        let redraws = if hidden { took.len() } else { ENOUGH - 1 };
        let mut pace = Pace::new(frame);
        let (mut gone, mut played) = (Duration::ZERO, 0);
        for thousandths in took.into_iter().take(redraws) {
            let elapsed = frame * thousandths / 1000;
            gone += elapsed;
            played += u128::from(pace.frames(elapsed));
            if hidden {
                pace.undrawn();
            }
            prop_assert_eq!(pace.in_step(), None);
            // What is left over is carried to the next redraw, so the
            // count is never out by even one.
            prop_assert_eq!(played, gone.as_nanos() / frame.as_nanos());
        }
    }
}
