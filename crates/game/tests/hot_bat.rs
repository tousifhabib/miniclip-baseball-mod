//! The hot bat mod: hits in a row widen the timing window, and a strike
//! takes it back.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{long_match, next, number, pitch_seen, said, state};

/// A match long enough for any run of hits, with the timing bar up.
fn game(hot_bat: bool) -> Option<Script> {
    let mut mods = vec![Mod::TimingIndicator];
    if hot_bat {
        mods.push(Mod::HotBat);
    }
    long_match(1, &mods)
}

/// Swings at a pitch `late` steps after the first the bar calls best, and
/// asks for the next. Returns whether the bat met the ball.
fn swing(script: &mut Script, late: i32) -> bool {
    let mut met = false;
    pitch_seen(script, late, (0.0, 0.0), |_, now| {
        met |= now.contains("Watching");
    });
    next(script);
    // The pitcher is standing for the next one.
    for _ in 0..400 {
        if state(script).contains("Settling") {
            break;
        }
        script.run("wait 1").unwrap();
    }
    met
}

fn in_a_row(script: &mut Script) -> u32 {
    number(&state(script), "hits in a row ").unwrap_or(0.0) as u32
}

/// A swing this many steps before the best is a frame too early to meet
/// the ball, as the game was.
const TOO_EARLY: i32 = -2;

#[test]
fn a_hit_lets_the_next_swing_be_a_frame_further_out_and_a_strike_takes_that_back() {
    let Some(mut script) = game(true) else {
        return;
    };
    assert!(!swing(&mut script, TOO_EARLY));
    assert_eq!(in_a_row(&mut script), 0);
    assert!(said(&script, "hotBat").is_empty());

    assert!(swing(&mut script, 0));
    assert_eq!(in_a_row(&mut script), 1);
    assert_eq!(said(&script, "hotBat"), ["HOT BAT 1"]);
    // The window is a frame wider at each end now, and that swing meets
    // the ball.
    assert!(swing(&mut script, TOO_EARLY));
    assert_eq!(in_a_row(&mut script), 2);
    assert_eq!(said(&script, "hotBat"), ["HOT BAT 2"]);
    // Two frames wider, and a swing a frame earlier still meets it too.
    assert!(swing(&mut script, TOO_EARLY - 1));
    assert_eq!(in_a_row(&mut script), 3);

    // A swing far too early is a strike, and the window is as it was.
    assert!(!swing(&mut script, -9));
    assert_eq!(in_a_row(&mut script), 0);
    assert!(said(&script, "hotBat").is_empty());
    assert!(!swing(&mut script, TOO_EARLY));
}

#[test]
fn it_adds_no_more_than_three_frames_to_each_end() {
    let Some(mut script) = game(true) else {
        return;
    };
    for hits in 1..=5 {
        assert!(swing(&mut script, 0));
        assert_eq!(in_a_row(&mut script), hits);
    }
    assert_eq!(said(&script, "hotBat"), ["HOT BAT 3"]);
    // Three frames early meets it. Four does not.
    assert!(swing(&mut script, TOO_EARLY - 2));
    assert!(!swing(&mut script, TOO_EARLY - 3));
}

#[test]
fn without_the_mod_hits_in_a_row_widen_nothing() {
    let Some(mut script) = game(false) else {
        return;
    };
    assert!(swing(&mut script, 0));
    assert!(swing(&mut script, 0));
    assert_eq!(in_a_row(&mut script), 0);
    assert!(!swing(&mut script, TOO_EARLY));
}
