//! Clicks that reach the game after they were made: a swing begins on the
//! step of the pitch the player saw when they clicked, and not on the
//! later one the click arrived on.

mod common;

use bb_game::delay::Delay;
use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_delayed, game_modded, number, said, state};

/// Where the ring is held: on the ball of every pitch the tests play.
const RING: &str = "300 250";

fn modded(screen: &str, seed: u64) -> Option<Script> {
    game_modded(screen, seed, &[Mod::TimingIndicator])
}

/// Plays up to the frame on which a click reaches the game on `step` of
/// the first pitch, counted from the first of the bar's best steps.
fn to_step(script: &mut Script, from_best: i32) {
    script.run(&format!("move {RING}")).unwrap();
    for _ in 0..600 {
        let now = state(script);
        if let (Some(step), Some(best)) = (
            number(&now, "Flight { step: "),
            number(&now, "best swung on steps "),
        ) {
            let wanted = best as i32 + from_best;
            assert!(step as i32 <= wanted, "{now}");
            script
                .run(&format!("wait {}", wanted - step as i32))
                .unwrap();
            return;
        }
        script.run("wait 1").unwrap();
    }
    panic!("no pitch was thrown: {}", state(script));
}

/// Clicks with `click`, and plays until the ball has been met or the pitch
/// has gone by. Returns the state then.
fn swing_with(script: &mut Script, click: &str) -> String {
    script.run(click).unwrap();
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
    said(script, "verdict")
}

#[test]
fn a_click_that_arrives_late_is_the_swing_it_was_when_it_was_made() {
    for screen in ["match", "arcade"] {
        for seed in [1, 2, 3] {
            for late in [1, 2, 4] {
                let (Some(mut on_time), Some(mut slow)) =
                    (modded(screen, seed), modded(screen, seed))
                else {
                    return;
                };
                to_step(&mut on_time, 0);
                to_step(&mut slow, late);
                let made = swing_with(&mut on_time, &format!("click {RING}"));
                let arrived = swing_with(&mut slow, &format!("click {RING} late {late}"));
                // The same swing, meeting the ball on the same frame.
                assert_eq!(made, arrived, "{screen} {seed}, {late} late");
                assert_eq!(verdict(&slow), ["PERFECT"], "{screen} {seed}, {late} late");
            }
        }
    }
}

#[test]
fn a_click_whose_lateness_is_not_known_swings_on_the_step_it_arrives() {
    let (Some(mut untimed), Some(mut timed)) = (modded("match", 1), modded("match", 1)) else {
        return;
    };
    to_step(&mut untimed, 3);
    to_step(&mut timed, 3);
    swing_with(&mut untimed, &format!("click {RING}"));
    swing_with(&mut timed, &format!("click {RING} late 0"));
    // Late on the bar, as it would always have been.
    assert_ne!(verdict(&untimed), ["PERFECT"]);
    assert_eq!(verdict(&untimed), verdict(&timed));
}

#[test]
fn a_click_that_arrives_after_the_bat_would_have_met_the_ball_meets_it_then() {
    let Some(mut on_time) = modded("match", 1) else {
        return;
    };
    to_step(&mut on_time, 0);
    on_time.run(&format!("click {RING}")).unwrap();
    // How many frames into the swing the bat meets the ball.
    let mut frames = 1;
    while state(&mut on_time).contains("Flight") {
        on_time.run("wait 1").unwrap();
        frames += 1;
    }
    let met = state(&mut on_time);
    assert!(met.contains("Watching"), "{met}");

    // The click reaches the game two frames after that, and the ball is
    // met as soon as it does.
    let late = frames + 2;
    let mut slow = modded("match", 1).unwrap();
    to_step(&mut slow, late);
    let after = swing_with(&mut slow, &format!("click {RING} late {late}"));
    assert!(after.contains("Watching"), "{after}");
    assert_eq!(verdict(&slow), ["PERFECT"]);
}

#[test]
fn a_click_made_before_the_ball_was_thrown_swings_as_it_is_thrown() {
    let Some(mut script) = modded("match", 1) else {
        return;
    };
    script.run(&format!("move {RING}")).unwrap();
    while !state(&mut script).contains("Flight { step: 2 }") {
        script.run("wait 1").unwrap();
    }
    let after = swing_with(&mut script, &format!("click {RING} late 10"));
    assert!(after.contains("Called"), "{after}");
    assert_eq!(verdict(&script), ["TOO EARLY"]);
}

#[test]
fn the_players_delay_takes_a_click_as_made_that_much_before_it_arrived() {
    for late in [1, 3] {
        // A delay of so many frames of a game played at sixty a second,
        // less half of one. A click made as a frame came up, and reaching
        // the game that much later, is taken as made on that frame.
        let delay = Delay::of((late as f32 - 0.5) * 1000.0 / 60.0);
        let (Some(mut on_time), Some(mut delayed)) = (
            modded("match", 2),
            game_delayed("match", 2, &[Mod::TimingIndicator], delay),
        ) else {
            return;
        };
        to_step(&mut on_time, 0);
        to_step(&mut delayed, late);
        let made = swing_with(&mut on_time, &format!("click {RING}"));
        // The window says the click was made as the frame it reached was
        // showing, and the player's delay takes it back from there.
        let arrived = swing_with(&mut delayed, &format!("click {RING} late 0"));
        assert_eq!(made, arrived, "{late} late");
        assert_eq!(verdict(&delayed), ["PERFECT"], "{late} late");
        // A click the window did not time is taken where it arrived, as
        // it would be with no delay at all.
        let mut untimed = game_delayed("match", 2, &[Mod::TimingIndicator], delay).unwrap();
        let mut undelayed = modded("match", 2).unwrap();
        to_step(&mut untimed, late);
        to_step(&mut undelayed, late);
        let untimed_swing = swing_with(&mut untimed, &format!("click {RING}"));
        let undelayed_swing = swing_with(&mut undelayed, &format!("click {RING}"));
        assert_eq!(untimed_swing, undelayed_swing, "{late} late");
        assert_eq!(verdict(&untimed), verdict(&undelayed), "{late} late");
    }
}
