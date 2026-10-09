//! The stolen bases mod: a runner sent on while the pitcher winds up, and
//! the catcher's throw to put him out.

mod book;

#[path = "../common/mod.rs"]
mod common;

use bb_game::art::all_named;
use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{
    LEAVE, MISS, ON, crossing, frame_of_a_pitch, long_match_ruled, next, number, pitch, ready,
    said, state, timing_bar_and,
};

/// A click on the little field in the corner of the batting view.
const LITTLE_FIELD: &str = "click 60 45";
/// One ball walks the batter, so that there is soon a runner on first, and
/// the catcher always takes as long over his throw.
const QUICK: &str = "[count]\nballs = 1\n[steal]\npop = { low = 30, high = 30 }\n";

fn game(steals: bool) -> Option<Script> {
    long_match_ruled(1, &timing_bar_and(Mod::StolenBases, steals), QUICK)
}

/// Lets pitches go by until a batter has walked to first with nobody else
/// on, and the pitcher stands ready for the next.
fn runner_on_first(script: &mut Script) -> String {
    for _ in 0..60 {
        let now = ready(script);
        if now.contains("bases x--") {
            return now;
        }
        pitch(script, LEAVE, (0.0, 0.0));
        next(script);
    }
    panic!("nobody walked: {}", state(script));
}

/// Plays the pitch that is coming. The little field is clicked on the
/// `send`th frame of the wind-up, if one is given, and the bat is swung
/// `late` steps after the best step for it. Returns how things stand when
/// the play is over.
fn play(script: &mut Script, send: Option<u32>, late: i32) -> String {
    play_sending(script, send.as_slice(), late)
}

/// The same, with the little field clicked on each of these frames of the
/// wind-up, which sends a runner each time one may go.
fn play_sending(script: &mut Script, sends: &[u32], late: i32) -> String {
    let mut wound = 0;
    for _ in 0..20_000 {
        let now = state(script);
        if !now.starts_with("Match,") && !now.starts_with("FullMatch,") || now.contains(": Ready") {
            return now;
        }
        wound += u32::from(now.contains("WindUp"));
        let steps = match crossing(&now) {
            Some((x, y)) if now.contains("WindUp") && sends.contains(&wound) => {
                // And back to the ball.
                format!("{LITTLE_FIELD}; move {x} {y}")
            }
            _ => frame_of_a_pitch(&now, late, ON),
        };
        script.run(&steps).unwrap();
    }
    panic!("the pitch never ended: {}", state(script));
}

/// How many runners are marked on the little field.
fn marked(script: &Script) -> usize {
    all_named(&script.runner.stage, &[], "lead").len()
}

#[test]
fn a_runner_sent_early_steals_the_base_and_the_count_goes_on() {
    let Some(mut script) = game(true) else {
        return;
    };
    let before = runner_on_first(&mut script);
    let outs = number(&before, "outs ").unwrap();
    let after = play(&mut script, Some(10), MISS);
    // He is on second, nobody is out, and the swing that missed is a
    // strike on the batter.
    assert!(after.contains("bases -x-"), "{after}");
    assert!(after.contains(", stolen 1, caught 0"), "{after}");
    assert!(
        after.contains(&format!("outs {outs}, count 0-1")),
        "{after}"
    );
    assert_eq!(said(&script, "stealNews"), ["STOLEN BASE!"]);
    // From second he can be sent for third.
    next(&mut script);
    ready(&mut script);
    let third = play(&mut script, Some(5), LEAVE);
    assert!(third.contains(", stolen 2, caught 0"), "{third}");
    // And from third there is nowhere to steal to.
    next(&mut script);
    if ready(&mut script).contains("bases --x") {
        let home = play(&mut script, Some(5), MISS);
        assert!(home.contains(", stolen 2, caught 0"), "{home}");
    }
}

#[test]
fn a_runner_sent_late_is_thrown_out_and_the_batter_bats_on() {
    let Some(mut script) = game(true) else {
        return;
    };
    let before = runner_on_first(&mut script);
    let outs = number(&before, "outs ").unwrap();
    let after = play(&mut script, Some(65), MISS);
    assert!(after.contains("bases ---"), "{after}");
    assert!(after.contains(", stolen 0, caught 1"), "{after}");
    assert!(
        after.contains(&format!("outs {}, count 0-1", outs + 1.0)),
        "{after}"
    );
    assert_eq!(said(&script, "stealNews"), ["CAUGHT STEALING!"]);
}

#[test]
fn a_steal_that_is_settled_as_the_play_ends_is_told_then_and_not_on_the_play_after() {
    // With the pitcher fielding alone nobody throws the ball on: the play
    // ends where the catcher's throw does, and a runner still on his way
    // is given his base.
    let mods = [Mod::TimingIndicator, Mod::StolenBases, Mod::LonePitcher];
    let Some(mut script) = long_match_ruled(1, &mods, QUICK) else {
        return;
    };
    // Pitches are let go by until two batters have walked, and there are
    // runners on first and second.
    let mut before = runner_on_first(&mut script);
    for _ in 0..60 {
        if before.contains("bases xx-") {
            break;
        }
        pitch(&mut script, LEAVE, (0.0, 0.0));
        next(&mut script);
        before = ready(&mut script);
    }
    assert!(before.contains("bases xx-"), "{before}");
    // The one on second goes early and is there before the throw. The one
    // on first goes late, and is still running when the play ends.
    let after = play_sending(&mut script, &[5, 60], MISS);
    assert!(after.contains("bases -xx"), "{after}");
    assert!(after.contains(", stolen 2, caught 0"), "{after}");
    // Both have been told by now, and when the telling has gone there is
    // nothing left to tell.
    assert_eq!(said(&script, "stealNews"), ["STOLEN BASE!"]);
    script.run("wait 200").unwrap();
    assert!(said(&script, "stealNews").is_empty());
    // So the next ball put in play says nothing of a steal.
    next(&mut script);
    ready(&mut script);
    let mut told = Vec::new();
    for _ in 0..2000 {
        let now = state(&mut script);
        told.extend(said(&script, "stealNews"));
        if now.contains(": Ready") {
            break;
        }
        let ring =
            number(&now, "crossing ").zip(number(now.split("crossing ").nth(1).unwrap_or(""), ","));
        let step = number(&now, "Flight { step: ").map(|step| step as i32);
        let best = number(&now, "best swung on steps ").map(|best| best as i32);
        let steps = match (ring, step, best) {
            (Some((x, y)), Some(step), Some(best)) if step == best => format!("click {x} {y}"),
            (Some((x, y)), None, _) if now.contains("Settling") => format!("move {x} {y}; wait 1"),
            _ => "wait 1".to_owned(),
        };
        script.run(&steps).unwrap();
    }
    assert!(told.is_empty(), "{told:?}");
}

#[test]
fn the_little_field_marks_the_runner_and_asks_for_the_click_in_the_wind_up() {
    let Some(mut script) = game(true) else {
        return;
    };
    ready(&mut script);
    // Nobody on, nothing marked.
    assert_eq!(marked(&script), 0);
    runner_on_first(&mut script);
    assert_eq!(marked(&script), 1);
    assert!(said(&script, "steal").is_empty());
    // A click before the wind-up sends nobody.
    script.run(&format!("{LITTLE_FIELD}; wait 2")).unwrap();
    assert!(!state(&mut script).contains("stealing"));
    for _ in 0..600 {
        if state(&mut script).contains("WindUp") {
            break;
        }
        script.run("wait 1").unwrap();
    }
    script.run("wait 1").unwrap();
    assert_eq!(said(&script, "steal"), ["CLICK TO STEAL"]);
    // Nor does a click anywhere else.
    script.run("click 300 300; wait 1").unwrap();
    assert!(!state(&mut script).contains("stealing"));
    script.run(&format!("{LITTLE_FIELD}; wait 1")).unwrap();
    assert!(state(&mut script).contains(", stealing 2"));
    assert_eq!(said(&script, "steal"), ["RUNNER GOING"]);
}

#[test]
fn a_pitch_that_is_hit_is_no_steal() {
    let Some(mut script) = game(true) else {
        return;
    };
    runner_on_first(&mut script);
    let after = play(&mut script, Some(10), 0);
    assert!(!after.contains("stolen"), "{after}");
    assert!(!after.contains("stealing"), "{after}");
    assert!(said(&script, "stealNews").is_empty());
}

#[test]
fn without_the_mod_nobody_is_marked_and_nobody_goes() {
    let Some(mut script) = game(false) else {
        return;
    };
    runner_on_first(&mut script);
    assert_eq!(marked(&script), 0);
    let after = play(&mut script, Some(10), MISS);
    assert!(after.contains("bases x--"), "{after}");
    assert!(!after.contains("stolen"), "{after}");
    assert!(said(&script, "steal").is_empty());
}
