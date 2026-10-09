//! Reading what the game says: a number out of its state, the words on
//! the stage, the sounds it asked for.

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

use super::play::state;

/// The number that follows `before` in a state line.
pub fn number(state: &str, before: &str) -> Option<f32> {
    let rest = state.split(before).nth(1)?;
    let digits: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    digits.parse().ok()
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
    let exports = &script.runner.library().manifest.exports;
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

/// What one of the art's own text fields says.
pub fn text(script: &Script, name: &str) -> String {
    script
        .runner
        .stage
        .text(name)
        .unwrap_or_default()
        .to_owned()
}
