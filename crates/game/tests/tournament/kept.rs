//! A tournament kept from one run of the game to the next.

use std::path::PathBuf;

use bb_game::tournament::Format;

use super::common::{full_match_played, short_tournament_kept, state, state_after, written};
use super::fixtures::{on_to_the_tables, to_the_first_pitch, to_the_summary};

/// A folder of this test's own to keep a tournament in, with nothing in
/// it.
fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("bb-kept-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    folder
}

/// Where things are on the menu's summary of a tournament.
const GIVE_IT_UP: &str = "click 450 292";

#[test]
fn a_tournament_is_kept_once_it_is_begun_and_picked_up_by_the_next_run() {
    let folder = folder("picked-up");
    let file = folder.join("tournament.toml");
    let Some(mut script) = short_tournament_kept(4, Format::League, &[], Some(&file)) else {
        return;
    };
    // Drawn, and looked at, it is not worth keeping yet.
    to_the_summary(&mut script);
    assert!(!file.exists());
    // Begun, it is.
    to_the_first_pitch(&mut script);
    assert!(file.exists());
    let over = full_match_played(&mut script, false);
    assert!(over.starts_with("MatchLost,"), "{over}");
    let tables = on_to_the_tables(&mut script);
    assert!(
        tables.contains("played 3 of 15, ROUND 2, next "),
        "{tables}"
    );
    let ours = written(&script, "tableOurs");
    // Another run of the game, by another seed, has the same tournament.
    let Some(mut again) = short_tournament_kept(99, Format::Cup, &[], Some(&file)) else {
        return;
    };
    assert_eq!(state_after(&mut again, "wait 120; state"), tables);
    assert_eq!(written(&again, "tableOurs"), ours);
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn one_that_is_given_up_is_kept_no_longer() {
    let folder = folder("given-up");
    let file = folder.join("tournament.toml");
    let Some(mut script) = short_tournament_kept(4, Format::League, &[], Some(&file)) else {
        return;
    };
    to_the_summary(&mut script);
    to_the_first_pitch(&mut script);
    full_match_played(&mut script, false);
    on_to_the_tables(&mut script);
    to_the_summary(&mut script);
    assert!(file.exists());
    let given_up = format!("{GIVE_IT_UP}; wait 5; {GIVE_IT_UP}; wait 70; state");
    assert_eq!(state_after(&mut script, &given_up), "Menu, Main, Medium");
    assert!(!file.exists());
    // The next run has one of its own to draw.
    let Some(mut again) = short_tournament_kept(99, Format::Cup, &[], Some(&file)) else {
        return;
    };
    let fresh = state_after(&mut again, "wait 120; state");
    assert!(
        fresh.contains("cup of 8, 1 innings, played 0 of 7, "),
        "{fresh}"
    );
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn a_file_that_is_not_a_tournament_is_none_kept() {
    let folder = folder("damaged");
    let file = folder.join("tournament.toml");
    std::fs::create_dir_all(&folder).expect("a folder");
    std::fs::write(&file, "version = 1\nformat = \"ladder\"\n").expect("a file");
    let Some(mut script) = short_tournament_kept(4, Format::League, &[], Some(&file)) else {
        return;
    };
    script.run("wait 120").unwrap();
    let fresh = state(&mut script);
    assert!(
        fresh.contains("league of 6, 1 innings, played 0 of 15, "),
        "{fresh}"
    );
    let _ = std::fs::remove_dir_all(folder);
}
