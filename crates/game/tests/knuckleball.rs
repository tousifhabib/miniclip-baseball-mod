//! The knuckleball mod: pitches that sway on the way in, and a marker that
//! is only roughly right about where they will cross.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_modded, number, pitch_seen, state};

/// A match with the timing bar up, and the knuckleball or not.
fn game(seed: u64, knuckleball: bool) -> Option<Script> {
    let mut mods = vec![Mod::TimingIndicator];
    if knuckleball {
        mods.push(Mod::Knuckleball);
    }
    game_modded("match", seed, &mods)
}

/// Where something in the batting view is.
fn place(script: &Script, name: &str) -> (f32, f32) {
    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain").unwrap();
    let path = stage.find(&main, &[name]).unwrap();
    let child = stage.child(&path).unwrap();
    (child.matrix.tx, child.matrix.ty)
}

/// Plays the first pitch without swinging. Returns how far the marker was
/// from where the pitch really crossed, across and down, and how many times
/// the ball changed the way it was going across the view.
fn watched(script: &mut Script) -> ((f32, f32), u32) {
    let (mut off, mut last, mut going, mut turns) = ((0.0, 0.0), None, 0.0f32, 0);
    for _ in 0..2000 {
        let now = state(script);
        if now.contains(": Called") || now.contains(": Ready") {
            break;
        }
        if now.contains("Flight") {
            let crosses = (
                number(&now, "crossing ").unwrap(),
                number(now.split("crossing ").nth(1).unwrap(), ",").unwrap(),
            );
            let marker = place(script, "ballPassesBat_marker");
            off = (marker.0 - crosses.0, marker.1 - crosses.1);
            let ball = place(script, "ballAll").0;
            if let Some(last) = last {
                let step: f32 = ball - last;
                if step.abs() > 0.05 {
                    if going != 0.0 && step.signum() != going {
                        turns += 1;
                    }
                    going = step.signum();
                }
            }
            last = Some(ball);
        }
        script.run("wait 1").unwrap();
    }
    (off, turns)
}

#[test]
fn the_ball_sways_and_the_marker_is_only_roughly_right() {
    let mut furthest = 0.0f32;
    for seed in 1..=6 {
        let Some(mut usual) = game(seed, false) else {
            return;
        };
        // As the game was, the marker is where the pitch crosses, and the
        // ball curves one way if it curves at all.
        let (off, turns) = watched(&mut usual);
        assert!(
            off.0.abs() < 1.0 && off.1.abs() < 1.0,
            "seed {seed}: {off:?}"
        );
        assert!(turns <= 1, "seed {seed}: {turns}");

        let mut knuckle = game(seed, true).unwrap();
        let (off, turns) = watched(&mut knuckle);
        // Out by no more than the ball sways, and only across.
        assert!(off.0.abs() <= 16.6, "seed {seed}: {off:?}");
        assert!(off.1.abs() < 1.0, "seed {seed}: {off:?}");
        assert!(turns >= 3, "seed {seed}: {turns}");
        furthest = furthest.max(off.0.abs());
    }
    assert!(furthest > 8.0, "{furthest}");
}

#[test]
fn a_swing_timed_as_the_bar_says_still_meets_it() {
    for seed in 1..=4 {
        let Some(mut script) = game(seed, true) else {
            return;
        };
        let mut met = false;
        pitch_seen(&mut script, 0, (0.0, 0.0), |_, now| {
            met |= now.contains("Watching");
        });
        assert!(met, "seed {seed}");
    }
}
