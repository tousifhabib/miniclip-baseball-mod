//! The rally mod: batters who reach base one after another make the runs
//! that follow worth more, until somebody is out.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{long_match_ruled, next, number, pitch, said, state};

/// The ring on the ball, a swing that misses, and one that never comes.
const ON: (f32, f32) = (0.0, 0.0);
const MISS: i32 = -6;
const LEAVE: i32 = 1000;
/// Every pitch is wide of the strike zone, and one ball walks the batter:
/// a pitch let go by puts a man on first.
const WALKS: &str = "[count]\nballs = 1\n[pitch.medium]\n\
                     target = { x = 420.0, y = 250.0, width = 2.0, height = 2.0 }\n";

fn game(rally: bool) -> Option<Script> {
    let mut mods = vec![Mod::TimingIndicator];
    if rally {
        mods.push(Mod::Rally);
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
