//! The sudden death mod: one strike and the batter is out, and runs count
//! double.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_modded, next, pitch};

/// A match with the timing bar up, sudden death or not, and whatever other
/// mods are asked for.
fn game(seed: u64, sudden_death: bool, with: &[Mod]) -> Option<Script> {
    let mut mods = vec![Mod::TimingIndicator];
    mods.extend_from_slice(with);
    if sudden_death {
        mods.push(Mod::SuddenDeath);
    }
    game_modded("match", seed, &mods)
}

/// The ring on the ball, and held so far to one side that the first pitch
/// of the first game is hit foul.
const ON: (f32, f32) = (0.0, 0.0);
const FOUL: (f32, f32) = (-140.0, 0.0);
/// A swing this many steps before the best misses the ball.
const MISS: i32 = -6;

#[test]
fn one_strike_is_out() {
    let Some(mut usual) = game(1, false, &[]) else {
        return;
    };
    let after = pitch(&mut usual, MISS, ON);
    assert!(after.contains("outs 0, count 0-1"), "{after}");

    let mut script = game(1, true, &[]).unwrap();
    let after = pitch(&mut script, MISS, ON);
    assert!(after.contains("outs 1, count 0-0"), "{after}");
    // And so is the next batter, and the one after: the match is lost in
    // three pitches.
    for outs in [2, 3] {
        next(&mut script);
        let after = pitch(&mut script, MISS, ON);
        assert!(
            after.contains(&format!("outs {outs}, count 0-0")),
            "{after}"
        );
    }
    next(&mut script);
    script.run("wait 60").unwrap();
    assert!(common::state(&mut script).starts_with("MatchLost"));
}

#[test]
fn a_foul_is_never_the_last_strike_so_it_is_no_strike_at_all() {
    let Some(mut usual) = game(1, false, &[]) else {
        return;
    };
    let after = pitch(&mut usual, 0, FOUL);
    assert!(after.contains("outs 0, count 0-1"), "{after}");

    let mut script = game(1, true, &[]).unwrap();
    let after = pitch(&mut script, 0, FOUL);
    assert!(after.contains("outs 0, count 0-0"), "{after}");
}

#[test]
fn a_run_counts_for_two() {
    // With every hit a home run, to have runs to count.
    let Some(mut usual) = game(1, false, &[Mod::ZingerHit]) else {
        return;
    };
    let after = pitch(&mut usual, 0, ON);
    assert!(after.contains("score 1 of 6"), "{after}");

    let mut script = game(1, true, &[Mod::ZingerHit]).unwrap();
    let after = pitch(&mut script, 0, ON);
    assert!(after.contains("score 2 of 6"), "{after}");
    next(&mut script);
    let after = pitch(&mut script, 0, ON);
    assert!(after.contains("score 4 of 6"), "{after}");
}
