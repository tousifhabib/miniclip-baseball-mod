//! What is seen while a pitch is played: the batter after his swing,
//! the ball off the bat, the landing pointer, and a fielder's throw.

use super::seen;
use crate::common;
use crate::common::game;

#[test]
fn after_a_swing_the_batter_is_still_and_the_strike_badge_is_taken_down() {
    let Some(mut script) = game("match") else {
        return;
    };
    let state = |script: &mut bb_game::script::Script| {
        script.run("state").unwrap().pop().unwrap_or_default()
    };
    // Swing as the ball leaves the pitcher's hand, which is far too early.
    for _ in 0..2000 {
        if seen(&state(&mut script)).phase == "Flight" {
            break;
        }
        script.run("wait 1").unwrap();
    }
    script.run("click 300 250; wait 220").unwrap();
    assert!(state(&mut script).contains("count 0-1"));

    let tree = script.run("tree").unwrap();
    // The parts of him that move with the swing have stopped with it. Left
    // running they swing on for ever over a body that has stopped.
    for part in ["skinMovie", "helmetMovie", "tShirtMovie"] {
        let name = format!("\"{part}\"");
        let lines: Vec<&String> = tree.iter().filter(|line| line.contains(&name)).collect();
        assert!(!lines.is_empty(), "no {part} on the stage");
        for line in lines {
            assert!(line.contains("stopped"), "{line}");
        }
    }
    // The badge has played once and gone back to showing nothing.
    let badge = tree
        .iter()
        .find(|line| line.contains("\"strikeAnim"))
        .expect("the strike badge");
    assert!(badge.contains("on frame 1 of"), "{badge}");
}

#[test]
fn a_ball_that_is_hit_leaves_the_bat_at_the_size_it_had_grown_to() {
    let Some(mut script) = game("match") else {
        return;
    };
    let state = |script: &mut bb_game::script::Script| {
        script.run("state").unwrap().pop().unwrap_or_default()
    };
    // Seed 1's first pitch is met by a swing 16 frames before it is gone.
    loop {
        let now = state(&mut script);
        let look = seen(&now);
        if look.phase == "Flight" {
            let (x, y) = look.crossing.unwrap();
            let wait = look.frames - 16;
            script
                .run(&format!(
                    "move {x} {y}; wait {wait}; click {x} {y}; wait 14"
                ))
                .unwrap();
            break;
        }
        if let Some((x, y)) = look.crossing {
            script.run(&format!("move {x} {y}")).unwrap();
        }
        script.run("wait 1").unwrap();
    }
    assert!(
        state(&mut script).contains("Watching"),
        "{}",
        state(&mut script)
    );
    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain").unwrap();
    let ball = stage.find(&main, &["ballFly", "ball"]).unwrap();
    let size = stage.child(&ball).unwrap().matrix.a;
    // The art's ball is three pixels across. By the plate the pitch has
    // grown to several times that, and the hit starts from there.
    assert!(
        size > 2.5,
        "the ball is drawn at {size} times the art's size"
    );
}

/// Where the landing pointer is across the screen, and whether it shows.
fn landing_pointer(script: &bb_game::script::Script) -> (f32, bool) {
    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain").unwrap();
    let area = stage.find(&main, &["aimArea"]).unwrap();
    let child = stage.child(&area).unwrap();
    (child.matrix.tx, child.visible)
}

#[test]
fn the_landing_pointer_is_hidden_until_the_pitch_is_shown() {
    let Some(mut script) = game("match") else {
        return;
    };
    script.run("wait 20; move 150 250; wait 30").unwrap();
    assert!(script.run("state").unwrap()[0].contains("Settling"));
    assert!(!landing_pointer(&script).1, "it should be out of sight");
    // Once the pitch has been thrown the crossing point has been shown.
    for _ in 0..2000 {
        if script.run("state").unwrap()[0].contains("Flight") {
            break;
        }
        script.run("wait 1").unwrap();
    }
    assert!(landing_pointer(&script).1, "it should be showing by now");
}

#[test]
fn the_rules_can_have_the_landing_pointer_out_from_the_start() {
    let layer = "[hit]\npointer_before_pitch = true\n";
    let rules = bb_game::rules::Rules::layered(&[("a test", layer)]).unwrap();
    let Some(mut script) = common::game_ruled("match", Some(1), Some(rules)) else {
        return;
    };
    // The pitcher has not begun his wind-up, so nothing has been shown yet.
    script.run("wait 20; move 150 250; wait 30").unwrap();
    let (ring_left, showing) = landing_pointer(&script);
    assert!(showing);
    script.run("move 420 250; wait 30").unwrap();
    let (ring_right, _) = landing_pointer(&script);
    assert!(script.run("state").unwrap()[0].contains("Settling"));
    // Aiming to one side sends the ball the other way.
    assert!(
        ring_left > ring_right + 100.0,
        "ring left gave {ring_left}, ring right gave {ring_right}"
    );
}

#[test]
fn a_fielder_throws_once_and_then_stands() {
    use bb_engine::display::Content;

    let Some(mut script) = game("match") else {
        return;
    };
    let state = |script: &mut bb_game::script::Script| {
        script.run("state").unwrap().pop().unwrap_or_default()
    };
    // Seed 1's first pitch, met poorly, stays in the field and is thrown in.
    loop {
        let look = seen(&state(&mut script));
        if look.phase == "Flight" {
            let (x, y) = look.crossing.unwrap();
            let wait = look.frames - 16;
            script
                .run(&format!("move {x} {y}; wait {wait}; click {x} {y}"))
                .unwrap();
            break;
        }
        if let Some((x, y)) = look.crossing {
            script.run(&format!("move {x} {y}")).unwrap();
        }
        script.run("wait 1").unwrap();
    }
    for _ in 0..3000 {
        if seen(&state(&mut script)).phase == "Ready" {
            break;
        }
        script.run("wait 1").unwrap();
    }
    assert_eq!(seen(&state(&mut script)).phase, "Ready");
    // Long enough for any throw to have played out, and to have begun
    // again if nothing stopped it.
    script.run("wait 240").unwrap();

    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain").unwrap();
    let mut threw = 0;
    for number in 1..=9 {
        let path = stage
            .find(&main, &["field", &format!("fielder{number}")])
            .unwrap();
        let fielder = stage.clip(&path).unwrap();
        if !(46..=145).contains(&fielder.frame) {
            continue;
        }
        threw += 1;
        for child in fielder.children.values() {
            if let Content::Clip(part) = &child.content {
                assert!(
                    !part.playing || part.frame_count(script.runner.library()) <= 1,
                    "fielder {number} is still going through frame {}",
                    part.frame
                );
            }
        }
    }
    assert!(threw > 0, "nobody was left in a throwing pose to check");
}
