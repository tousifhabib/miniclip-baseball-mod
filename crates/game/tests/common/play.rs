//! Playing the game by written steps: a pitch swung at or let go by, and
//! the wait for the next.

use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use bb_engine::app::Runner;
use bb_engine::library::Library;
use bb_engine::stage::Stage;
use bb_game::art::all_named;
use bb_game::baseball::{Baseball, Screen};
use bb_game::mods::Mod;
use bb_game::rules::Rules;
use bb_game::script::Script;
use bb_game::settings::Ground;

use super::read::number;

/// The ring held on the ball: no way off where it will cross.
pub const ON: (f32, f32) = (0.0, 0.0);
/// A swing this many steps before the best misses the ball, and one this
/// many after never comes: the pitch is let go by.
pub const MISS: i32 = -6;
pub const LEAVE: i32 = 1000;

/// Follows `steps` and returns what the last `state` in them gave.
pub fn state_after(script: &mut Script, steps: &str) -> String {
    let lines = script.run(steps).expect("the steps to run");
    lines.last().cloned().unwrap_or_default()
}

pub fn state(script: &mut Script) -> String {
    script.run("state").unwrap().pop().unwrap_or_default()
}

/// Whether the game is still on the screen it was started on, or has not
/// yet got to it.
pub fn playing(state: &str) -> bool {
    ["Match,", "FullMatch,", "Arcade,", "Loading"]
        .iter()
        .any(|screen| state.starts_with(screen))
}

/// Plays one pitch. The swing begins `late` steps after the first step the
/// bar calls best, with the ring held `off` away from where the ball will
/// cross. `seen` is given the game after every frame. Returns the state
/// once the play is over: the next pitch is on offer, or the game has
/// ended.
pub fn pitch_seen(
    script: &mut Script,
    late: i32,
    off: (f32, f32),
    mut seen: impl FnMut(&Script, &str),
) -> String {
    // Far more frames than any pitch needs: a play that never ends fails
    // here instead of hanging the test.
    for _ in 0..20_000 {
        let now = state(script);
        seen(script, &now);
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

pub fn pitch(script: &mut Script, late: i32, off: (f32, f32)) -> String {
    pitch_seen(script, late, off, |_, _| {})
}

/// Asks for the next pitch.
pub fn next(script: &mut Script) {
    // The button takes a moment to come up.
    for _ in 0..200 {
        script.run("click 545 355; wait 2").unwrap();
        if !state(script).contains(": Ready") {
            return;
        }
    }
    panic!("the next pitch never came: {}", state(script));
}

/// Waits for the pitcher to stand ready for the next pitch, and returns how
/// things stand then.
pub fn ready(script: &mut Script) -> String {
    for _ in 0..600 {
        let now = state(script);
        if now.contains("Settling") {
            return now;
        }
        script.run("wait 1").unwrap();
    }
    panic!("the next pitch never came: {}", state(script));
}

/// Lets the next pitch go by, which walks the batter where the rules make
/// one ball a walk. Returns the score when he is on first.
pub fn walk(script: &mut Script) -> f32 {
    ready(script);
    let after = pitch(script, LEAVE, ON);
    next(script);
    number(&after, "score ").unwrap()
}

/// Swings at and misses three pitches: the batter is out.
pub fn strike_out(script: &mut Script) {
    for _ in 0..3 {
        ready(script);
        pitch(script, MISS, ON);
        next(script);
    }
}
