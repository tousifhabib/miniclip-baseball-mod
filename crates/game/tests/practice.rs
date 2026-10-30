//! Batting practice: pitches that never run out, each to a batter just come
//! up, and a report at the end.

#[path = "common/mod.rs"]
mod common;

use std::fs;

use bb_game::baseball::Screen;
use bb_game::mods::Mod;
use common::{
    LEAVE, MISS, ON, game, game_keeping, game_modded, next, number, pitch, ready, state,
    state_after,
};

/// Where the buttons are, in stage pixels: the one that quits a game, the
/// one that says yes to that, the arrow that turns the report's pages and
/// the button that leaves it.
const QUIT: &str = "click 40 386";
const YES: &str = "click 345 232";
const PAGE_ON: &str = "click 357 322";
const LEAVE_THE_REPORT: &str = "click 542 357";

const VIEWS: [&str; 2] = [Screen::PRACTICE_ARCADE, Screen::PRACTICE_MATCH];

/// How a figure of the session stands, as the game says it.
fn figure(now: &str, name: &str) -> u32 {
    number(now, &format!(", {name} ")).unwrap_or_else(|| panic!("no {name} in: {now}")) as u32
}

/// Plays `pitches` pitches, each swung at as `late` says, and leaves the
/// next on its way. Returns how the session stands after the last.
fn play(script: &mut bb_game::script::Script, pitches: u32, late: i32) -> String {
    let mut after = String::new();
    for _ in 0..pitches {
        ready(script);
        after = pitch(script, late, ON);
        next(script);
    }
    after
}

/// On the first page and on the setup page: the row that leads to practice,
/// the boxes of the view and of the timing bar, and the way on.
const BATTING_PRACTICE: &str = "click 235 286";
const MATCH_VIEW: &str = "click 510 289";
const BAR_OFF: &str = "click 510 316";
const NEXT: &str = "click 490 362";

#[test]
fn the_menu_leads_to_practice_on_the_view_chosen_for_it() {
    let Some(mut script) = game("menu") else {
        return;
    };
    let setup = state_after(
        &mut script,
        &format!("wait 60; {BATTING_PRACTICE}; wait 90; state"),
    );
    assert_eq!(setup, "Menu, PracticeSetup, Medium: Arcade view, bar on");
    let chosen = state_after(
        &mut script,
        &format!("{MATCH_VIEW}; wait 3; {BAR_OFF}; wait 3; state"),
    );
    assert_eq!(chosen, "Menu, PracticeSetup, Medium: Match view, bar off");
    let begun = state_after(&mut script, &format!("{NEXT}; wait 90; state"));
    assert!(begun.starts_with("PracticeMatch, Medium: "), "{begun}");
    // The bar says when to swing, and with it off nothing does.
    let pitch = ready(&mut script);
    assert!(!pitch.contains("best swung on steps"), "{pitch}");
    assert!(pitch.contains("practice, pitches 0"), "{pitch}");
}

#[test]
fn the_menu_leads_to_practice_on_the_arcade_view_with_the_bar_up_unless_told_otherwise() {
    let Some(mut script) = game("menu") else {
        return;
    };
    let steps = format!("wait 60; {BATTING_PRACTICE}; wait 90; {NEXT}; wait 90; state");
    let begun = state_after(&mut script, &steps);
    assert!(begun.starts_with("PracticeArcade, Medium: "), "{begun}");
    assert!(ready(&mut script).contains("best swung on steps"));
}

#[test]
fn a_session_goes_on_past_what_would_end_either_game() {
    for view in VIEWS {
        let Some(mut script) = game(view) else {
            return;
        };
        // More than the arcade's ten pitches, and more than would strike
        // out the side in a match.
        let after = play(&mut script, 14, MISS);
        assert!(after.contains(": Ready"), "{view}: {after}");
        assert_eq!(figure(&after, "pitches"), 14, "{after}");
        assert_eq!(figure(&after, "swings"), 14, "{after}");
        assert_eq!(figure(&after, "met"), 0, "{after}");
    }
}

#[test]
fn every_pitch_of_a_session_is_counted_by_what_came_of_it() {
    for view in VIEWS {
        let Some(mut script) = game(view) else {
            return;
        };
        let swung = play(&mut script, 1, 0);
        assert_eq!(figure(&swung, "met"), 1, "{view}: {swung}");
        assert_eq!(figure(&swung, "sweet"), 1, "{view}: {swung}");
        assert!(figure(&swung, "longest") > 0, "{view}: {swung}");
        let let_go = play(&mut script, 1, LEAVE);
        assert_eq!(figure(&let_go, "pitches"), 2, "{view}: {let_go}");
        assert_eq!(figure(&let_go, "swings"), 1, "{view}: {let_go}");
    }
}

#[test]
fn nothing_said_of_the_session_is_lost_when_the_next_pitch_is_called_at_once() {
    // In the arcade game's view the next pitch is on offer while the ball
    // is still up, and the ball counts all the same.
    let Some(mut script) = game(Screen::PRACTICE_ARCADE) else {
        return;
    };
    ready(&mut script);
    let hit = pitch(&mut script, 0, ON);
    assert_eq!(figure(&hit, "pitches"), 1, "{hit}");
    next(&mut script);
    let now = ready(&mut script);
    assert_eq!(figure(&now, "pitches"), 1, "{now}");
    assert!(figure(&now, "longest") > 0, "{now}");
}

#[test]
fn the_timing_bar_is_up_in_practice_whether_or_not_its_mod_is_on() {
    let Some(mut script) = game(Screen::PRACTICE_ARCADE) else {
        return;
    };
    assert!(ready(&mut script).contains("best swung on steps"));
}

#[test]
fn quitting_with_nothing_seen_goes_back_to_the_menu() {
    for view in VIEWS {
        let Some(mut script) = game(view) else {
            return;
        };
        let steps = format!("wait 100; {QUIT}; wait 40; {YES}; wait 90; state");
        let now = state_after(&mut script, &steps);
        assert!(now.starts_with("Menu, Main"), "{view}: {now}");
    }
}

#[test]
fn quitting_after_a_pitch_shows_the_report_whose_pages_turn_and_lead_to_the_menu() {
    for view in VIEWS {
        let Some(mut script) = game(view) else {
            return;
        };
        play(&mut script, 2, 0);
        let steps = format!("wait 30; {QUIT}; wait 40; {YES}; wait 90; state");
        let up = state_after(&mut script, &steps);
        assert_eq!(
            up, "PracticeReport, Medium: 2 pitches, page 1 of 2",
            "{view}"
        );
        let turned = state_after(&mut script, &format!("{PAGE_ON}; wait 5; state"));
        assert_eq!(turned, "PracticeReport, Medium: 2 pitches, page 2 of 2");
        let back = state_after(&mut script, &format!("{PAGE_ON}; wait 5; state"));
        assert_eq!(back, up);
        let menu = state_after(&mut script, &format!("{LEAVE_THE_REPORT}; wait 90; state"));
        assert!(menu.starts_with("Menu, Main"), "{view}: {menu}");
    }
}

#[test]
fn practice_goes_down_in_no_table_of_scores_or_records() {
    let dir = std::env::temp_dir().join(format!("bb-practice-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let scores = dir.join("scores.toml");
    let Some(mut script) = game_keeping(Screen::PRACTICE_ARCADE, 1, &[Mod::ZingerHit], &scores)
    else {
        return;
    };
    let after = play(&mut script, 3, 0);
    assert!(figure(&after, "longest") > 0, "{after}");
    assert!(!scores.exists(), "something was kept: {after}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_mods_that_go_by_a_score_or_a_count_have_no_part_in_practice() {
    let mods = [Mod::SuddenDeath, Mod::HeatCheck, Mod::TiredArm];
    let Some(mut script) = game_modded(Screen::PRACTICE_MATCH, 1, &mods) else {
        return;
    };
    // One strike would put a batter out in sudden death, and the state would
    // tell of a tiring arm or of heat.
    let after = play(&mut script, 4, MISS);
    assert_eq!(figure(&after, "pitches"), 4, "{after}");
    assert!(
        !after.contains("arm ") && !after.contains("heat "),
        "{after}"
    );
    let _ = state(&mut script);
}
