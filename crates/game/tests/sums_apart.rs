//! The game's sums are kept apart from its drawing: the files that work out
//! what happens never touch the stage, and so can be tested as sums.
//!
//! Nothing in the language stops one of them reaching for the stage one
//! day. This does, by reading them.

use std::fs;
use std::path::{Path, PathBuf};

/// The files that are sums and nothing else, as they are found under
/// `src`. A folder is named for every file in it.
const SUMS: [&str; 25] = [
    "rng.rs",
    "rules.rs",
    "settings.rs",
    "scores.rs",
    "kept.rs",
    "locate.rs",
    "play/pitch",
    "play/field",
    "play/book",
    "play/paper",
    "play/snapshot",
    "play/mode.rs",
    "play/mods/butterfingers.rs",
    "play/mods/clutch.rs",
    "play/mods/heat_check.rs",
    "play/mods/hot_bat.rs",
    "play/mods/knuckleball.rs",
    "play/mods/lone_pitcher.rs",
    "play/mods/moon_ball.rs",
    "play/mods/mystery_pitch.rs",
    "play/mods/pinball_park",
    "play/mods/rally.rs",
    "play/mods/sudden_death.rs",
    "play/mods/tired_arm.rs",
    "play/mods/turbo_runners.rs",
];

/// The game's own code.
fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every file of code at a place, which is one file or a folder of them,
/// with what each holds.
fn files_at(place: &Path) -> Vec<(PathBuf, String)> {
    if place.is_dir() {
        let mut inside: Vec<PathBuf> = fs::read_dir(place)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        inside.sort();
        return inside.iter().flat_map(|path| files_at(path)).collect();
    }
    let text = fs::read_to_string(place)
        .unwrap_or_else(|fault| panic!("{} is not to be read: {fault}", place.display()));
    vec![(place.to_owned(), text)]
}

#[test]
fn the_files_that_are_sums_never_name_the_stage() {
    // A colour or a place is a plain thing and may be named. The stage,
    // the art it plays from and the tree of clips may not.
    let barred = [
        "Stage",
        "bb_engine::library",
        "bb_engine::display",
        "bb_engine::app",
    ];
    let reaching: Vec<String> = SUMS
        .iter()
        .flat_map(|sums| files_at(&src().join(sums)))
        .flat_map(|(file, text)| {
            barred
                .iter()
                .filter(|word| text.contains(*word))
                .map(|word| format!("{} names {word}", file.display()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        reaching.is_empty(),
        "these are meant to be sums and nothing else: {reaching:#?}"
    );
}

#[test]
fn no_mod_does_its_work_as_a_function_of_the_match() {
    // A mod answers what it is asked and says what to write. It is the
    // play that acts on the answer, where it does everything else.
    let mods = src().join("play/mods");
    for (file, text) in files_at(&mods) {
        assert!(
            !text.contains("impl Match"),
            "{} reaches into the match",
            file.display()
        );
    }
    // Every mod's file or folder, the file that lists them, and the folder
    // of what the game asks them.
    let listed = fs::read_dir(&mods).unwrap().count();
    assert_eq!(listed, 25, "in {}", mods.display());
}
