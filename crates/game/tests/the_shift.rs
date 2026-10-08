//! The shift mod: the fielders stand where the ball has been going.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{long_match, next, number, pitch, said, state};

/// The middle of the batting view, which a hit straight up the field goes
/// to.
const MIDDLE: f32 = 295.0;
/// How far to the side of straight a hit is sent, in pixels of the batting
/// view, to come down well to the left or the right and still be fair.
const LEFT: f32 = -260.0;
const RIGHT: f32 = 240.0;
/// The fielders who roam, and two who do not: the pitcher and the man at
/// first.
const ROAMERS: [&str; 4] = ["fielder1", "fielder2", "fielder4", "fielder5"];
const STAY: [&str; 2] = ["fielder3", "fielder6"];

/// A match long enough for any number of hits, with the timing bar up.
fn game(shift: bool) -> Option<Script> {
    let mut mods = vec![Mod::TimingIndicator];
    if shift {
        mods.push(Mod::TheShift);
    }
    long_match(1, &mods)
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

/// Hits the next pitch `aside` pixels to the side of straight, and asks
/// for the one after.
fn hit(script: &mut Script, aside: f32) {
    let crosses = number(&ready(script), "crossing ").unwrap();
    // A hit goes four pixels aside for each the ball is off the middle,
    // and three the other way for each the ring is off the ball.
    let off = (4.0 * (crosses - MIDDLE) - aside) / 3.0;
    pitch(script, 0, (off, 0.0));
    next(script);
}

/// Where a fielder stands on the field, by his name in the art.
fn stands(script: &Script, name: &str) -> (f32, f32) {
    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain").unwrap();
    let fielder = stage.find(&main, &["field", name]).unwrap();
    let matrix = stage.child(&fielder).unwrap().matrix;
    (matrix.tx, matrix.ty)
}

/// Where the left fielder's mark is on the little field in the corner.
fn little_mark(script: &Script) -> (f32, f32) {
    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain").unwrap();
    let mut mark = stage.find_symbol(&main, 1092).unwrap();
    mark.push(6);
    let matrix = stage.child(&mark).unwrap().matrix;
    (matrix.tx, matrix.ty)
}

fn all(script: &Script, names: &[&str]) -> Vec<(f32, f32)> {
    names.iter().map(|name| stands(script, name)).collect()
}

#[test]
fn nobody_moves_until_three_balls_have_gone_one_way_and_then_they_all_do() {
    let (Some(mut script), Some(mut plain)) = (game(true), game(false)) else {
        return;
    };
    ready(&mut script);
    ready(&mut plain);
    let (roamers, stay) = (all(&script, &ROAMERS), all(&script, &STAY));
    let mark = little_mark(&script);
    // They start where the game has always had them.
    assert_eq!(roamers, all(&plain, &ROAMERS));
    for _ in 0..2 {
        hit(&mut script, LEFT);
        let now = ready(&mut script);
        assert!(!now.contains("shifted"), "{now}");
        assert_eq!(all(&script, &ROAMERS), roamers);
        assert!(said(&script, "shift").is_empty());
    }
    hit(&mut script, LEFT);
    let now = ready(&mut script);
    assert!(now.contains(", shifted left 0."), "{now}");
    assert_eq!(said(&script, "shift"), ["SHIFT LEFT"]);
    for (was, is) in roamers.iter().zip(all(&script, &ROAMERS)) {
        assert!(is.0 < was.0 - 10.0, "{was:?} to {is:?}");
    }
    // The pitcher and the men at the bases are where they were.
    assert_eq!(all(&script, &STAY), stay);
    // And the little field in the corner shows it.
    assert!(little_mark(&script).0 < mark.0 - 2.0);
}

#[test]
fn going_the_other_way_brings_them_back_and_then_over() {
    let Some(mut script) = game(true) else {
        return;
    };
    for _ in 0..3 {
        hit(&mut script, LEFT);
    }
    let left = number(&ready(&mut script), "shifted left ").unwrap();
    let mut over = None;
    for hits in 1..=8 {
        hit(&mut script, RIGHT);
        let now = ready(&mut script);
        if let Some(less) = number(&now, "shifted left ") {
            assert!(less < left, "{now}");
        }
        if now.contains("shifted right") && over.is_none() {
            over = Some(hits);
        }
    }
    // Eight balls are all they go by, so by now it is as if the first
    // three had never been.
    assert!(over.is_some());
    assert_eq!(said(&script, "shift"), ["SHIFT RIGHT"]);
    let centre = stands(&script, "fielder4");
    let Some(mut plain) = game(false) else {
        return;
    };
    ready(&mut plain);
    assert!(centre.0 > stands(&plain, "fielder4").0 + 10.0);
}

#[test]
fn without_the_mod_they_stay_where_they_are() {
    let Some(mut script) = game(false) else {
        return;
    };
    ready(&mut script);
    let roamers = all(&script, &ROAMERS);
    for _ in 0..4 {
        hit(&mut script, LEFT);
    }
    let now = ready(&mut script);
    assert!(!now.contains("shifted"), "{now}");
    assert_eq!(all(&script, &ROAMERS), roamers);
    assert!(said(&script, "shift").is_empty());
}
