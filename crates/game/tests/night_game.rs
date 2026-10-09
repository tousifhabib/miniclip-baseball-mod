//! The night game mod: a dark stadium with the players lit, and lights
//! that flash for a home run.

mod common;

use bb_game::art::BACKDROPS;
use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_modded, pitch_seen, timing_bar_and};

fn game(screen: &str, night: bool) -> Option<Script> {
    game_modded(screen, 1, &timing_bar_and(Mod::NightGame, night))
}

/// How bright each picture of the stadium on the stage is, by its red: 1
/// is as it is by day.
fn stadium(script: &Script) -> Vec<f32> {
    let stage = &script.runner.stage;
    let Some(main) = stage.find_named(&[], "gameMain") else {
        return Vec::new();
    };
    let mut views = vec![main.clone()];
    views.extend(stage.find(&main, &["field"]));
    let mut bright = Vec::new();
    for view in views {
        for child in stage.clip(&view).unwrap().children.values() {
            if BACKDROPS.contains(&child.symbol) {
                bright.push(child.color.mult[0] + child.color.add[0]);
            }
        }
    }
    bright
}

/// How bright the batter is, the same way.
fn batter(script: &Script) -> f32 {
    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain").unwrap();
    let hitter = stage.find(&main, &["hitter"]).unwrap();
    let color = stage.child(&hitter).unwrap().color;
    color.mult[0] + color.add[0]
}

#[test]
fn the_stadium_is_dark_and_the_players_are_not() {
    for screen in ["match", "arcade"] {
        let Some(mut script) = game(screen, true) else {
            return;
        };
        script.run("move 300 250; wait 100").unwrap();
        let pictures = stadium(&script);
        // Behind the batter, and over the field for when that is seen.
        assert!(pictures.len() >= 2, "{screen}: {pictures:?}");
        assert!(pictures.iter().all(|&bright| bright < 0.5), "{pictures:?}");
        assert_eq!(batter(&script), 1.0);

        let mut script = game(screen, false).unwrap();
        script.run("move 300 250; wait 100").unwrap();
        assert!(stadium(&script).iter().all(|&bright| bright == 1.0));
    }
}

#[test]
fn it_stays_dark_from_one_pitch_to_the_next_and_a_home_run_flashes_the_lights() {
    let Some(mut script) = game("match", true) else {
        return;
    };
    // The first pitch of the first game, timed as well as can be with the
    // ring on the ball, is a home run.
    let (mut before, mut after) = (Vec::new(), Vec::new());
    let end = pitch_seen(&mut script, 0, (0.0, 0.0), |script, now| {
        let pictures = stadium(script);
        if pictures.is_empty() {
            return;
        }
        let brightest = pictures.iter().copied().fold(0.0, f32::max);
        if now.contains("score 0 of") {
            before.push(brightest);
        } else {
            after.push(brightest);
        }
    });
    assert!(end.contains("score 1 of 3"), "{end}");
    // Dark all the way to the home run.
    assert!(before.iter().all(|&bright| bright < 0.5), "{before:?}");
    // Then lit, brighter than by day, and dark, by turns, and dark at the
    // end of it.
    let lit: Vec<bool> = after.iter().map(|&bright| bright > 1.0).collect();
    let changes = lit.windows(2).filter(|pair| pair[0] != pair[1]).count();
    assert!(changes >= 6, "{changes}");
    assert!(after.iter().all(|&bright| !(0.5..=1.0).contains(&bright)));
    assert_eq!(lit.last(), Some(&false));
}

#[test]
fn without_the_mod_a_home_run_flashes_nothing() {
    let Some(mut script) = game("match", false) else {
        return;
    };
    let end = pitch_seen(&mut script, 0, (0.0, 0.0), |script, _| {
        assert!(stadium(script).iter().all(|&bright| bright == 1.0));
    });
    assert!(end.contains("score 1 of 3"), "{end}");
}
