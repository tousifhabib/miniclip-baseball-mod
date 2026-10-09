//! How far a zinger goes, and what of the swing decides it.

use super::{SPOILT, feet, game};
use crate::common::{ON, pitch};
use crate::timed_to_the_run;

/// The furthest a zinger goes at the skill level these games are played
/// at, and the least any goes.
const MOST: u32 = 850;
const LEAST: u32 = 440;

#[test]
fn every_ball_the_bat_meets_is_a_home_run() {
    for seed in [1, 2, 3] {
        // From the earliest swing that meets the ball to one of the latest.
        for late in -1..=4 {
            let Some(mut script) = game("match", seed, true) else {
                return;
            };
            let after = pitch(&mut script, late, ON);
            assert!(after.contains("score 1 of"), "seed {seed}, {late}: {after}");
            assert!(after.contains("outs 0, count 0-0, bases ---"), "{after}");
            assert!(feet(&after).is_some(), "seed {seed}, {late}: {after}");
        }
    }
}

#[test]
fn it_is_a_home_run_wherever_the_ring_was_held() {
    for (seed, off) in SPOILT {
        let Some(mut usual) = game("match", seed, false) else {
            return;
        };
        // As the game was, none of these gets a run.
        let after = pitch(&mut usual, 0, off);
        assert!(after.contains("score 0 of"), "{off:?}: {after}");
        assert_eq!(feet(&after), None);

        let mut zinger = game("match", seed, true).unwrap();
        let after = pitch(&mut zinger, 0, off);
        assert!(after.contains("score 1 of"), "{off:?}: {after}");
        assert!(after.contains("outs 0, count 0-0, bases ---"), "{after}");
    }
}

#[test]
fn the_better_the_swing_is_timed_the_further_the_ball_goes() {
    for seed in [1, 2, 3] {
        // Later and later after the best moment, as far as the last swing
        // that still meets the ball.
        let mut went = Vec::new();
        for late in 0..=4 {
            let Some(mut script) = game("match", seed, true) else {
                return;
            };
            went.push(feet(&pitch(&mut script, late, ON)).unwrap());
        }
        assert!(went.is_sorted_by(|a, b| a >= b), "seed {seed}: {went:?}");
        // From the furthest there is to the least a ring on the ball gets.
        assert!(
            went[0] > MOST - 5 && went[0] <= MOST,
            "seed {seed}: {went:?}"
        );
        assert!(went[4] > LEAST && went[4] <= LEAST + 60, "{went:?}");
        assert!(
            went[0] > went[2] && went[2] > went[4],
            "seed {seed}: {went:?}"
        );

        // Too early by the same amount is no better than too late.
        let mut script = game("match", seed, true).unwrap();
        let early = feet(&pitch(&mut script, -1, ON)).unwrap();
        assert!(early < went[0], "seed {seed}: {early}");
    }
}

#[test]
fn holding_the_ring_on_the_ball_adds_a_little_and_never_as_much_as_timing() {
    let went = |late, off| {
        let mut script = game("match", 1, true)?;
        feet(&pitch(&mut script, late, off))
    };
    let Some(on) = went(0, ON) else {
        return;
    };
    let off = went(0, (-40.0, 0.0)).unwrap();
    let far_off = went(0, (-140.0, 0.0)).unwrap();
    assert!(on > off && off > far_off, "{on} {off} {far_off}");
    assert_eq!(on - far_off, 60);
    // The best-timed swing with the ring nowhere near still beats one a
    // little late with the ring on the ball.
    assert!(far_off > went(2, ON).unwrap());
    // And the least there is comes of the worst of both.
    assert_eq!(went(3, (-140.0, 0.0)), Some(LEAST));
}

#[test]
fn a_skied_ball_hangs_and_a_driven_one_is_soon_gone() {
    let Some((level, ..)) = timed_to_the_run(ON) else {
        return;
    };
    let (skied, _, after) = timed_to_the_run((0.0, 50.0)).unwrap();
    let (driven, ..) = timed_to_the_run((0.0, -50.0)).unwrap();
    assert!(skied > level * 2, "{skied} against {level}");
    assert!(driven < level, "{driven} against {level}");
    // However long it hung, it was a home run in the end.
    assert!(after.contains("score 1 of"), "{after}");
    assert!(after.contains("bases ---"), "{after}");
}
