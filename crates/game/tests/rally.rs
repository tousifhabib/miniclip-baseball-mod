//! The rally mod: batters who reach base one after another make the runs
//! that follow worth more, until somebody is out.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{long_match_ruled, number, ready, said, strike_out, timing_bar_and, walk};

/// Every pitch is wide of the strike zone, and one ball walks the batter:
/// a pitch let go by puts a man on first.
const WALKS: &str = "[count]\nballs = 1\n[pitch.medium]\n\
                     target = { x = 420.0, y = 250.0, width = 2.0, height = 2.0 }\n";

fn game(rally: bool) -> Option<Script> {
    long_match_ruled(1, &timing_bar_and(Mod::Rally, rally), WALKS)
}

#[test]
fn each_batter_in_a_row_who_reaches_base_makes_a_run_worth_one_more() {
    let Some(mut script) = game(true) else {
        return;
    };
    // Three walks load the bases, and nobody has scored.
    for walked in 1..=3 {
        assert_eq!(walk(&mut script), 0.0);
        let now = ready(&mut script);
        assert!(now.contains(&format!(", rally {walked}")), "{now}");
        let worth = format!("RALLY: RUNS X{}", walked + 1);
        assert_eq!(said(&script, "rally"), [worth]);
    }
    // The fourth forces a run in, worth one and one for each of the three.
    assert_eq!(walk(&mut script), 4.0);
    // The fifth's is worth five, and that is as much as a run is ever
    // worth.
    assert_eq!(walk(&mut script), 9.0);
    assert!(ready(&mut script).contains(", rally 5"));
    assert_eq!(said(&script, "rally"), ["RALLY: RUNS X5"]);
    assert_eq!(walk(&mut script), 14.0);
}

#[test]
fn an_out_ends_the_rally() {
    let Some(mut script) = game(true) else {
        return;
    };
    for _ in 0..4 {
        walk(&mut script);
    }
    assert!(ready(&mut script).contains(", rally 4"));
    strike_out(&mut script);
    let now = ready(&mut script);
    assert!(!now.contains("rally"), "{now}");
    assert!(said(&script, "rally").is_empty());
    // The bases are still loaded, and the next run is worth one.
    let score = number(&now, "score ").unwrap();
    assert_eq!(walk(&mut script), score + 1.0);
    assert!(ready(&mut script).contains(", rally 1"));
}

#[test]
fn without_the_mod_a_run_is_a_run() {
    let Some(mut script) = game(false) else {
        return;
    };
    for _ in 0..3 {
        assert_eq!(walk(&mut script), 0.0);
    }
    for runs in 1..=3 {
        assert_eq!(walk(&mut script), runs as f32);
    }
    assert!(!ready(&mut script).contains("rally"));
    assert!(said(&script, "rally").is_empty());
}
