//! The southpaw mod: the batter bats left-handed, from the other side of
//! the plate, and nothing else in the view changes.

mod common;

use bb_engine::math::Matrix;
use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{
    LEAVE, MISS, ON, game_modded, game_ruled, long_match, next, number, pitch, pitch_seen, ready,
    state, timing_bar_and,
};

/// The middle of the plate, across the batting view.
const PLATE: f32 = 295.0;

fn game(southpaw: bool) -> Option<Script> {
    long_match(1, &timing_bar_and(Mod::Southpaw, southpaw))
}

/// How something in the batting view is put there, by the names that lead
/// to it from the view.
fn placed(script: &Script, names: &[&str]) -> Matrix {
    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain").expect("the view");
    let path = stage.find(&main, names).expect("something of that name");
    stage.child(&path).expect("it").matrix
}

#[test]
fn the_batter_stands_on_the_other_side_of_the_plate_and_nothing_else_moves() {
    let (Some(mut plain), Some(mut lefty)) = (game(false), game(true)) else {
        return;
    };
    assert!(ready(&mut lefty).contains(", southpaw"));
    assert!(!ready(&mut plain).contains("southpaw"));
    // He is as far to the right of the plate as he was to the left of it,
    // and turned round.
    let (was, is) = (placed(&plain, &["hitter"]), placed(&lefty, &["hitter"]));
    assert!(was.a > 0.0 && is.a == -was.a, "{is:?}");
    assert!((is.tx - (PLATE * 2.0 - was.tx)).abs() < 0.01, "{is:?}");
    assert_eq!((is.ty, is.d), (was.ty, was.d));
    // The stadium, the scoreboard, the pitcher, the field and the pointer
    // are where they were.
    for names in [
        &["scoreboard"][..],
        &["pitcher"],
        &["field"],
        &["aimArea"],
        &["strikeZone"],
        &["btn_nextBall"],
        &["field", "fielder1"],
    ] {
        assert_eq!(placed(&lefty, names), placed(&plain, names), "{names:?}");
    }
    let stage = |script: &Script| {
        let stage = &script.runner.stage;
        let main = stage.find_named(&[], "gameMain").unwrap();
        // The picture of the stadium, and the little field in the corner.
        [501, 1092].map(|symbol| {
            let path = stage.find_symbol(&main, symbol).unwrap();
            stage.child(&path).unwrap().matrix
        })
    };
    assert_eq!(stage(&lefty), stage(&plain));
    // The number on his shirt is drawn the right way round, turned round
    // as he is.
    assert!(lefty.runner.stage.upright_text);
    assert!(!plain.runner.stage.upright_text);
}

#[test]
fn he_is_pitched_to_as_a_right_hander_was() {
    let (Some(mut plain), Some(mut lefty)) = (game(false), game(true)) else {
        return;
    };
    let mut curved = 0;
    for _ in 0..12 {
        let (was, is) = (ready(&mut plain), ready(&mut lefty));
        let crossing = |now: &str| {
            let after = now.split("crossing ").nth(1).unwrap();
            (
                number(now, "crossing ").unwrap(),
                number(after, ",").unwrap(),
            )
        };
        let ((was_x, was_y), (is_x, is_y)) = (crossing(&was), crossing(&is));
        // Each pitch crosses as far to the other side of the plate, and as
        // high, and takes as long.
        assert!((is_x - (PLATE * 2.0 - was_x)).abs() <= 2.0, "{was} | {is}");
        assert_eq!(is_y, was_y);
        assert_eq!(number(&is, "after "), number(&was, "after "));
        assert_eq!(
            is.contains("outside the zone"),
            was.contains("outside the zone"),
            "{was} | {is}"
        );
        curved += u32::from((was_x - PLATE).abs() > 5.0);
        for script in [&mut plain, &mut lefty] {
            pitch(script, LEAVE, ON);
            next(script);
        }
    }
    // Most of them were well to one side or the other, so that is a
    // difference to be seen.
    assert!(curved > 6, "{curved}");
}

#[test]
fn the_ring_is_kept_to_his_side_of_the_plate_as_it_was_to_a_right_handers() {
    let (Some(mut plain), Some(mut lefty)) = (game(false), game(true)) else {
        return;
    };
    // How far the ring goes to the left and to the right, while the
    // pitcher stands and winds up.
    let reach = |script: &mut Script| {
        ready(script);
        [20.0, 570.0].map(|across| {
            script.run(&format!("move {across} 250; wait 70")).unwrap();
            placed(script, &["aimCircle"]).tx
        })
    };
    // A right-hander's ring goes further to the left, towards him, than
    // to the right. A left-hander's goes as much further to the right.
    let [left, right] = reach(&mut plain);
    assert!(PLATE - left > right - PLATE + 5.0, "{left} and {right}");
    let [far_left, far_right] = reach(&mut lefty);
    assert!(
        (far_right - (PLATE * 2.0 - left)).abs() < 1.5,
        "{far_right}"
    );
    assert!((far_left - (PLATE * 2.0 - right)).abs() < 1.5, "{far_left}");
}

#[test]
fn he_runs_to_first_the_way_anyone_does() {
    let (Some(mut plain), Some(mut lefty)) = (game(false), game(true)) else {
        return;
    };
    // Where the batter is put, with how many frames the ball has been
    // watched leaving the bat.
    let watched = |script: &mut Script| {
        ready(script);
        let mut seen = Vec::new();
        pitch_seen(script, 0, ON, |script, now| {
            if now.contains(": Watching") {
                seen.push(placed(script, &["hitter"]));
            }
        });
        seen
    };
    let (was, is) = (watched(&mut plain), watched(&mut lefty));
    assert_eq!(was.len(), is.len());
    // Through his swing he is turned round, on his own side.
    assert!(is[5].a < 0.0 && is[5].tx > PLATE, "{:?}", is[5]);
    // When he sets off he faces the way a right-hander does, who runs to
    // the right, and goes from where he stood, not from across the plate.
    let last = is.len() - 1;
    assert_eq!(was[last], was[5]);
    assert!(is[last].a > 0.0, "{:?}", is[last]);
    assert!(is[last].tx > was[last].tx + 150.0, "{:?}", is[last]);
}

#[test]
fn a_miss_is_a_strike_and_a_hit_is_a_hit_for_him_as_for_anyone() {
    let Some(mut script) = game(true) else {
        return;
    };
    ready(&mut script);
    let missed = pitch(&mut script, MISS, ON);
    assert!(missed.contains("count 0-1"), "{missed}");
    next(&mut script);
    ready(&mut script);
    // Timed as well as can be with the ring on the ball: over the wall.
    let hit = pitch(&mut script, 0, ON);
    assert!(hit.contains("score 1 of"), "{hit}");
}

#[test]
fn the_arcade_game_has_him_too_and_no_other_screen_is_changed() {
    let mods = [Mod::TimingIndicator, Mod::Southpaw];
    let Some(mut arcade) = game_modded("arcade", 1, &mods) else {
        return;
    };
    ready(&mut arcade);
    assert!(placed(&arcade, &["hitter"]).a < 0.0);
    // A match that is lost with the first batter out. Whatever is written
    // on the screen it ends on is drawn as it always was.
    let short = bb_game::rules::Rules::layered(&[("one out", "[match]\nouts = 1\n")]).unwrap();
    let mut script = game_ruled("match", Some(1), Some(short)).unwrap();
    script.runner.stage.upright_text = true;
    for _ in 0..600 {
        let now = state(&mut script);
        if !common::playing(&now) {
            break;
        }
        script.run("click 545 355; wait 5").unwrap();
    }
    assert!(state(&mut script).starts_with("MatchLost"));
    assert!(!script.runner.stage.upright_text);
}
