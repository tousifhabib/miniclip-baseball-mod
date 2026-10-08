//! The turbo runners mod: runners who are fast, and can be sent on with
//! the ball in the air.

mod common;

use bb_game::art::all_named;
use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_levelled, game_modded, pitch_seen};

/// The ring on the ball, and where to hold it for a ball along the ground
/// that gets the batter to first, in the first game.
const ON: (f32, f32) = (0.0, 0.0);
const GROUNDER: (f32, f32) = (0.0, -18.0);

fn game(turbo: Option<u8>) -> Option<Script> {
    match turbo {
        Some(level) => game_levelled("match", 1, Mod::TurboRunners, level),
        None => game_modded("match", 1, &[Mod::TimingIndicator]),
    }
}

/// How many frames after the view changes to the field the batter is on
/// first, for a ball along the ground.
fn frames_to_first(script: &mut Script) -> u32 {
    let (mut frames, mut there) = (0, None);
    let after = pitch_seen(script, 0, GROUNDER, |_, now| {
        if now.contains(": Fielding") {
            frames += 1;
            if now.contains("bases x--") && there.is_none() {
                there = Some(frames);
            }
        }
    });
    there.unwrap_or_else(|| panic!("he never got to first: {after}"))
}

#[test]
fn runners_are_as_many_times_as_fast_as_the_mod_is_set_to() {
    let Some(mut script) = game(None) else {
        return;
    };
    let usual = frames_to_first(&mut script) as f32;
    // Each level's speed, as the rules have it.
    for (level, speed) in [(1, 1.5), (2, 2.0), (5, 4.0)] {
        let mut script = game(Some(level)).unwrap();
        let fast = frames_to_first(&mut script) as f32;
        let times = usual / fast;
        assert!((times - speed).abs() < 0.15, "level {level}: {times}");
    }
}

/// Whether, at some time while a ball hit over the wall was still in the
/// air, the batter was on a base with the button up that sends him on.
fn could_go_on_with_the_ball_in_the_air(script: &mut Script) -> bool {
    let mut could = false;
    pitch_seen(script, 0, ON, |script, now| {
        if !now.contains(": Fielding") || !now.contains("score 0 of") {
            return;
        }
        let stage = &script.runner.stage;
        could |= all_named(stage, &[], "runBtn")
            .iter()
            .any(|button| stage.child(button).unwrap().visible);
    });
    could
}

#[test]
fn a_runner_can_be_sent_on_while_the_ball_is_in_the_air() {
    let Some(mut usual) = game(None) else {
        return;
    };
    assert!(!could_go_on_with_the_ball_in_the_air(&mut usual));
    // At four times the speed the batter is on first long before the ball
    // is over the wall.
    let mut turbo = game(Some(5)).unwrap();
    assert!(could_go_on_with_the_ball_in_the_air(&mut turbo));
}
