//! The timing indicator mod: the bar in the batting view, and whether what
//! it shows is what a swing really comes to.

mod common;

use bb_game::art::all_named;
use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_modded, game_with, state};

/// The number that follows `before` in a state line.
fn number(state: &str, before: &str) -> Option<usize> {
    let rest = state.split(before).nth(1)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The first and last of the steps the bar shows as the best to swing on.
fn best(state: &str) -> (usize, usize) {
    let first = number(state, "best swung on steps ").expect("the best steps to be given");
    let last = number(state, &format!("best swung on steps {first} to ")).unwrap();
    (first, last)
}

/// Plays up to the frame on which a swing would begin on `step` of the
/// first pitch, counted from the first of the bar's best steps. Returns the
/// state there.
fn to_step(script: &mut Script, from_best: i32) -> String {
    for _ in 0..600 {
        let now = state(script);
        if let Some(step) = number(&now, "Flight { step: ") {
            let wanted = (best(&now).0 as i32 + from_best) as usize;
            assert!(step <= wanted, "{now}");
            script.run(&format!("wait {}", wanted - step)).unwrap();
            return state(script);
        }
        script.run("wait 1").unwrap();
    }
    panic!("no pitch was thrown: {}", state(script));
}

/// Swings where the pointer is, and lets the pitch finish. Returns the
/// state it finished in.
fn swing(script: &mut Script) -> String {
    script.run("click 300 250").unwrap();
    for _ in 0..200 {
        let now = state(script);
        if !now.contains("Flight") {
            return now;
        }
        script.run("wait 1").unwrap();
    }
    panic!("the pitch never ended: {}", state(script));
}

/// What the bar says of the swing.
fn verdict(script: &Script) -> Vec<String> {
    let stage = &script.runner.stage;
    all_named(stage, &[], "verdict")
        .iter()
        .map(|words| stage.child(words).unwrap())
        .filter(|words| words.visible)
        .filter_map(|words| words.said.clone())
        .collect()
}

fn modded(screen: &str, seed: u64) -> Option<Script> {
    game_modded(screen, seed, &[Mod::TimingIndicator])
}

#[test]
fn the_bar_is_there_only_with_the_mod_on() {
    for screen in ["match", "arcade"] {
        let Some(mut script) = game_with(screen, Some(1)) else {
            return;
        };
        script.run("move 300 250; wait 120").unwrap();
        assert!(all_named(&script.runner.stage, &[], "timingBar").is_empty());
        assert!(!state(&mut script).contains("best swung"));

        let mut script = modded(screen, 1).unwrap();
        script.run("move 300 250; wait 120").unwrap();
        assert_eq!(all_named(&script.runner.stage, &[], "timingBar").len(), 1);
        assert!(state(&mut script).contains("best swung"));
        // Nothing is said until there has been a swing.
        assert!(verdict(&script).is_empty());
    }
}

#[test]
fn a_click_that_is_over_before_the_frame_is_played_swings_all_the_same() {
    for screen in ["match", "arcade"] {
        let (Some(mut held), Some(mut tapped), Some(mut left)) =
            (modded(screen, 1), modded(screen, 1), modded(screen, 1))
        else {
            return;
        };
        for script in [&mut held, &mut tapped, &mut left] {
            script.run("move 300 250").unwrap();
            to_step(script, 0);
        }
        // The button is held down while a frame is played, as a slow hand
        // holds it, or is down and up again before any frame is.
        held.run("press; wait 1; release; wait 30").unwrap();
        tapped.run("press; release; wait 31").unwrap();
        left.run("wait 31").unwrap();
        assert_eq!(verdict(&tapped), ["PERFECT", "PERFECT"], "{screen}");
        assert_eq!(state(&mut tapped), state(&mut held), "{screen}");
        assert!(verdict(&left).is_empty(), "{screen}");
    }
}

#[test]
fn a_swing_on_a_step_the_bar_calls_best_meets_the_ball() {
    for screen in ["match", "arcade"] {
        for seed in [1, 2, 3, 4, 5] {
            let Some(mut script) = modded(screen, seed) else {
                return;
            };
            script.run("move 300 250").unwrap();
            let before = to_step(&mut script, 0);
            assert_eq!(
                number(&before, "Flight { step: "),
                Some(best(&before).0),
                "{before}"
            );
            let after = swing(&mut script);
            assert!(after.contains("Watching"), "{screen} {seed}: {after}");
            assert_eq!(verdict(&script), ["PERFECT", "PERFECT"]);
        }
    }
}

#[test]
fn a_swing_well_before_the_coloured_steps_misses_and_is_called_too_early() {
    for seed in [1, 2, 3] {
        let Some(mut script) = modded("match", seed) else {
            return;
        };
        script.run("move 300 250").unwrap();
        to_step(&mut script, -6);
        let after = swing(&mut script);
        assert!(after.contains("Called"), "{seed}: {after}");
        assert_eq!(verdict(&script), ["TOO EARLY", "TOO EARLY"]);
    }
}

#[test]
fn a_swing_after_the_coloured_steps_misses_and_is_called_too_late() {
    let Some(mut script) = modded("match", 1) else {
        return;
    };
    script.run("move 300 250").unwrap();
    let before = to_step(&mut script, 0);
    // To within a few steps of the end of the pitch, long after the ball
    // could be met.
    let frames = number(&before, " after ").unwrap();
    let step = number(&before, "Flight { step: ").unwrap();
    script.run(&format!("wait {}", frames - 3 - step)).unwrap();
    let after = swing(&mut script);
    assert!(after.contains("Called"), "{after}");
    assert_eq!(verdict(&script), ["TOO LATE", "TOO LATE"]);
}

#[test]
fn the_bar_goes_when_the_view_changes_to_the_field() {
    let Some(mut script) = modded("match", 1) else {
        return;
    };
    script.run("move 300 250").unwrap();
    to_step(&mut script, 0);
    swing(&mut script);
    assert_eq!(all_named(&script.runner.stage, &[], "timingBar").len(), 1);
    for _ in 0..400 {
        if state(&mut script).contains("Fielding") {
            break;
        }
        script.run("wait 1").unwrap();
    }
    assert!(state(&mut script).contains("Fielding"));
    assert!(all_named(&script.runner.stage, &[], "timingBar").is_empty());
}
