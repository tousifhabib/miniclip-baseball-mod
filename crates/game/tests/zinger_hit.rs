//! The zinger hit mod: every ball the bat meets in a match is a home run,
//! and the better the swing was timed the further it goes.

mod common;

use bb_game::art::all_named;
use bb_game::mods::Mod;
use bb_game::script::Script;
use common::game_modded;

/// The number that follows `before` in a state line.
fn number(state: &str, before: &str) -> Option<f32> {
    let rest = state.split(before).nth(1)?;
    let digits: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    digits.parse().ok()
}

fn state(script: &mut Script) -> String {
    script.run("state").unwrap().pop().unwrap_or_default()
}

/// Whether the game is still on the screen it was started on, or has not
/// yet got to it.
fn playing(state: &str) -> bool {
    ["Match,", "Arcade,", "Loading"]
        .iter()
        .any(|screen| state.starts_with(screen))
}

/// A game with the timing bar up, which says when to swing, and with the
/// zinger mod on or off.
fn game(screen: &str, seed: u64, zinger: bool) -> Option<Script> {
    let mut mods = vec![Mod::TimingIndicator];
    if zinger {
        mods.push(Mod::ZingerHit);
    }
    game_modded(screen, seed, &mods)
}

/// Plays one pitch. The swing begins `late` steps after the first step the
/// bar calls best, with the ring held `off` away from where the ball will
/// cross. Returns the state once the play is over: the next pitch is on
/// offer, or the game has ended.
fn pitch(script: &mut Script, late: i32, off: (f32, f32)) -> String {
    // Far more frames than any pitch needs: a play that never ends fails
    // here instead of hanging the test.
    for _ in 0..20_000 {
        let now = state(script);
        if !playing(&now) || now.contains(": Ready") {
            return now;
        }
        let ring = number(&now, "crossing ")
            .zip(number(now.split("crossing ").nth(1).unwrap_or(""), ","))
            .map(|(x, y)| (x + off.0, y + off.1));
        let step = number(&now, "Flight { step: ").map(|step| step as i32);
        let best = number(&now, "best swung on steps ").map(|best| best as i32);
        let steps = match (ring, step, best) {
            (Some((x, y)), Some(step), Some(best)) if step == best + late => {
                format!("click {x} {y}")
            }
            (Some((x, y)), None, _) if now.contains("Settling") => {
                format!("move {x} {y}; wait 1")
            }
            _ => "wait 1".to_owned(),
        };
        script.run(&steps).unwrap();
    }
    panic!("the pitch never ended: {}", state(script));
}

/// Asks for the next pitch.
fn next(script: &mut Script) {
    // The button takes a moment to come up.
    for _ in 0..200 {
        script.run("click 545 355; wait 2").unwrap();
        if !state(script).contains(": Ready") {
            return;
        }
    }
    panic!("the next pitch never came: {}", state(script));
}

/// How far the state says the ball went, if it was hit for a zinger.
fn feet(state: &str) -> Option<u32> {
    number(state, "a zinger of ").map(|feet| feet as u32)
}

/// What is written under the home-run banner.
fn told(script: &Script) -> Vec<String> {
    let stage = &script.runner.stage;
    all_named(stage, &[], "zingerFeet")
        .iter()
        .map(|words| stage.child(words).unwrap())
        .filter(|words| words.visible)
        .filter_map(|words| words.said.clone())
        .collect()
}

/// Ways to hold the ring that spoil a hit as the game was, each with a
/// game whose first pitch it spoils: far above the ball, which tops it into
/// the ground, far below, which skies it, and far to either side, which
/// sends it foul past one line or the other.
const SPOILT: [(u64, (f32, f32)); 4] = [
    (1, (0.0, -80.0)),
    (1, (0.0, 80.0)),
    (1, (-140.0, 0.0)),
    (4, (170.0, 0.0)),
];

#[test]
fn every_ball_the_bat_meets_is_a_home_run() {
    for seed in [1, 2, 3] {
        // From the earliest swing that meets the ball to one of the latest.
        for late in -1..=4 {
            let Some(mut script) = game("match", seed, true) else {
                return;
            };
            let after = pitch(&mut script, late, (0.0, 0.0));
            assert!(after.contains("score 1 of"), "seed {seed}, {late}: {after}");
            assert!(after.contains("outs 0, count 0-0, bases ---"), "{after}");
            assert!(feet(&after).is_some(), "seed {seed}, {late}: {after}");
        }
    }
}

#[test]
fn it_is_a_home_run_wherever_the_ring_was_held() {
    for (seed, off) in SPOILT {
        let Some(mut usual) = game("match", seed, false) else {
            return;
        };
        // As the game was, none of these gets a run.
        let after = pitch(&mut usual, 0, off);
        assert!(after.contains("score 0 of"), "{off:?}: {after}");
        assert_eq!(feet(&after), None);

        let mut zinger = game("match", seed, true).unwrap();
        let after = pitch(&mut zinger, 0, off);
        assert!(after.contains("score 1 of"), "{off:?}: {after}");
        assert!(after.contains("outs 0, count 0-0, bases ---"), "{after}");
        // The swing was timed as well as it could be, and the ring does not
        // come into how far the ball goes.
        assert_eq!(feet(&after), Some(800), "{off:?}: {after}");
    }
}

#[test]
fn the_better_the_swing_is_timed_the_further_the_ball_goes() {
    for seed in [1, 2, 3] {
        // Later and later after the best moment, as far as the last swing
        // that still meets the ball.
        let mut went = Vec::new();
        for late in 0..=4 {
            let Some(mut script) = game("match", seed, true) else {
                return;
            };
            went.push(feet(&pitch(&mut script, late, (0.0, 0.0))).unwrap());
        }
        assert!(went.is_sorted_by(|a, b| a >= b), "seed {seed}: {went:?}");
        // From the furthest there is to just over the wall.
        assert_eq!((went[0], went[4]), (800, 440), "seed {seed}: {went:?}");
        assert!(
            went[0] > went[2] && went[2] > went[4],
            "seed {seed}: {went:?}"
        );

        // Too early by the same amount is no better than too late.
        let mut script = game("match", seed, true).unwrap();
        let early = feet(&pitch(&mut script, -1, (0.0, 0.0))).unwrap();
        assert!(early < went[0], "seed {seed}: {early}");
    }
}

#[test]
fn the_player_is_told_how_far_it_went_until_the_next_pitch() {
    let Some(mut script) = game("match", 1, true) else {
        return;
    };
    assert!(told(&script).is_empty());
    pitch(&mut script, 0, (0.0, 0.0));
    assert_eq!(told(&script), ["800 FT", "800 FT"]);
    next(&mut script);
    script.run("wait 60").unwrap();
    assert!(told(&script).is_empty());
    // A swing not so well timed is told as it is.
    pitch(&mut script, 3, (0.0, 0.0));
    let said = told(&script);
    assert_eq!(said.len(), 2, "{said:?}");
    assert_ne!(said[0], "800 FT");
    assert!(state(&mut script).contains("score 2 of"));
}

#[test]
fn a_swing_that_misses_is_still_a_strike() {
    let Some(mut script) = game("match", 1, true) else {
        return;
    };
    let after = pitch(&mut script, -6, (0.0, 0.0));
    assert!(after.contains("score 0 of"), "{after}");
    assert!(after.contains("count 0-1"), "{after}");
    assert_eq!(feet(&after), None);
    assert!(told(&script).is_empty());
}

#[test]
fn a_match_is_won_by_as_many_hits_as_there_are_runs_to_get() {
    let Some(mut script) = game("match", 1, true) else {
        return;
    };
    let mut pitches = 0;
    let end = loop {
        let (_, off) = SPOILT[pitches % SPOILT.len()];
        let now = pitch(&mut script, 0, off);
        if !playing(&now) {
            break now;
        }
        pitches += 1;
        assert!(pitches <= 3, "{now}");
        assert!(now.contains(&format!("score {pitches} of 3")), "{now}");
        next(&mut script);
    };
    assert!(end.starts_with("MatchWon"), "{end}");
    assert_eq!(pitches, 3);
}

#[test]
fn without_the_mod_a_poorly_timed_swing_is_no_home_run() {
    let Some(mut script) = game("match", 1, false) else {
        return;
    };
    let after = pitch(&mut script, 3, (0.0, 0.0));
    assert!(after.contains("score 0 of"), "{after}");
    assert_eq!(feet(&after), None);
    assert!(told(&script).is_empty());
}

#[test]
fn the_arcade_game_is_left_as_it_was() {
    // The same swings at the same pitches come to the same points, with
    // the mod on or off.
    let mut ends = Vec::new();
    for zinger in [false, true] {
        let Some(mut script) = game("arcade", 1, zinger) else {
            return;
        };
        let mut states = Vec::new();
        for late in [0, 1, 2, 0, 3] {
            let after = pitch(&mut script, late, (0.0, 0.0));
            assert_eq!(feet(&after), None, "{after}");
            // The ball is still being followed over the field when the
            // next pitch is offered: let it finish.
            script.run("wait 300").unwrap();
            states.push(state(&mut script));
            next(&mut script);
        }
        assert!(told(&script).is_empty());
        ends.push(states);
    }
    assert_eq!(ends[0], ends[1]);
}
