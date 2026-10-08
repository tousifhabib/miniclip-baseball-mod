//! The called shot mod: a target put on the outfield before the pitch, and
//! runs for a hit that comes down on it.

mod common;

use bb_game::art::all_named;
use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_modded, number, pitch, said, state};

/// How to swing at the first pitch of the first game for a hit that comes
/// down in the outfield and gets the batter to first: how late, and where
/// to hold the ring.
const HIT: (i32, (f32, f32)) = (2, (0.0, 8.0));

fn game(screen: &str, called_shot: bool) -> Option<Script> {
    let mut mods = vec![Mod::TimingIndicator];
    if called_shot {
        mods.push(Mod::CalledShot);
    }
    game_modded(screen, 1, &mods)
}

/// The two numbers that follow `before` in a state line, as a place.
fn place(state: &str, before: &str) -> Option<(f32, f32)> {
    let rest = state.split(before).nth(1)?;
    Some((number(state, before)?, number(rest, ",")?))
}

/// Clicks the batting view over a place on the field, once the pitcher is
/// standing and waiting.
fn click_over(script: &mut Script, on_field: (f32, f32)) {
    for _ in 0..400 {
        if state(script).contains("Settling") {
            break;
        }
        script.run("wait 1").unwrap();
    }
    // As the arcade game lays its target into the batting view.
    let seen = (
        295.0 + 1.3 * (on_field.0 - 301.45),
        176.1 + 0.3 * (on_field.1 - 108.45),
    );
    script
        .run(&format!("click {} {}; wait 1", seen.0, seen.1))
        .unwrap();
}

/// Where the hit comes down when no shot has been called.
fn comes_down() -> Option<(f32, f32)> {
    let mut script = game("match", true)?;
    let after = pitch(&mut script, HIT.0, HIT.1);
    assert!(after.contains("score 0 of 3"), "{after}");
    assert!(!after.contains("called"), "{after}");
    Some(place(&after, "came down at ").expect("the hit to come down"))
}

#[test]
fn a_hit_that_comes_down_on_the_target_is_worth_runs_by_its_rings() {
    let Some(down) = comes_down() else {
        return;
    };
    // How far to one side of where it comes down the shot is called, and
    // the runs that is worth: the middle, each ring out, and off it.
    for (aside, runs) in [(0.0, 3), (18.0, 2), (32.0, 1), (45.0, 1), (70.0, 0)] {
        let mut script = game("match", true).unwrap();
        let call = (down.0 + aside, down.1);
        click_over(&mut script, call);
        let called = place(&state(&mut script), "called ").expect("the shot to be called");
        assert!((called.0 - call.0).abs() <= 1.0 && (called.1 - call.1).abs() <= 1.0);
        let after = pitch(&mut script, HIT.0, HIT.1);
        // The hit itself is as it was: the batter is on first.
        assert!(after.contains("bases x--"), "{after}");
        assert!(
            after.contains(&format!("score {runs} of 3")),
            "{aside}: {after}"
        );
        let told = said(&script, "calledIt");
        if runs > 0 {
            assert_eq!(told, [format!("CALLED IT! +{runs}")]);
        } else {
            assert!(told.is_empty(), "{told:?}");
        }
    }
}

#[test]
fn the_target_goes_where_the_outfield_is_clicked_until_the_wind_up() {
    let Some(mut script) = game("match", true) else {
        return;
    };
    assert!(all_named(&script.runner.stage, &[], "calledShot").is_empty());
    click_over(&mut script, (200.0, 160.0));
    assert_eq!(place(&state(&mut script), "called "), Some((200.0, 160.0)));
    // One in the batting view, and one on the field for when it is seen.
    assert_eq!(all_named(&script.runner.stage, &[], "calledShot").len(), 2);
    // Another click moves it, and one off the part of the outfield that
    // can be called is taken as the nearest place on it.
    click_over(&mut script, (400.0, 220.0));
    assert_eq!(place(&state(&mut script), "called "), Some((400.0, 220.0)));
    script.run("click 580 330; wait 1").unwrap();
    assert_eq!(place(&state(&mut script), "called "), Some((460.0, 240.0)));
    assert_eq!(all_named(&script.runner.stage, &[], "calledShot").len(), 2);
    // Once he has started his wind-up it stays where it is.
    for _ in 0..400 {
        if state(&mut script).contains("WindUp") {
            break;
        }
        script.run("wait 1").unwrap();
    }
    script.run("click 300 200; wait 1").unwrap();
    assert_eq!(place(&state(&mut script), "called "), Some((460.0, 240.0)));
}

#[test]
fn nothing_is_called_without_the_mod_or_in_the_arcade_game() {
    for (screen, called_shot) in [("match", false), ("arcade", true)] {
        let Some(mut script) = game(screen, called_shot) else {
            return;
        };
        click_over(&mut script, (300.0, 170.0));
        assert!(!state(&mut script).contains("called"), "{screen}");
        assert!(all_named(&script.runner.stage, &[], "calledShot").is_empty());
    }
}
