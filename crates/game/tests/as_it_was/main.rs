//! Plays a great many games by written steps and checks that each still goes
//! exactly as it did when it was last written down.
//!
//! This is the net under any change that is meant to leave the game as it
//! was. Every frame of every game, three things are added to running sums:
//! what the game said of itself, what was heard, and everything on the
//! stage. Every so many frames the sums are compared with the ones written
//! down in the folder beside this file. A game that has gone differently
//! fails, and says which game, by which frame, and in which of the three.
//!
//! - `cargo test -p bb-game --test as_it_was` plays the start of every game.
//! - `BB_WHOLE=1` plays each to its end. `scripts/as-it-was.sh` does that
//!   with the game built to run fast.
//! - `BB_WRITE_DOWN=1` writes down how the games go now, in place of what
//!   was written before. Only a change that is meant to alter the game
//!   should need it, and it should say so.
//! - `BB_TRACES=folder`, with `BB_ONLY=part of a name` and perhaps
//!   `BB_FROM=frame` and `BB_TO=frame`, writes out in full what was taken in
//!   of those frames of those games, to compare by eye with the same from
//!   another version of the game. `scripts/what-changed.sh` does both and
//!   shows where they part.
//!
//! The triangles the art is cut into are written down beside the games, and
//! checked the same way: see `triangles.rs`.
//!
//! What is written down is for one kind of machine, since a few of the
//! game's sums are done by the machine's own mathematics. Where nothing is
//! written down for the machine in hand, each game is played twice and the
//! two are checked against each other.
//!
//! These tests need the extracted art. Where it is missing they pass
//! without checking anything, and say so.

mod games;
mod players;
mod playing;
mod record;
mod seeing;
mod sums;
mod triangles;

use std::path::PathBuf;

use record::check;

/// How many frames go by between one writing down of the sums and the next.
const EVERY: u32 = 600;

/// The folder that holds the art, if it is there.
fn extracted() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted");
    dir.join("manifest.json").exists().then_some(dir)
}

fn asked(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// The sums of a game after so many frames of it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Line {
    frame: u32,
    said: String,
    heard: String,
    seen: String,
}

#[test]
fn the_last_innings_goes_as_it_did_with_each_mod() {
    check("last-innings", games::last_innings());
}

#[test]
fn the_arcade_game_goes_as_it_did_with_each_mod() {
    check("arcade", games::arcade());
}

#[test]
fn a_full_match_at_home_goes_as_it_did_with_each_mod() {
    check("full-match-at-home", games::full_match_at_home());
}

#[test]
fn a_full_match_away_goes_as_it_did_with_each_mod() {
    check("full-match-away", games::full_match_away());
}

#[test]
fn mods_that_meet_go_together_as_they_did() {
    check("mods-together", games::mods_together());
}

#[test]
fn a_mod_set_to_its_least_and_its_most_goes_as_it_did() {
    check("levels", games::levels());
}

#[test]
fn the_easy_and_the_hard_game_go_as_they_did() {
    check("skills", games::skills());
}

#[test]
fn the_menu_goes_as_it_did() {
    check("menu", games::the_menu());
}

#[test]
fn a_match_played_to_its_end_and_its_pages_go_as_they_did() {
    check("finished-matches", games::finished_matches());
}

#[test]
fn a_tournament_goes_as_it_did() {
    check("tournament", games::tournaments());
}

#[test]
fn a_monkey_at_the_controls_gets_what_it_got() {
    check("monkeys", games::monkeys());
}

#[test]
fn whole_matches_go_as_they_did() {
    // Nine innings take too long to play every time.
    let asked_for = ["BB_WHOLE", "BB_WRITE_DOWN", "BB_TRACES"];
    if asked_for.iter().all(|name| asked(name).is_none()) {
        return;
    }
    check("whole-matches", games::whole_matches());
}
