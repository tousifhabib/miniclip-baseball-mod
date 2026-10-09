use proptest::prelude::*;

use super::*;
use crate::play::pitch::properties::{any_choice, any_window};
use crate::play::pitch::tests::mound;
use crate::play::pitch::{Choice, meets};
use crate::rules::Rules;
use crate::settings::Difficulty;

/// A pitch down the middle at this skill level's slowest.
fn pitch(rules: &Rules, difficulty: Difficulty) -> Pitch {
    let choice = Choice {
        speed: rules.pitch.at(difficulty).speed.high as f32,
        swing: 0.0,
        dip: 0.0,
        aim: (295.0, 250.0),
    };
    Pitch::throw(&choice, &mound(), &rules.throw)
}

const LEVELS: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];

#[test]
fn a_swing_meets_the_ball_in_the_band_at_a_point_in_its_window() {
    let rules = Rules::default();
    for difficulty in LEVELS {
        let table = rules.pitch.at(difficulty);
        let pitch = pitch(&rules, difficulty);
        let mut met = 0;
        for step in 0..pitch.samples.len() {
            let Some((on, quality, power)) = pitch.swing_from(table, step) else {
                continue;
            };
            met += 1;
            assert!(pitch.samples[on].in_band(table.band));
            assert_eq!(meets(table, (on - step) as u32), Some((quality, power)));
            // And on no frame before that one.
            for earlier in step..on {
                let frames = (earlier - step) as u32;
                assert!(
                    !pitch.samples[earlier].in_band(table.band) || meets(table, frames).is_none()
                );
            }
        }
        assert!(met > 0, "{difficulty:?}");
    }
}

#[test]
fn the_steps_that_meet_the_ball_come_in_one_run_near_the_end() {
    let rules = Rules::default();
    for difficulty in LEVELS {
        let pitch = pitch(&rules, difficulty);
        let timing = Timing::of(&pitch, rules.pitch.at(difficulty));
        let stretches = timing.stretches();
        let (first, last) = (stretches[0].0, stretches.last().unwrap().1);
        for pair in stretches.windows(2) {
            assert_eq!(pair[0].1 + 1, pair[1].0, "{difficulty:?}: {stretches:?}");
        }
        assert!(first > pitch.samples.len() / 2, "{difficulty:?}");
        assert_eq!(timing.at(first - 1), None);
        assert_eq!(timing.at(last + 1), None);
        // Past the end of the pitch there is nothing to meet.
        assert_eq!(timing.at(pitch.samples.len()), None);
    }
}

#[test]
fn the_best_moment_is_the_good_one_and_it_is_shorter_the_harder_the_level() {
    let rules = Rules::default();
    let best = |difficulty| {
        let pitch = pitch(&rules, difficulty);
        let timing = Timing::of(&pitch, rules.pitch.at(difficulty));
        let (first, last) = timing.best().unwrap();
        for step in first..=last {
            assert_eq!(timing.at(step), Some(Quality::Good));
        }
        last - first + 1
    };
    assert!(best(Difficulty::Easy) > best(Difficulty::Hard));
    assert_eq!(best(Difficulty::Hard), 1);
}

#[test]
fn a_swing_is_judged_against_the_best_moment() {
    let rules = Rules::default();
    let pitch = pitch(&rules, Difficulty::Easy);
    let timing = Timing::of(&pitch, rules.pitch.at(Difficulty::Easy));
    let (first, last) = timing.best().unwrap();
    let stretches = timing.stretches();
    let (start, end) = (stretches[0].0, stretches.last().unwrap().1);
    assert_eq!(timing.verdict(first), Some(Verdict::Perfect));
    assert_eq!(timing.verdict(last), Some(Verdict::Perfect));
    assert_eq!(timing.verdict(0), Some(Verdict::TooEarly));
    assert_eq!(timing.verdict(end + 1), Some(Verdict::TooLate));
    // On easy a swing can be a little out either way and still meet
    // the ball.
    assert!(start < first && last < end);
    assert_eq!(timing.verdict(start), Some(Verdict::Early));
    assert_eq!(timing.verdict(end), Some(Verdict::Late));
}

#[test]
fn a_pitch_no_swing_can_meet_has_no_best_moment() {
    let rules = Rules::default();
    let mut table = rules.pitch.at(Difficulty::Easy).clone();
    table.window.clear();
    let timing = Timing::of(&pitch(&rules, Difficulty::Easy), &table);
    assert_eq!(timing.best(), None);
    assert!(timing.stretches().is_empty());
    assert_eq!(timing.verdict(10), None);
}

/// Any pitch, and a table to swing at it by: one of the levels' own,
/// with any window in place of its own.
fn any_pitch() -> impl Strategy<Value = (Pitch, PitchRules)> {
    let level = prop::sample::select(&LEVELS[..]);
    (any_choice(), any_window(), level).prop_map(|(choice, window, level)| {
        let rules = Rules::default();
        let table = PitchRules {
            window,
            ..rules.pitch.at(level).clone()
        };
        (Pitch::throw(&choice, &mound(), &rules.throw), table)
    })
}

proptest! {
    #[test]
    fn a_step_is_coloured_on_the_bar_exactly_when_a_swing_begun_on_it_would_meet_the_ball(
        (pitch, table) in any_pitch(),
    ) {
        let timing = Timing::of(&pitch, &table);
        let stretches = timing.stretches();
        // The steps of the pitch, and a couple past the end of it.
        for step in 0..pitch.samples.len() + 2 {
            let swing = pitch.swing_from(&table, step);
            let met = swing.map(|(_, quality, _)| quality);
            // The colours the bar has for the step: one, and the right
            // one, or none.
            let coloured: Vec<Quality> = stretches
                .iter()
                .filter(|&&(first, last, _)| (first..=last).contains(&step))
                .map(|&(.., quality)| quality)
                .collect();
            prop_assert_eq!(coloured, Vec::from_iter(met), "step {}", step);
            prop_assert_eq!(timing.at(step), met);
            let frames = swing.map(|(on, ..)| (on - step) as u32);
            prop_assert_eq!(timing.frames(step), frames);
        }
        // Each stretch of colour is as long as it can be: the next is
        // further on, and if it touches it is of another colour.
        for pair in stretches.windows(2) {
            let ((_, last, colour), (first, _, next)) = (pair[0], pair[1]);
            prop_assert!(last < first && (last + 1 < first || colour != next), "{:?}", pair);
        }
    }

    #[test]
    fn the_best_moment_is_the_first_of_the_longest_stretches_that_meet_the_ball_best(
        (pitch, table) in any_pitch(),
    ) {
        let timing = Timing::of(&pitch, &table);
        let stretches = timing.stretches();
        let Some((first, last)) = timing.best() else {
            // No best moment is no moment at all.
            prop_assert!(stretches.is_empty(), "{:?}", stretches);
            return Ok(());
        };
        let top = stretches.iter().map(|&(.., quality)| quality).max();
        prop_assert!(top.is_some_and(|top| stretches.contains(&(first, last, top))));
        for (from, to, quality) in stretches {
            if Some(quality) == top {
                // None as good is longer, and none as long is sooner.
                prop_assert!(to - from <= last - first);
                prop_assert!(to - from < last - first || from >= first);
            }
        }
    }
}
