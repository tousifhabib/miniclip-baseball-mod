//! The heat check mod: every run makes the next pitch faster, and every
//! strike slows the pitches down again.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_modded, next, number, pitch, said, state};

/// A match in which every hit is a run, with the timing bar up, and the
/// heat check or not.
fn game(heat: bool) -> Option<Script> {
    let mut mods = vec![Mod::TimingIndicator, Mod::ZingerHit];
    if heat {
        mods.push(Mod::HeatCheck);
    }
    game_modded("match", 1, &mods)
}

/// How many frames the pitch about to be thrown takes, and how much heat
/// the state says is on.
fn coming(script: &mut Script) -> (u32, u32) {
    for _ in 0..400 {
        let now = state(script);
        // The pitcher has yet to start on it.
        if let (Some(frames), true) = (number(&now, " after "), now.contains("Settling")) {
            return (frames as u32, number(&now, "heat ").unwrap_or(0.0) as u32);
        }
        script.run("wait 1").unwrap();
    }
    panic!("no pitch came: {}", state(script));
}

/// Swings at the pitch, to hit it for a run or to miss it, and asks for
/// the next.
fn swing(script: &mut Script, hit: bool) {
    pitch(script, if hit { 0 } else { -6 }, (0.0, 0.0));
    next(script);
}

#[test]
fn a_run_makes_the_next_pitch_faster_and_a_strike_slows_it_again() {
    let Some(mut script) = game(true) else {
        return;
    };
    let (cold, heat) = coming(&mut script);
    assert_eq!(heat, 0);
    assert!(said(&script, "heat").is_empty());

    swing(&mut script, true);
    let (warm, heat) = coming(&mut script);
    assert_eq!(heat, 1);
    assert!(warm < cold, "{warm} against {cold}");
    assert_eq!(said(&script, "heat"), ["HEAT 1"]);

    swing(&mut script, true);
    let (hot, heat) = coming(&mut script);
    assert_eq!(heat, 2);
    assert!(hot < warm, "{hot} against {warm}");
    assert_eq!(said(&script, "heat"), ["HEAT 2"]);

    // A swing and a miss is a strike, which takes one run's worth off.
    swing(&mut script, false);
    assert_eq!(coming(&mut script), (warm, 1));
    swing(&mut script, false);
    assert_eq!(coming(&mut script), (cold, 0));
    assert!(said(&script, "heat").is_empty());
    // There is none to take off after that.
    swing(&mut script, false);
    assert_eq!(coming(&mut script), (cold, 0));
}

#[test]
fn without_the_mod_runs_change_nothing_about_the_pitches() {
    let Some(mut script) = game(false) else {
        return;
    };
    let (first, _) = coming(&mut script);
    for _ in 0..3 {
        swing(&mut script, true);
        assert_eq!(coming(&mut script), (first, 0));
    }
}
