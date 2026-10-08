//! The golden ball mod: every fifth pitch is gold, worth triple runs if it
//! is hit and an out if it is not.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_modded, long_match, next, number, pitch, said, state};

/// The ring on the ball.
const ON: (f32, f32) = (0.0, 0.0);
/// A swing this many steps before the best misses the ball, and one this
/// many after never comes: the pitch is let go by.
const MISS: i32 = -6;
const LEAVE: i32 = 1000;

/// Lets pitches go by until the next to be thrown is the fifth.
fn to_the_fifth(script: &mut Script) {
    for _ in 0..4 {
        let after = pitch(script, LEAVE, ON);
        assert!(!after.contains("golden"), "{after}");
        assert!(said(script, "goldenBall").is_empty());
        next(script);
    }
    for _ in 0..400 {
        if state(script).contains("Settling") {
            return;
        }
        script.run("wait 1").unwrap();
    }
    panic!("the fifth pitch never came: {}", state(script));
}

/// Whether the ball in the batting view has been coloured.
fn gilded(script: &Script) -> bool {
    let stage = &script.runner.stage;
    stage
        .find_named(&[], "gameMain")
        .and_then(|main| stage.find(&main, &["ballAll"]))
        .and_then(|ball| stage.child(&ball))
        .is_some_and(|ball| ball.color.mult[2] < 0.9)
}

#[test]
fn every_fifth_pitch_is_gold_and_says_so() {
    let Some(mut script) = long_match(1, &[Mod::TimingIndicator, Mod::GoldenBall]) else {
        return;
    };
    assert!(!gilded(&script));
    to_the_fifth(&mut script);
    assert!(state(&mut script).contains(", golden"));
    assert_eq!(said(&script, "goldenBall"), ["GOLDEN BALL"]);
    assert!(gilded(&script));
    // The one after it is a pitch like any other.
    pitch(&mut script, LEAVE, ON);
    next(&mut script);
    script.run("wait 60").unwrap();
    assert!(!state(&mut script).contains("golden"));
    assert!(!gilded(&script));
}

#[test]
fn a_strike_on_a_golden_ball_is_an_out_whatever_the_count() {
    let Some(mut script) = long_match(1, &[Mod::TimingIndicator, Mod::GoldenBall]) else {
        return;
    };
    to_the_fifth(&mut script);
    let before = state(&mut script);
    let outs = number(&before, "outs ").unwrap();
    // The count is not full: a strike would not be the last, as the game
    // was.
    assert!(!before.contains("-2,"), "{before}");
    let after = pitch(&mut script, MISS, ON);
    assert!(
        after.contains(&format!("outs {}, count 0-0", outs + 1.0)),
        "{after}"
    );
}

#[test]
fn runs_off_a_golden_ball_count_for_three() {
    // With every hit a home run, to have a run to count.
    let mods = [Mod::TimingIndicator, Mod::ZingerHit, Mod::GoldenBall];
    let Some(mut script) = long_match(1, &mods) else {
        return;
    };
    let first = pitch(&mut script, 0, ON);
    assert!(first.contains("score 1 of"), "{first}");
    next(&mut script);
    for _ in 0..3 {
        pitch(&mut script, LEAVE, ON);
        next(&mut script);
    }
    let fifth = pitch(&mut script, 0, ON);
    assert!(fifth.contains(", golden"), "{fifth}");
    assert!(fifth.contains("score 4 of"), "{fifth}");
}

#[test]
fn without_the_mod_or_in_the_arcade_game_no_pitch_is_gold() {
    for (screen, mods) in [
        ("match", vec![Mod::TimingIndicator]),
        ("arcade", vec![Mod::TimingIndicator, Mod::GoldenBall]),
    ] {
        let Some(mut script) = game_modded(screen, 1, &mods) else {
            return;
        };
        for _ in 0..5 {
            let after = pitch(&mut script, LEAVE, ON);
            assert!(!after.contains("golden"), "{screen}: {after}");
            next(&mut script);
        }
        assert!(said(&script, "goldenBall").is_empty());
    }
}
