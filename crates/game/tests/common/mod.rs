//! Starts the real game with no window, for tests that drive it with
//! written steps.
//!
//! These tests need the extracted art. Where it is missing they pass
//! without checking anything, and say so.

#![allow(
    dead_code,
    reason = "each file of tests uses some of these and not the rest"
)]

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

/// The folder that holds the art, if it is there.
fn extracted() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted");
    dir.join("manifest.json").exists().then_some(dir)
}

/// The art, read once for all the tests of a file and played from by each.
/// `None` when there is none to read.
static ART: LazyLock<Option<Arc<Library>>> = LazyLock::new(|| {
    let library = Library::load(&extracted()?).expect("loading the extracted art");
    Some(Arc::new(library))
});

/// The game, opened on the screen with this label. `None` when there is no
/// extracted art to play.
pub fn game(screen: &str) -> Option<Script> {
    game_with(screen, Some(1))
}

/// The game opened on a screen, with its chances worked out from `seed`.
pub fn game_with(screen: &str, seed: Option<u64>) -> Option<Script> {
    game_ruled(screen, seed, None)
}

/// The same, played by `rules` instead of the ones built in.
pub fn game_ruled(screen: &str, seed: Option<u64>, rules: Option<Rules>) -> Option<Script> {
    game_made(screen, seed, rules, &[], None, None, None)
}

/// The game opened on a screen, with these mods switched on.
pub fn game_modded(screen: &str, seed: u64, mods: &[Mod]) -> Option<Script> {
    game_made(screen, Some(seed), None, mods, None, None, None)
}

/// A full match with these mods on, played at home or away, by `rules` if
/// any are given.
pub fn full_match(seed: u64, ground: Ground, mods: &[Mod], rules: Option<Rules>) -> Option<Script> {
    let screen = Screen::FULL_MATCH;
    game_made(screen, Some(seed), rules, mods, None, None, Some(ground))
}

/// A match with these mods on that takes a long time to win or lose, for
/// tests that need a good many pitches: forty runs behind, with thirty outs
/// to get them in.
pub fn long_match(seed: u64, mods: &[Mod]) -> Option<Script> {
    long_match_ruled(seed, mods, "")
}

/// The same, with the numbers in `layer` laid over the rules: what a test
/// changes of them to see a mod's whole effect in a few pitches.
pub fn long_match_ruled(seed: u64, mods: &[Mod], layer: &str) -> Option<Script> {
    let long = "[match]\nouts = 30\n[match.runs_down]\neasy = 40\nmedium = 40\nhard = 40\n";
    let rules = Rules::layered(&[("a long match", long), ("the test's own rules", layer)])
        .expect("rules that read");
    game_made("match", Some(seed), Some(rules), mods, None, None, None)
}

/// The same, keeping its scores in `scores` and starting from what is
/// there.
pub fn game_keeping(screen: &str, seed: u64, mods: &[Mod], scores: &Path) -> Option<Script> {
    game_made(screen, Some(seed), None, mods, Some(scores), None, None)
}

/// The game opened on a screen with one mod switched on, its setting at
/// `level`, and the timing bar, which says when to swing.
pub fn game_levelled(screen: &str, seed: u64, which: Mod, level: u8) -> Option<Script> {
    let mods = [Mod::TimingIndicator, which];
    game_made(
        screen,
        Some(seed),
        None,
        &mods,
        None,
        Some((which, level)),
        None,
    )
}

fn game_made(
    screen: &str,
    seed: Option<u64>,
    rules: Option<Rules>,
    mods: &[Mod],
    scores: Option<&Path>,
    level: Option<(Mod, u8)>,
    ground: Option<Ground>,
) -> Option<Script> {
    let Some(library) = ART.clone() else {
        eprintln!("skipped: there is no extracted art to play");
        return None;
    };
    let stage = Stage::new(None, &library);
    let mut logic = Box::new(Baseball::new(&library));
    logic.start_on(Screen::from_label(screen).expect("a screen with that label"));
    if let Some(seed) = seed {
        logic.seed(seed);
    }
    if let Some(rules) = rules {
        logic.play_by(rules);
    }
    for &which in mods {
        logic.switch_mod(which, true);
    }
    if let Some(scores) = scores {
        logic.keep_scores_in(scores.to_owned());
    }
    if let Some((which, level)) = level {
        logic.set_mod_level(which, level);
    }
    if let Some(ground) = ground {
        logic.play_on(ground);
    }
    let runner = Runner::new(library, stage, logic, None);
    Some(Script::new(runner).expect("a renderer with no window"))
}

/// Follows `steps` and returns what the last `state` in them gave.
pub fn state_after(script: &mut Script, steps: &str) -> String {
    let lines = script.run(steps).expect("the steps to run");
    lines.last().cloned().unwrap_or_default()
}

/// The number that follows `before` in a state line.
pub fn number(state: &str, before: &str) -> Option<f32> {
    let rest = state.split(before).nth(1)?;
    let digits: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    digits.parse().ok()
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

/// What the words with this name on the stage say, where they can be seen.
/// Each line is there twice, once as its own shadow.
pub fn said(script: &Script, name: &str) -> Vec<String> {
    let stage = &script.runner.stage;
    let mut said: Vec<String> = all_named(stage, &[], name)
        .iter()
        .map(|words| stage.child(words).unwrap())
        .filter(|words| words.visible)
        .filter_map(|words| words.said.clone())
        .collect();
    said.dedup();
    said
}

/// What every line of words with this name says, where they can be seen,
/// in the order they were written. Unlike [`said`], lines that say the same
/// thing one after another are each given.
pub fn written(script: &Script, name: &str) -> Vec<String> {
    let stage = &script.runner.stage;
    let all: Vec<String> = all_named(stage, &[], name)
        .iter()
        .map(|words| stage.child(words).unwrap())
        .filter(|words| words.visible)
        .filter_map(|words| words.said.clone())
        .collect();
    // Each line is there twice, once as its own shadow.
    all.chunks(2).map(|pair| pair[0].clone()).collect()
}

/// The sounds asked for since this was last asked, by their names in the
/// art.
pub fn sounds(script: &mut Script) -> Vec<String> {
    let lines = script.run("events").unwrap();
    let exports = &script.runner.library.manifest.exports;
    lines
        .iter()
        .filter_map(|line| line.trim().strip_prefix("sound ")?.parse::<u16>().ok())
        .filter_map(|id| {
            exports
                .iter()
                .find(|(_, symbol)| **symbol == id)
                .map(|(name, _)| name.clone())
        })
        .collect()
}
