//! The way to a full match through the menu: the choice of ground, and
//! the summary before the first ball.

use bb_game::art;

use crate::common;
use crate::common::{game, said, state_after};

/// The row of the menu's first page that leads to the full match, and the
/// buttons on the pages after it.
const FULL_MATCH: &str = "click 200 216";
const AWAY: &str = "click 445 312";
const NEXT: &str = "click 490 362";
const BACK: &str = "click 290 362";
const PLAY_BALL: &str = "click 480 362";

#[test]
fn the_menu_leads_to_a_full_match_at_the_ground_chosen() {
    let Some(mut script) = game("menu") else {
        return;
    };
    let setup = state_after(
        &mut script,
        &format!("wait 60; {FULL_MATCH}; wait 90; state"),
    );
    assert_eq!(setup, "Menu, FullSetup, Medium");
    assert_eq!(said(&script, "groundWord"), ["Home", "Away", "Toss"]);
    let summary = state_after(
        &mut script,
        &format!("{AWAY}; wait 5; {NEXT}; wait 90; state"),
    );
    assert_eq!(summary, "Menu, FullSummary, Medium");
    let lines = said(&script, "fullLine");
    assert_eq!(lines[0], "IT'S A FULL MATCH OF 9 INNINGS!");
    assert_eq!(
        lines[1..3],
        ["As you asked,", "you are away and bat first."]
    );
    // The art's own words about the last innings are out of sight.
    let stage = &script.runner.stage;
    let menu = art::in_shell(stage, art::MENU).expect("the menu");
    let theirs = stage
        .clip(&menu)
        .unwrap()
        .children
        .values()
        .filter(|child| art::SUMMARY_WORDS.contains(&child.symbol))
        .collect::<Vec<_>>();
    assert!(!theirs.is_empty());
    assert!(theirs.iter().all(|child| !child.visible));
    let playing = state_after(&mut script, &format!("{PLAY_BALL}; wait 120; state"));
    assert!(playing.starts_with("FullMatch,"), "{playing}");
    assert!(playing.contains("top of innings 1"), "{playing}");
}

#[test]
fn left_to_the_toss_the_match_is_played_where_the_summary_says() {
    let mut seen = [false, false];
    for seed in 1..=8 {
        let Some(mut script) = common::game_with("menu", Some(seed)) else {
            return;
        };
        let steps = format!("wait 60; {FULL_MATCH}; wait 90; {NEXT}; wait 90; state");
        assert_eq!(
            state_after(&mut script, &steps),
            "Menu, FullSummary, Medium"
        );
        let lines = said(&script, "fullLine");
        assert_eq!(lines[1], "The coin is tossed, and");
        let home = lines[2] == "you are at home and bat second.";
        assert!(
            home || lines[2] == "you are away and bat first.",
            "{lines:?}"
        );
        seen[usize::from(home)] = true;
        let playing = state_after(&mut script, &format!("{PLAY_BALL}; wait 120; state"));
        let (screen, ground) = if home {
            ("Interval,", "at home")
        } else {
            ("FullMatch,", "away")
        };
        assert!(playing.starts_with(screen), "{playing}");
        assert!(playing.contains(ground), "{playing}");
    }
    // A coin that only ever came down one way would be no coin.
    assert_eq!(seen, [true, true]);
}

#[test]
fn the_choice_of_ground_is_only_on_the_full_matchs_setup() {
    let Some(mut script) = game("menu") else {
        return;
    };
    script
        .run(&format!("wait 60; {FULL_MATCH}; wait 90"))
        .unwrap();
    assert!(!said(&script, "groundWord").is_empty());
    let main = state_after(&mut script, &format!("{BACK}; wait 90; state"));
    assert_eq!(main, "Menu, Main, Medium");
    assert!(said(&script, "groundWord").is_empty());
    // The last innings on its own has nothing to choose.
    let setup = state_after(&mut script, "click 200 181; wait 90; state");
    assert_eq!(setup, "Menu, MatchSetup, Medium");
    assert!(said(&script, "groundWord").is_empty());
}
