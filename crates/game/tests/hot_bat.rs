//! The hot bat mod: hits in a row widen the timing window, and a strike
//! takes it back.

mod common;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_modded, long_match, next, number, pitch_seen, said, state, timing_bar_and};

/// A match long enough for any run of hits, with the timing bar up.
fn game(hot_bat: bool) -> Option<Script> {
    long_match(1, &timing_bar_and(Mod::HotBat, hot_bat))
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

#[test]
fn in_the_arcade_game_a_pitch_that_goes_by_takes_the_run_of_hits_back_too() {
    let mods = [Mod::TimingIndicator, Mod::HotBat];
    let Some(mut script) = game_modded("arcade", 1, &mods) else {
        return;
    };
    // The arcade game says nothing of a run of hits when asked how it
    // stands, so the corner of the view is read for it.
    assert!(swing(&mut script, 0));
    assert!(swing(&mut script, 0));
    assert_eq!(said(&script, "hotBat"), ["HOT BAT 2"]);
    // The arcade game keeps no count, but a miss is a miss.
    assert!(!swing(&mut script, -9));
    assert!(said(&script, "hotBat").is_empty());
    // And the window is as it was: a swing a frame too early misses.
    assert!(!swing(&mut script, TOO_EARLY));
}
