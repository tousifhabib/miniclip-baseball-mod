//! The game's sums are kept apart from its drawing: the files that work out
//! what happens never touch the stage, and so can be tested as sums.
//!
//! Nothing in the language stops one of them reaching for the stage one
//! day. This does, by reading them.

/// The files that are sums and nothing else, with what each holds.
const SUMS: [(&str, &str); 25] = [
    ("rng.rs", include_str!("../src/rng.rs")),
    ("rules.rs", include_str!("../src/rules.rs")),
    ("settings.rs", include_str!("../src/settings.rs")),
    ("scores.rs", include_str!("../src/scores.rs")),
    ("kept.rs", include_str!("../src/kept.rs")),
    ("locate.rs", include_str!("../src/locate.rs")),
    ("play/pitch.rs", include_str!("../src/play/pitch.rs")),
    ("play/field.rs", include_str!("../src/play/field.rs")),
    ("play/book.rs", include_str!("../src/play/book.rs")),
    ("play/paper.rs", include_str!("../src/play/paper.rs")),
    ("play/snapshot.rs", include_str!("../src/play/snapshot.rs")),
    ("play/mode.rs", include_str!("../src/play/mode.rs")),
    (
        "play/mods/butterfingers.rs",
        include_str!("../src/play/mods/butterfingers.rs"),
    ),
    (
        "play/mods/clutch.rs",
        include_str!("../src/play/mods/clutch.rs"),
    ),
    (
        "play/mods/heat_check.rs",
        include_str!("../src/play/mods/heat_check.rs"),
    ),
    (
        "play/mods/hot_bat.rs",
        include_str!("../src/play/mods/hot_bat.rs"),
    ),
    (
        "play/mods/knuckleball.rs",
        include_str!("../src/play/mods/knuckleball.rs"),
    ),
    (
        "play/mods/lone_pitcher.rs",
        include_str!("../src/play/mods/lone_pitcher.rs"),
    ),
    (
        "play/mods/moon_ball.rs",
        include_str!("../src/play/mods/moon_ball.rs"),
    ),
    (
        "play/mods/mystery_pitch.rs",
        include_str!("../src/play/mods/mystery_pitch.rs"),
    ),
    (
        "play/mods/pinball_park.rs",
        include_str!("../src/play/mods/pinball_park.rs"),
    ),
    (
        "play/mods/rally.rs",
        include_str!("../src/play/mods/rally.rs"),
    ),
    (
        "play/mods/sudden_death.rs",
        include_str!("../src/play/mods/sudden_death.rs"),
    ),
    (
        "play/mods/tired_arm.rs",
        include_str!("../src/play/mods/tired_arm.rs"),
    ),
    (
        "play/mods/turbo_runners.rs",
        include_str!("../src/play/mods/turbo_runners.rs"),
    ),
];

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
        .flat_map(|(file, text)| {
            barred
                .iter()
                .filter(|word| text.contains(*word))
                .map(move |word| format!("{file} names {word}"))
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
    let mods = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/play/mods");
    let mut read = 0;
    for entry in std::fs::read_dir(&mods).unwrap() {
        let file = entry.unwrap().path();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(
            !text.contains("impl Match"),
            "{} reaches into the match",
            file.display()
        );
        read += 1;
    }
    // Every mod's file, and the one that lists them.
    assert_eq!(read, 24, "in {}", mods.display());
}
