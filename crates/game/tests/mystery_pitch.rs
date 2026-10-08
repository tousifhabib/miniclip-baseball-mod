//! The mystery pitch mod: a fastball, a change-up or a curve, and no way of
//! telling which until it has been thrown.

mod common;

use std::collections::BTreeMap;

use bb_game::mods::Mod;
use bb_game::script::Script;
use common::{game_modded, number, said, state};

/// Where the marker of where the pitch will cross is in the batting view.
fn marker(script: &Script) -> (f32, f32) {
    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain").unwrap();
    let path = stage.find(&main, &["ballPassesBat_marker"]).unwrap();
    let child = stage.child(&path).unwrap();
    (child.matrix.tx, child.matrix.ty)
}

/// What is known of a first pitch that was watched go by.
struct Watched {
    kind: String,
    /// Frames the pitch took, and frames the pitcher stood before his
    /// wind-up.
    frames: u32,
    stood: u32,
    /// Whether the marker moved, or the pitch was named, before the ball
    /// left his hand, and what it was named after.
    told_early: bool,
    named: Vec<String>,
}

fn watch(seed: u64, mystery: bool) -> Option<Watched> {
    let mods: &[Mod] = if mystery { &[Mod::MysteryPitch] } else { &[] };
    let mut script = game_modded("match", seed, mods)?;
    let mut watched = Watched {
        kind: String::new(),
        frames: 0,
        stood: 0,
        told_early: false,
        named: Vec::new(),
    };
    let mut at_first = None;
    for _ in 0..2000 {
        let now = state(&mut script);
        if now.contains(": Called") || now.contains(": Ready") {
            break;
        }
        watched.stood += u32::from(now.contains("Settling"));
        if now.contains("Settling") || now.contains("WindUp") {
            let at = marker(&script);
            watched.told_early |= *at_first.get_or_insert(at) != at;
            watched.told_early |= !said(&script, "mysteryPitch").is_empty();
        }
        if now.contains("Flight") {
            watched.frames = number(&now, " after ").unwrap() as u32;
            watched.kind = now.split("mystery ").nth(1).unwrap_or_default().to_owned();
            // The name comes down again before a slow pitch is in.
            if watched.named.is_empty() {
                watched.named = said(&script, "mysteryPitch");
            }
        }
        script.run("wait 1").unwrap();
    }
    Some(watched)
}

#[test]
fn a_pitch_is_one_of_three_kinds_and_is_named_as_it_is_thrown() {
    let mut frames: BTreeMap<String, u32> = BTreeMap::new();
    let mut stood = Vec::new();
    for seed in 1..=12 {
        let Some(pitch) = watch(seed, true) else {
            return;
        };
        // Nothing gives it away beforehand, and then it is named.
        assert!(!pitch.told_early, "seed {seed}");
        assert_eq!(pitch.named, [pitch.kind.to_uppercase()], "seed {seed}");
        frames.insert(pitch.kind, pitch.frames);
        stood.push(pitch.stood);
    }
    let kinds: Vec<&str> = frames.keys().map(String::as_str).collect();
    assert_eq!(kinds, ["change-up", "curve", "fastball"]);
    // A curve takes as long as any pitch at this skill level.
    let usual = watch(1, false).unwrap();
    assert_eq!(frames["curve"], usual.frames);
    assert!(frames["fastball"] * 10 < usual.frames * 8, "{frames:?}");
    assert!(frames["change-up"] * 10 > usual.frames * 13, "{frames:?}");
    // The pitcher stands as long before one kind as before another.
    assert!(stood.iter().all(|&frames| frames == stood[0]), "{stood:?}");
    assert_eq!(stood[0], usual.stood);
}

#[test]
fn without_the_mod_the_marker_shows_before_the_pitch_and_nothing_is_named() {
    let Some(pitch) = watch(1, false) else {
        return;
    };
    assert!(pitch.told_early);
    assert!(pitch.named.is_empty());
    assert_eq!(pitch.kind, "");
}
