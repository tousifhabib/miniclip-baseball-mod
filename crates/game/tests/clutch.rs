//! The clutch mod: with two out and a runner on second or third, runs count
//! double.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{long_match_ruled, next, number, pitch, said, state};

/// The ring on the ball, a swing that misses, and one that never comes.
const ON: (f32, f32) = (0.0, 0.0);
const MISS: i32 = -6;
const LEAVE: i32 = 1000;
/// Three outs to a side, every pitch wide of the strike zone, and one ball
/// a walk: a pitch let go by puts a man on first.
const WALKS: &str = "[match]\nouts = 3\n[count]\nballs = 1\n[pitch.medium]\n\
                     target = { x = 420.0, y = 250.0, width = 2.0, height = 2.0 }\n";

fn game(clutch: bool) -> Option<Script> {
    let mut mods = vec![Mod::TimingIndicator];
    if clutch {
        mods.push(Mod::Clutch);
    }
    long_match_ruled(1, &mods, WALKS)
}

/// Waits for the pitcher to stand ready for the next pitch, and returns how
/// things stand then.
fn ready(script: &mut Script) -> String {
    for _ in 0..600 {
        let now = state(script);
        if now.contains("Settling") {
            return now;
        }
        script.run("wait 1").unwrap();
    }
    panic!("the next pitch never came: {}", state(script));
}

/// Lets the next pitch go by, which walks the batter. Returns the score
/// when he is on first.
fn walk(script: &mut Script) -> f32 {
    ready(script);
    let after = pitch(script, LEAVE, ON);
    next(script);
    number(&after, "score ").unwrap()
}

/// Swings at and misses three pitches: the batter is out.
fn strike_out(script: &mut Script) {
    for _ in 0..3 {
        ready(script);
        pitch(script, MISS, ON);
        next(script);
    }
}

/// Whether the pitch that is coming is one that runs count double on, and
/// says so.
fn in_the_clutch(script: &mut Script) -> bool {
    let now = ready(script);
    let told = said(script, "clutch") == ["CLUTCH: RUNS X2"];
    assert_eq!(now.contains(", clutch"), told, "{now}");
    told
}

#[test]
fn runs_count_double_with_two_out_and_a_runner_on_second_or_third() {
    let Some(mut script) = game(true) else {
        return;
    };
    // Runners on first and second, and nobody out: not yet.
    walk(&mut script);
    walk(&mut script);
    assert!(ready(&mut script).contains("outs 0, count 0-0, bases xx-"));
    assert!(!in_the_clutch(&mut script));
    strike_out(&mut script);
    assert!(!in_the_clutch(&mut script));
    // Two out now, with a man on second.
    strike_out(&mut script);
    assert!(ready(&mut script).contains("outs 2, count 0-0, bases xx-"));
    assert!(in_the_clutch(&mut script));
    // It is still so with the bases loaded, and the walk after that
    // forces in a run that counts for two.
    assert_eq!(walk(&mut script), 0.0);
    assert!(in_the_clutch(&mut script));
    assert_eq!(walk(&mut script), 2.0);
    assert_eq!(walk(&mut script), 4.0);
}

#[test]
fn a_runner_on_first_alone_is_not_enough() {
    let Some(mut script) = game(true) else {
        return;
    };
    strike_out(&mut script);
    strike_out(&mut script);
    assert!(ready(&mut script).contains("outs 2, count 0-0, bases ---"));
    assert!(!in_the_clutch(&mut script));
    walk(&mut script);
    assert!(ready(&mut script).contains("bases x--"));
    assert!(!in_the_clutch(&mut script));
    // The next walk moves him to second.
    walk(&mut script);
    assert!(in_the_clutch(&mut script));
}

#[test]
fn without_the_mod_a_run_is_a_run() {
    let Some(mut script) = game(false) else {
        return;
    };
    walk(&mut script);
    walk(&mut script);
    strike_out(&mut script);
    strike_out(&mut script);
    assert!(!in_the_clutch(&mut script));
    assert_eq!(walk(&mut script), 0.0);
    assert_eq!(walk(&mut script), 1.0);
}
