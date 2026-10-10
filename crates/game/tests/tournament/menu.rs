//! The menu's pages for a tournament: the summary of where it has got to,
//! and the setup page it is drawn from.

use bb_game::tournament::Format;

use super::common::{said, state, state_after, written};
use super::{CARRY_ON, tables};

/// Where things are on those pages, in stage pixels.
const BACK: &str = "click 290 362";
const NEXT: &str = "click 490 362";
const PLAY_BALL: &str = "click 480 362";
const SEE_THE_TABLES: &str = "click 304 292";
const GIVE_IT_UP: &str = "click 450 292";
const LEAGUE: &str = "click 434 290";
const CUP: &str = "click 513 290";
const FIVE_INNINGS: &str = "click 489 317";

/// The steps from the tables to the menu's page of what is next.
fn on() -> String {
    format!("{CARRY_ON}; wait 70; state")
}

#[test]
fn on_from_the_tables_the_menu_says_what_is_next() {
    let Some(mut script) = tables(Format::League, 9) else {
        return;
    };
    let now = state_after(&mut script, &on());
    let begins =
        "Menu, TournamentSummary, Medium: league of 6, 3 innings, played 9 of 15, ROUND 4, ";
    assert!(now.starts_with(begins), "{now}");
    assert_eq!(said(&script, "tournamentHeading"), ["TOURNAMENT"]);
    let lines = written(&script, "tournamentLine");
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0], "ROUND 4 OF THE LEAGUE!");
    assert!(lines[1].starts_with("You play ") && lines[2].ends_with(", over 3 innings."));
    assert!(lines[3].starts_with("You have won ") && lines[4].ends_with(" of 6."));
    assert_eq!(
        written(&script, "boxWords"),
        ["See the tables", "Give it up"]
    );
    // One of its boxes is the way back to the tables.
    let back = state_after(&mut script, &format!("{SEE_THE_TABLES}; wait 130; state"));
    assert!(
        back.starts_with("Tournament, Medium: league of 6, "),
        "{back}"
    );
    assert!(back.ends_with("TABLE, page 1 of 1"), "{back}");
}

#[test]
fn a_tournament_with_a_result_in_it_is_gone_on_with_and_not_set_up_again() {
    let Some(mut script) = tables(Format::League, 9) else {
        return;
    };
    script.run(&on()).unwrap();
    let back = state_after(&mut script, &format!("{BACK}; wait 70; state"));
    assert_eq!(back, "Menu, Main, Medium");
}

#[test]
fn one_with_no_result_yet_can_be_set_up_afresh_in_another_shape() {
    let Some(mut script) = tables(Format::League, 0) else {
        return;
    };
    let now = state_after(&mut script, &on());
    assert!(
        now.contains("league of 6, 3 innings, played 0 of 15"),
        "{now}"
    );
    assert_eq!(
        written(&script, "tournamentLine")[3],
        "It is your first match."
    );
    // Back is the setup page, with what was chosen for this one.
    let setup = state_after(&mut script, &format!("{BACK}; wait 70; state"));
    assert_eq!(setup, "Menu, TournamentSetup, Medium: league, 3 innings");
    assert_eq!(written(&script, "shapeWord"), ["Groups", "League", "Cup"]);
    assert_eq!(said(&script, "inningsHeading"), ["Innings:"]);
    assert_eq!(written(&script, "inningsWord"), ["3", "5", "9"]);
    // Another shape and another length, and on: a new one is drawn.
    let chosen = state_after(
        &mut script,
        &format!("{CUP}; {FIVE_INNINGS}; wait 2; state"),
    );
    assert_eq!(chosen, "Menu, TournamentSetup, Medium: cup, 5 innings");
    let drawn = state_after(&mut script, &format!("{NEXT}; wait 70; state"));
    let begins = "Menu, TournamentSummary, Medium: cup of 8, 5 innings, played 0 of 7, ";
    assert!(drawn.starts_with(begins), "{drawn}");
    let lines = written(&script, "tournamentLine");
    assert_eq!(lines[0], "THE QUARTER-FINALS OF THE CUP!");
    assert!(lines[2].ends_with(", over 5 innings."), "{}", lines[2]);
    // Nothing of the setup page is left on this one.
    assert!(written(&script, "shapeWord").is_empty());
    // And back again, to choose a league after all.
    script
        .run(&format!("{BACK}; wait 70; {LEAGUE}; wait 2"))
        .unwrap();
    assert_eq!(
        state(&mut script),
        "Menu, TournamentSetup, Medium: league, 5 innings"
    );
    assert!(written(&script, "tournamentLine").is_empty());
    let main = state_after(&mut script, &format!("{BACK}; wait 70; state"));
    assert_eq!(main, "Menu, Main, Medium");
}

#[test]
fn giving_a_tournament_up_takes_two_clicks() {
    let Some(mut script) = tables(Format::Groups, 4) else {
        return;
    };
    script.run(&on()).unwrap();
    let once = state_after(&mut script, &format!("{GIVE_IT_UP}; wait 5; state"));
    assert!(once.starts_with("Menu, TournamentSummary, "), "{once}");
    assert_eq!(written(&script, "boxWords"), ["See the tables", "Really?"]);
    let twice = state_after(&mut script, &format!("{GIVE_IT_UP}; wait 70; state"));
    assert_eq!(twice, "Menu, Main, Medium");
    assert!(written(&script, "tournamentLine").is_empty());
}

#[test]
fn when_it_is_over_the_menu_says_who_won_and_offers_another() {
    let Some(mut script) = tables(Format::Cup, 7) else {
        return;
    };
    let now = state_after(&mut script, &on());
    assert!(now.contains("played 7 of 7, over, won by "), "{now}");
    let lines = written(&script, "tournamentLine");
    assert_eq!(lines[0], "THE CUP IS OVER!");
    assert!(lines[1].ends_with(" won it.") || lines[1] == "You have won it!");
    // There is nothing left to give up.
    assert_eq!(written(&script, "boxWords"), ["See the tables"]);
    let another = state_after(&mut script, &format!("{PLAY_BALL}; wait 70; state"));
    assert_eq!(another, "Menu, TournamentSetup, Medium: cup, 3 innings");
}
