//! The clutch mod: with two out and a runner on second or third, runs count
//! double.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{long_match_ruled, ready, said, strike_out, timing_bar_and, walk};

/// Three outs to a side, every pitch wide of the strike zone, and one ball
/// a walk: a pitch let go by puts a man on first.
const WALKS: &str = "[match]\nouts = 3\n[count]\nballs = 1\n[pitch.medium]\n\
                     target = { x = 420.0, y = 250.0, width = 2.0, height = 2.0 }\n";

fn game(clutch: bool) -> Option<Script> {
    long_match_ruled(1, &timing_bar_and(Mod::Clutch, clutch), WALKS)
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
