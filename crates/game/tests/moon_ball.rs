//! The moon ball mod: a hit that floats to where it was going.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{ON, game_levelled, game_modded, pitch, pitch_seen, sounds};

/// How to swing at the first pitch of the first game for a ball that comes
/// down in the outfield before anyone can get under it, as the game was.
const DROPS_IN: (i32, (f32, f32)) = (2, (0.0, 8.0));

fn usual(screen: &str) -> Option<Script> {
    game_modded(screen, 1, &[Mod::TimingIndicator])
}

fn moon(screen: &str, level: u8) -> Option<Script> {
    game_levelled(screen, 1, Mod::MoonBall, level)
}

/// Plays the first pitch. Returns the state after, and how many frames the
/// ball was in the field.
fn played(script: &mut Script, hit: (i32, (f32, f32))) -> (String, u32) {
    let mut frames = 0;
    let after = pitch_seen(script, hit.0, hit.1, |_, now| {
        frames += u32::from(now.contains(": Fielding"));
    });
    (after, frames)
}

#[test]
fn a_home_run_is_still_one_and_takes_longer_the_higher_the_mod_is_set() {
    let Some(mut script) = usual("match") else {
        return;
    };
    let (after, quick) = played(&mut script, (0, ON));
    assert!(after.contains("score 1 of 3"), "{after}");

    let mut last = quick;
    for level in [1, 3, 5] {
        let mut script = moon("match", level).unwrap();
        let (after, slow) = played(&mut script, (0, ON));
        assert!(after.contains("score 1 of 3"), "level {level}: {after}");
        assert!(slow > last, "level {level}: {slow} after {last}");
        last = slow;
    }
    assert!(last > quick * 3, "{last} against {quick}");
}

#[test]
fn a_ball_that_dropped_in_is_caught_when_the_fielder_has_time_to_get_under_it() {
    let Some(mut script) = usual("match") else {
        return;
    };
    let (after, _) = played(&mut script, DROPS_IN);
    assert!(after.contains("outs 0, count 0-0, bases x--"), "{after}");

    let mut script = moon("match", 5).unwrap();
    let (after, _) = played(&mut script, DROPS_IN);
    assert!(after.contains("outs 1, count 0-0, bases ---"), "{after}");
    // Out to a catch, not to a throw.
    assert!(sounds(&mut script).contains(&"umpire_out_1".to_owned()));
}

#[test]
fn the_arcade_game_is_left_as_it_was() {
    let mut ends = Vec::new();
    for script in [usual("arcade"), moon("arcade", 5)] {
        let Some(mut script) = script else {
            return;
        };
        pitch(&mut script, 2, (0.0, 8.0));
        script.run("wait 500").unwrap();
        ends.push(common::state(&mut script));
    }
    assert_eq!(ends[0], ends[1]);
}
